<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 25: Production Deployment, Clippy & Performance

This chapter covers production deployment strategies, Clippy lint configuration for code quality, performance optimization techniques, and monitoring best practices for Pierre.

## What You'll Learn

- Production deployment architecture
- Clippy lint configuration
- Performance optimization patterns
- Database connection pooling
- Monitoring and observability
- Security hardening
- Scaling strategies

## Clippy Configuration

Pierre uses strict Clippy lints to maintain code quality.

**Source**: Cargo.toml:158-200 (lints section)
```toml
[lints.clippy]
# Pedantic lints for high-quality code
all = "warn"
pedantic = "warn"
nursery = "warn"

# Specific denials for critical issues
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unimplemented = "deny"
todo = "deny"
unreachable = "deny"

# Allow specific patterns used throughout codebase
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
```

**Lint categories**:
- **all**: Enable all Clippy lints
- **pedantic**: Extra pedantic lints for code quality
- **nursery**: Experimental lints being tested
- **Denials**: `unwrap_used`, `panic`, `todo` cause build failures

**Why deny unwrap**: Prevents runtime panics in production. Use `?` operator or `unwrap_or` instead.

## Production Deployment

Pierre deployment architecture:

```
┌──────────────┐
│   Nginx      │ (Reverse proxy, TLS termination)
└──────┬───────┘
       │
┌──────▼───────┐
│   Pierre     │ (Rust binary, multiple instances)
│   Server     │
└──────┬───────┘
       │
┌──────▼───────┐
│  PostgreSQL  │ (Primary database)
└──────────────┘
```

**Deployment checklist**:
1. **Environment variables**: Set `DATABASE_URL`, `JWT_SECRET`, `OAUTH_*` vars
2. **TLS certificates**: Configure HTTPS with Let's Encrypt
3. **Database migrations**: Run `sqlx migrate run`
4. **Connection pooling**: Set `DATABASE_MAX_CONNECTIONS=20`
5. **Logging**: Configure `RUST_LOG=info`
6. **Monitoring**: Enable Prometheus metrics endpoint

## Performance Optimization

**Database connection pooling**:
```rust
let pool = PgPoolOptions::new()
    .max_connections(20)
    .acquire_timeout(Duration::from_secs(3))
    .connect(&database_url)
    .await?;
```

**Query optimization**:
- **Indexes**: Create indexes on `user_id`, `provider`, `activity_date`
- **Prepared statements**: Use SQLx compile-time verification
- **Batch operations**: Insert multiple activities in single transaction
- **Connection reuse**: Pool connections, avoid per-request connections

**Async runtime optimization**:
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

**Tokio configuration**:
- **Worker threads**: Default = CPU cores
- **Blocking threads**: Separate pool for blocking operations
- **Stack size**: Increase if deep recursion needed

## Monitoring

**Metrics to track**:
1. **Request latency**: P50, P95, P99 response times
2. **Error rate**: 4xx and 5xx responses per endpoint
3. **Database connections**: Active, idle, waiting
4. **Memory usage**: RSS, heap allocation
5. **OAuth success rate**: Connection success vs failures

**Logging best practices**:
```rust
tracing::info!(
    user_id = %user_id,
    provider = %provider,
    activities_count = activities.len(),
    "Successfully fetched activities"
);
```

## Security Hardening

**Production security**:
1. **TLS only**: Redirect HTTP to HTTPS
2. **CORS restrictions**: Whitelist allowed origins
3. **Rate limiting**: IP-based limits for public endpoints
4. **Input validation**: Validate all user inputs
5. **SQL injection prevention**: Use parameterized queries (SQLx)
6. **Secret management**: Use environment variables or vault
7. **Audit logging**: Log all authentication attempts

**Environment configuration**:
```bash
# Production environment variables
export DATABASE_URL="postgresql://user:pass@localhost/pierre"
export JWT_SECRET="$(openssl rand -base64 32)"
export RUST_LOG="info"
export HTTP_PORT="8081"
export CORS_ALLOWED_ORIGINS="https://app.pierre.ai"
```

## Scaling Strategies

**Horizontal scaling**:
- **Load balancer**: Nginx/HAProxy distributes requests
- **Multiple instances**: Run 2-4 Pierre servers behind load balancer
- **Session affinity**: Not required (stateless JWT authentication)

**Database scaling**:
- **Read replicas**: Offload read-heavy queries
- **Connection pooling**: Limit connections per instance
- **Caching**: Redis for frequently accessed data

**Performance targets**:
- **API latency**: P95 < 200ms
- **Database queries**: P95 < 50ms
- **OAuth flow**: Complete in < 5 seconds
- **Throughput**: 1000 req/sec per instance

## Key Takeaways

1. **Clippy lints**: Strict lints (`deny` unwrap, panic, todo) prevent common errors.
2. **Connection pooling**: Reuse database connections for performance.
3. **Deployment architecture**: Nginx → Pierre (multiple instances) → PostgreSQL.
4. **Monitoring**: Track latency, errors, connections, memory.
5. **Security hardening**: TLS, CORS, rate limiting, input validation.
6. **Horizontal scaling**: Load balancer + multiple stateless instances.
7. **Environment config**: Use env vars for secrets and configuration.

---

**End of Part VII: Testing & Deployment**

You've completed the testing and deployment section. You now understand:
- Testing framework with synthetic data (Chapter 23)
- Design system and templates (Chapter 24)
- Production deployment and performance (Chapter 25)

**Next**: [Appendix A: Rust Idioms Reference](./appendix-a-rust-idioms.md) - Quick reference for Rust idioms used throughout Pierre.
