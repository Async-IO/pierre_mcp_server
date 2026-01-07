<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 30: Performance Characteristics & Benchmarks

This appendix documents Pierre's performance characteristics, optimization strategies, and benchmarking guidelines for production deployments.

## Performance Overview

Pierre is designed for low-latency fitness data processing with the following targets:

| Operation | Target Latency | Notes |
|-----------|---------------|-------|
| Health check | < 5ms | No DB, no auth |
| JWT validation | < 10ms | Cached JWKS |
| Simple tool call | < 50ms | Cached data |
| Provider API call | < 500ms | Network-bound |
| TSS calculation | < 20ms | CPU-bound |
| Complex analysis | < 200ms | Multi-algorithm |

## Algorithmic Complexity

### Training Load Calculations

| Algorithm | Time Complexity | Space Complexity |
|-----------|----------------|------------------|
| Average Power TSS | O(1) | O(1) |
| Normalized Power TSS | O(n) | O(w) where w=window |
| TRIMP | O(n) | O(1) |
| CTL/ATL/TSB | O(n) | O(1) per activity |
| VO2max estimation | O(1) | O(1) |

**Normalized Power calculation**:
```rust
// O(n) where n = power samples
// O(w) space for rolling window
pub fn calculate_np(power_stream: &[f64], window_seconds: u32) -> f64 {
    // 30-second rolling average of power^4
    let window_size = window_seconds as usize;
    let rolling_averages: Vec<f64> = power_stream
        .windows(window_size)           // O(n) iterations
        .map(|w| w.iter().sum::<f64>() / w.len() as f64)  // O(w) per window
        .collect();

    // Fourth root of mean of fourth powers
    let mean_fourth = rolling_averages.iter()
        .map(|p| p.powi(4))
        .sum::<f64>() / rolling_averages.len() as f64;

    mean_fourth.powf(0.25)
}
```

### Database Operations

| Operation | Complexity | Index Used |
|-----------|------------|-----------|
| Get user by ID | O(1) | PRIMARY KEY |
| Get user by email | O(log n) | idx_users_email |
| List activities (paginated) | O(k + log n) | Composite index |
| Get OAuth token | O(1) | UNIQUE constraint |
| Usage analytics (monthly) | O(log n) | idx_api_key_usage_timestamp |

## Memory Characteristics

### Static Memory

| Component | Approximate Size |
|-----------|-----------------|
| Binary size | ~45 MB |
| Startup memory | ~50 MB |
| Per connection | ~8 KB |
| SQLite pool (10 conn) | ~2 MB |
| JWKS cache | ~100 KB |
| LRU cache (default) | ~10 MB |

### Dynamic Memory

**Activity processing**:
```rust
// Memory per activity analysis
// - Activity struct: ~500 bytes
// - Power stream (1 hour @ 1Hz): 3600 * 8 = 29 KB
// - Heart rate stream: 3600 * 8 = 29 KB
// - GPS stream: 3600 * 24 = 86 KB
// - Analysis result: ~2 KB
// Total per activity: ~150 KB peak
```

**Concurrent request handling**:
```rust
// Per-request memory estimate
// - Request parsing: ~4 KB
// - Auth context: ~1 KB
// - Response buffer: ~8 KB
// - Tool execution: ~50 KB (varies by tool)
// Total per request: ~65 KB average
```

## Concurrency Model

### Tokio Runtime Configuration

```rust
// Production runtime (src/bin/pierre-mcp-server.rs)
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // Worker threads = CPU cores
    // I/O threads = 2 * CPU cores
}
```

### Connection Pooling

```rust
// SQLite pool configuration
SqlitePoolOptions::new()
    .max_connections(10)        // Max concurrent DB connections
    .min_connections(2)         // Keep-alive connections
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Some(Duration::from_secs(600)))
```

### Rate Limiting

| Tier | Requests/Month | Burst Limit | Window |
|------|---------------|-------------|--------|
| Trial | 1,000 | 10/min | 30 days |
| Starter | 10,000 | 60/min | 30 days |
| Professional | 100,000 | 300/min | 30 days |
| Enterprise | Unlimited | 1000/min | N/A |

## Optimization Strategies

### 1. Lazy Loading

```rust
// Providers loaded only when needed
impl ProviderRegistry {
    pub fn get(&self, name: &str) -> Option<Arc<dyn FitnessProvider>> {
        // Factory creates provider on first access
        self.factories.get(name)?.create_provider()
    }
}
```

### 2. Response Caching

