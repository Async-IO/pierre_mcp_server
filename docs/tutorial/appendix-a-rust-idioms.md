# appendix A: rust idioms reference

Quick reference for Rust idioms used throughout Pierre.

## error handling

**`?` operator**: Propagate errors up the call stack.
```rust
let data = fetch_data()?; // Returns early if error
```

**`thiserror`**: Derive Error trait with formatted messages.
```rust
#[derive(Error, Debug)]
#[error("Database error: {0}")]
pub struct DbError(String);
```

## option and result patterns

**`Option::is_some_and`**: Check Some and condition in one call.
```rust
token.expires_at.is_some_and(|exp| exp > Utc::now())
```

**`Result::map_or`**: Transform result or use default.
```rust
result.map_or(0, |val| val.len())
```

## ownership and borrowing

**`Arc<T>`**: Shared ownership across threads.
```rust
let database = Arc::new(Database::new());
let db_clone = database.clone(); // Cheap reference count increment
```

**`Box<dyn Trait>`**: Heap-allocated trait objects.
```rust
let provider: Box<dyn FitnessProvider> = Box::new(StravaProvider::new());
```

## async patterns

**`async_trait`**: Async methods in traits.
```rust
#[async_trait]
trait Provider {
    async fn get_data(&self) -> Result<Data>;
}
```

**HRTB for Deserialize**: Higher-ranked trait bound.
```rust
where
    T: for<'de> Deserialize<'de>,
```

## type safety patterns

**Enum for algorithm selection**:
```rust
enum Algorithm {
    Method1 { param: u32 },
    Method2,
}
```

**`#[must_use]`**: Compiler warning if return value ignored.
```rust
#[must_use]
pub fn calculate(&self) -> f64 { ... }
```

## memory management

**`zeroize`**: Secure memory cleanup for secrets.
```rust
use zeroize::Zeroize;
secret.zeroize(); // Overwrite with zeros
```

**`OnceLock`**: Thread-safe lazy static initialization.
```rust
static CONFIG: OnceLock<Config> = OnceLock::new();
CONFIG.get_or_init(|| Config::load());
```

## key takeaways

1. **Error propagation**: Use `?` operator for clean error handling.
2. **Trait objects**: `Arc<dyn Trait>` for shared polymorphism.
3. **Async traits**: `#[async_trait]` macro enables async methods in traits.
4. **Type safety**: Enums and `#[must_use]` prevent common mistakes.
5. **Secure memory**: `zeroize` crate for cryptographic key cleanup.
