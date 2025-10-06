# Claude Code Anti-Patterns Detection

This document describes the new architectural validation patterns designed to catch common anti-patterns that AI code generators (like Claude Code) tend to create when writing Rust code.

## Purpose

While AI-generated code is often functionally correct, it sometimes lacks the idiomatic patterns that make Rust code clean, efficient, and maintainable. These validations help ensure that code quality remains high even when AI assists with development.

## Detected Anti-Patterns

### 1. String Allocation Anti-Patterns
**Threshold:** 20 occurrences

Catches unnecessary owned `String` where `&str` would suffice:
- Functions taking `String` parameters instead of `&str`
- Round-trip allocations like `.to_string().as_str()`
- Taking references of owned strings: `&something.to_string()`

**Why it matters:** Unnecessary allocations hurt performance and increase memory pressure.

**Example - Bad:**
```rust
fn process_name(name: String) -> String {
    // Forces caller to allocate even if they have &str
    name.to_uppercase()
}
```

**Example - Good:**
```rust
fn process_name(name: &str) -> String {
    // Caller can pass &str without allocation
    name.to_uppercase()
}
```

### 2. Iterator Anti-Patterns
**Threshold:** 15 occurrences

Detects manual loops where iterator chains would be cleaner:
- Mutable vec + manual loop (use `.collect()`)
- Manual counting (use `.count()`)
- Manual find loops (use `.find()`)

**Why it matters:** Iterator chains are more idiomatic, safer, and often more performant.

**Example - Bad:**
```rust
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item);
    }
}
```

**Example - Good:**
```rust
let results: Vec<_> = items
    .into_iter()
    .filter(|item| item.is_valid())
    .collect();
```

### 3. Error Context Anti-Patterns
**Threshold:** 10 occurrences

Flags errors without proper context:
- Plain error messages without cause chain
- Generic error wrapping without context
- Errors created without preserving underlying cause

**Why it matters:** Good error messages make debugging exponentially easier.

**Example - Bad:**
```rust
.map_err(|e| anyhow!("{}", e))?  // Loses error context
```

**Example - Good:**
```rust
.map_err(|e| anyhow!("Failed to parse config").context(e))?
```

### 4. Async Anti-Patterns
**Threshold:** 5 occurrences

Detects blocking operations in async contexts:
- Using `std::fs` instead of `tokio::fs` in async functions
- Using `std::thread::sleep` instead of `tokio::time::sleep`
- Blocking mutex locks in async code

**Why it matters:** Blocking in async contexts defeats the purpose of async and can cause deadlocks.

**Example - Bad:**
```rust
async fn read_config() -> Result<String> {
    std::fs::read_to_string("config.toml")?  // Blocks executor thread!
}
```

**Example - Good:**
```rust
async fn read_config() -> Result<String> {
    tokio::fs::read_to_string("config.toml").await?  // Proper async I/O
}
```

### 5. Lifetime Complexity Anti-Patterns
**Threshold:** 3 occurrences

Flags overly complex lifetime specifications:
- Triple `'static` lifetimes (usually wrong)
- Four or more lifetime parameters (too complex)

**Why it matters:** Complex lifetimes indicate design issues and make code hard to maintain.

**Example - Bad:**
```rust
fn complex<'a, 'b, 'c, 'd>(
    x: &'a str,
    y: &'b str,
    z: &'c str,
    w: &'d str
) -> &'a str {
    // Too many lifetimes! Redesign needed
}
```

**Example - Good:**
```rust
// Often you can simplify with a single lifetime or restructure
fn simple<'a>(items: &'a [&str]) -> &'a str {
    items[0]
}
```

## Validation Output

These patterns appear in the Unified Architectural Validation table with `⚠️ INFO` or `⚠️ WARN` status:

```
├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤
│ String allocations (String vs &str) │    21 │ ⚠️ INFO  │ src/routes/auth.rs:45                   │
│ Iterator anti-patterns              │     0 │ ✅ PASS  │ Idiomatic iterator usage                │
│ Error context missing               │     0 │ ✅ PASS  │ Good error context                      │
│ Async anti-patterns (blocking)      │     0 │ ✅ PASS  │ Proper async patterns                   │
│ Lifetime complexity                 │     1 │ ✅ PASS  │ Reasonable lifetime usage               │
└─────────────────────────────────────┴───────┴──────────┴─────────────────────────────────────────┘
```

## Configuration

Patterns are defined in `scripts/validation-patterns.toml`:
```toml
[validation_thresholds]
max_string_allocation_antipatterns = 20
max_iterator_antipatterns = 15
max_error_context_antipatterns = 10
max_async_antipatterns = 5
max_lifetime_antipatterns = 3
```

## Philosophy

These validations are **informational** rather than **critical failures**. They encourage idiomatic Rust without blocking development. Over time, as patterns improve, thresholds can be tightened.

The goal: Help Claude Code (and human developers!) write more idiomatic Rust code from the start.