```rust
// LRU cache for expensive computations
pub struct Cache {
    lru: Mutex<LruCache<String, CacheEntry>>,
    default_ttl: Duration,
}

// Cache key patterns
// - activities:{provider}:{user_id} -> Vec<Activity>
// - athlete:{provider}:{user_id} -> Athlete
// - stats:{provider}:{user_id} -> Stats
// - analysis:{activity_id} -> AnalysisResult
```

### 3. Query Optimization

```rust
// Efficient pagination with cursor-based approach
pub async fn list_activities_paginated(
    &self,
    user_id: Uuid,
    cursor: Option<&str>,
    limit: u32,
) -> Result<CursorPage<Activity>> {
    // Uses indexed seek instead of OFFSET
    sqlx::query_as!(
        Activity,
        r#"
        SELECT * FROM activities
        WHERE user_id = ?1 AND id > ?2
        ORDER BY id
        LIMIT ?3
        "#,
        user_id,
        cursor.unwrap_or(""),
        limit + 1  // Fetch one extra to detect has_more
    )
    .fetch_all(&self.pool)
    .await
}
```

### 4. Zero-Copy Serialization

```rust
// Use Cow<str> for borrowed strings
pub struct ActivityResponse<'a> {
    pub id: Cow<'a, str>,
    pub name: Cow<'a, str>,
    // Avoids cloning when data comes from cache
}
```

## Benchmarking Guidelines

### Running Benchmarks

```bash
# Install criterion
cargo install cargo-criterion

# Run all benchmarks
cargo criterion

# Run specific benchmark
cargo criterion --bench tss_calculation

# Generate HTML report
cargo criterion --bench tss_calculation -- --save-baseline main
```

### Example Benchmark

```rust
// benches/tss_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn tss_benchmark(c: &mut Criterion) {
    let activity = create_test_activity(3600); // 1 hour

    c.bench_function("tss_avg_power", |b| {
        b.iter(|| {
            TssAlgorithm::AvgPower.calculate(
                black_box(&activity),
                black_box(250.0),
                black_box(1.0),
            )
        })
    });

    c.bench_function("tss_normalized_power", |b| {
        b.iter(|| {
            TssAlgorithm::NormalizedPower { window_seconds: 30 }
                .calculate(
                    black_box(&activity),
                    black_box(250.0),
                    black_box(1.0),
                )
        })
    });
}

criterion_group!(benches, tss_benchmark);
criterion_main!(benches);
```

### Expected Results

| Benchmark | Expected Time | Acceptable Range |
|-----------|--------------|------------------|
| TSS (avg power) | 50 ns | < 100 ns |
| TSS (normalized) | 15 µs | < 50 µs |
| JWT validation | 100 µs | < 500 µs |
| Activity parse | 200 µs | < 1 ms |
| SQLite query | 500 µs | < 5 ms |

## Production Monitoring

### Key Metrics

```rust
// Prometheus metrics exposed at /metrics
counter!("pierre_requests_total", "method" => method, "status" => status);
histogram!("pierre_request_duration_seconds", "method" => method);
gauge!("pierre_active_connections");
gauge!("pierre_db_pool_connections");
counter!("pierre_provider_requests_total", "provider" => provider);
histogram!("pierre_provider_latency_seconds", "provider" => provider);
```

### Alert Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| Request latency p99 | > 500ms | > 2s |
| Error rate | > 1% | > 5% |
| DB pool saturation | > 70% | > 90% |
| Memory usage | > 70% | > 90% |
| Provider latency p99 | > 2s | > 10s |

## Profiling

### CPU Profiling

```bash
# Using perf
perf record -g cargo run --release
perf report

# Using flamegraph
cargo install flamegraph
cargo flamegraph --bin pierre-mcp-server
```

### Memory Profiling

```bash
# Using heaptrack
heaptrack cargo run --release
heaptrack_gui heaptrack.pierre-mcp-server.*.gz

# Using valgrind
valgrind --tool=massif ./target/release/pierre-mcp-server
ms_print massif.out.*
```

## Key Takeaways

1. **Target latencies**: Simple operations < 50ms, provider calls < 500ms.
2. **Algorithm efficiency**: NP-TSS is O(n), use AvgPower-TSS for quick estimates.
3. **Memory footprint**: ~50MB baseline, ~150KB per activity analysis.
4. **Connection pooling**: 10 SQLite connections handle typical workloads.
5. **Cursor pagination**: Avoids O(n) OFFSET performance degradation.
6. **LRU caching**: Reduces provider API calls and computation.
7. **Prometheus metrics**: Monitor latency, error rates, pool saturation.
8. **Benchmark before optimize**: Use criterion for reproducible measurements.

---

**Related Chapters**:
- Chapter 20: Sports Science Algorithms (algorithm complexity)
- Chapter 25: Deployment (production configuration)
- Appendix E: Rate Limiting (quota management)
