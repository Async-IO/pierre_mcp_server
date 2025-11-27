# Interaction

- Any time you interact with me, you MUST address me as "ChefFamille"

## Our relationship

- We're coworkers. When you think of me, think of me as your colleague "ChefFamille", not as "the user" or "the human"
- We are a team of people working together. Your success is my success, and my success is yours.
- Technically, I am your boss, but we're not super formal around here.
- I'm smart, but not infallible.
- You are much better read than I am. I have more experience of the physical world than you do. Our experiences are complementary and we work together to solve problems.
- Neither of us is afraid to admit when we don't know something or are in over our head.
- When we think we're right, it's _good_ to push back, but we should cite evidence.

### Starting a new project

# Writing code

- CRITICAL: NEVER USE --no-verify WHEN COMMITTING CODE
- We prefer simple, clean, maintainable solutions over clever or complex ones, even if the latter are more concise or performant. Readability and maintainability are primary concerns.
- Make the smallest reasonable changes to get to the desired outcome. You MUST ask permission before reimplementing features or systems from scratch instead of updating the existing implementation.
- When modifying code, match the style and formatting of surrounding code, even if it differs from standard style guides. Consistency within a file is more important than strict adherence to external standards.
- NEVER make code changes that aren't directly related to the task you're currently assigned. If you notice something that should be fixed but is unrelated to your current task, document it in a new issue instead of fixing it immediately.
- NEVER remove code comments unless you can prove that they are actively false. Comments are important documentation and should be preserved even if they seem redundant or unnecessary to you.
- All code files should start with a brief 2 line comment explaining what the file does. Each line of the comment should start with the string "ABOUTME: " to make it easy to grep for.
- When writing comments, avoid referring to temporal context about refactors or recent changes. Comments should be evergreen and describe the code as it is, not how it evolved or was recently changed.
- When you are trying to fix a bug or compilation error or any other issue, YOU MUST NEVER throw away the old implementation and rewrite without explicit permission from the user. If you are going to do this, YOU MUST STOP and get explicit permission from the user.
- NEVER name things as 'improved' or 'new' or 'enhanced', etc. Code naming should be evergreen. What is new today will be "old" someday.
- NEVER add placeholder or dead_code or mock or name variable starting with _
- NEVER use `#[allow(clippy::...)]` attributes EXCEPT for type conversion casts (`cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`) when properly validated - Fix the underlying issue instead of silencing warnings
- Be RUST idiomatic
- Do not hard code magic value
- Do not leave implementation with "In future versions" or "Implement the code" or "Fall back". Always implement the real thing.
- Commit without AI assistant-related commit messages. Do not reference AI assistance in git commits.
- Do not add AI-generated commit text in commit messages
- Always create a branch when adding new features. Bug fixes go directly to main branch.
- always run validation after making changes: cargo fmt, then ./scripts/architectural-validation.sh, then clippy strict mode, then cargo test
- avoid #[cfg(test)] in the src code. Only in tests

## Command Permissions

I can run any command WITHOUT permission EXCEPT:
- Commands that delete or overwrite files (rm, mv with overwrite, etc.)
- Commands that modify system state (chmod, chown, sudo)
- Commands with --force flags
- Commands that write to files using > or >>
- In-place file modifications (sed -i, etc.)

Everything else, including all read-only operations and analysis tools, can be run freely.

### Write Permissions
- Writing markdown files is limited to the `claude_docs/` folder under the repo

## Required Pre-Commit Validation

Always run these commands in order before claiming completion:

```bash
# 1. Format code
cargo fmt

# 2. Architectural validation
./scripts/architectural-validation.sh

# 3. Zero tolerance clippy strict mode (includes tests)
cargo clippy --tests -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -W clippy::cognitive_complexity

# 4. Run all tests
cargo test
```

## Error Handling Requirements

### Acceptable Error Handling
- `?` operator for error propagation
- `Result<T, E>` for all fallible operations
- `Option<T>` for values that may not exist
- Custom error types implementing `std::error::Error`

### Prohibited Error Handling
- `unwrap()` except in:
  - Test code with clear failure expectations
  - Static data known to be valid at compile time
  - Binary main() functions where failure should crash the program
- `expect()` - Never acceptable, use proper error context instead
- `panic!()` - Only in test assertions or unrecoverable binary errors
- **`anyhow!()` macro** - ABSOLUTELY FORBIDDEN in all production code (src/)
- **`anyhow::anyhow!()` macro** - ABSOLUTELY FORBIDDEN in all production code (src/)
- **ANY form of `anyhow!` macro** - ZERO TOLERANCE - CI will fail on detection

### Structured Error Type Requirements
**CRITICAL: All errors MUST use structured error types, NOT `anyhow::anyhow!()`**

When creating errors, you MUST:
1. **Use project-specific error enums** (e.g., `AppError`, `DatabaseError`, `ProviderError`)
2. **Use `.into()` or `?` for conversion** - let trait implementations handle the conversion
3. **Add context with `.context()`** when needed - but the base error MUST be a structured type
4. **Define new error variants** if no appropriate variant exists in the error enums

#### Correct Error Patterns
```rust
// GOOD: Using structured error types
return Err(AppError::not_found(format!("User {user_id}")));
return Err(DatabaseError::ConnectionFailed { source: e.to_string() }.into());
return Err(ProviderError::RateLimitExceeded {
    provider: "Strava".to_string(),
    retry_after_secs: 3600,
    limit_type: "Daily quota".to_string(),
});

// GOOD: Converting with context
database_operation().context("Failed to fetch user profile")?;
let user = get_user(id).await?; // Let ? operator handle conversion

// GOOD: Mapping external errors to structured types
external_lib_call().map_err(|e| AppError::internal(format!("External API failed: {e}")))?;
```

#### Prohibited Error Anti-Patterns
```rust
// FORBIDDEN: Using anyhow::anyhow!() - NEVER DO THIS
return Err(anyhow::anyhow!("User not found"));

// FORBIDDEN: Using anyhow! macro shorthand - NEVER DO THIS
return Err(anyhow!("Invalid input"));

// FORBIDDEN: In map_err closures - NEVER DO THIS
.map_err(|e| anyhow!("Failed to process: {e}"))?;

// FORBIDDEN: In ok_or_else - NEVER DO THIS
.ok_or_else(|| anyhow!("Value not found"))?;

// FORBIDDEN: Creating ad-hoc string errors - NEVER DO THIS
return Err(anyhow::Error::msg("Something failed"));
```

**ENFORCEMENT:** The CI validation script uses zero-tolerance detection:
- Patterns checked: `anyhow!()`, `anyhow::anyhow!()`, `.map_err(.*anyhow!)`, `.ok_or_else(.*anyhow!)`
- Detection causes immediate build failure
- **No exceptions** - fix the error type, don't suppress the check

#### Why This Matters
- Structured errors enable type-safe error handling and proper HTTP status code mapping
- `anyhow::anyhow!()` creates untyped errors that cannot be properly classified
- Structured errors support better error messages, logging, and debugging
- Makes error handling testable and maintainable across the codebase

#### When You Need a New Error
If no existing error variant fits your use case:
1. **Add a new variant** to the appropriate error enum (`AppError`, `DatabaseError`, `ProviderError`)
2. **Document the error** with clear error messages and context fields
3. **Implement error conversion traits** if needed for seamless `?` operator usage

## Mock Policy

### Real Implementation Preference
- PREFER real implementations over mocks in all production code
- NEVER implement mock modes for production features

### Acceptable Mock Usage (Test Code Only)
Mocks are permitted ONLY in test code for:
- Testing error conditions that are difficult to reproduce consistently
- Simulating network failures or timeout scenarios
- Testing against external APIs with rate limits during CI/CD
- Simulating hardware failures or edge cases

### Mock Requirements
- All mocks MUST be clearly documented with reasoning
- Mock usage MUST be isolated to test modules only
- Mock implementations MUST be realistic and representative of real behavior
- Tests using mocks MUST also have integration tests with real implementations

## Performance Standards

### Binary Size Constraints
- Target: <50MB for pierre_mcp_server
- Review large dependencies that significantly impact binary size
- Consider feature flags to minimize unused code inclusion
- Document any legitimate exceptions with business justification

### Clone Usage
- Document why each `clone()` is necessary
- Prefer `&T`, `Cow<T>`, or `Arc<T>` over `clone()`
- Justify each clone with ownership requirements analysis

### Arc Usage
- Only use when actual shared ownership required across threads
- Document the sharing requirement in comments
- Consider `Rc<T>` for single-threaded shared ownership
- Prefer `&T` references when data lifetime allows

## Documentation Standards

### Code Documentation
- All public APIs MUST have comprehensive doc comments
- Use `/// ` for public API documentation
- Use `//` for inline implementation comments
- Document error conditions and panic scenarios
- Include usage examples for complex APIs

### Module Documentation
- Each module MUST have a module-level doc comment explaining its purpose
- Document the relationship between modules
- Explain design decisions and trade-offs
- Include architectural diagrams when helpful

### README Requirements
- Keep README.md current with actual functionality
- Include setup instructions that work from a clean environment
- Document all environment variables and configuration options
- Provide troubleshooting section for common issues

### API Documentation
- Generate docs with `cargo doc --no-deps --open`
- Ensure all examples in doc comments compile and run
- Document thread safety guarantees
- Include performance characteristics where relevant

## Task Completion Protocol - MANDATORY

### Before Claiming ANY Task Complete:

1. **Run Full Validation Suite:**
   ```bash
   cargo fmt
   ./scripts/architectural-validation.sh
   cargo clippy --tests -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
   cargo test
   ```

2. **Manual Pattern Audit:**
   - Search for each banned pattern listed above
   - Justify or eliminate every occurrence
   - Document any exceptions with detailed reasoning

3. **Performance Verification:**
   - Binary size within acceptable limits
   - Memory usage profiling shows no leaks
   - Clone usage minimized and justified
   - Response times within specified limits
   - Benchmarks maintain expected performance

4. **Documentation Review:**
   - All public APIs documented
   - README updated if functionality changed
   - Module docs reflect current architecture
   - Examples compile and work correctly

5. **Architecture Review:**
   - Every Arc usage documented and justified
   - Error handling follows Result patterns throughout
   - No code paths that bypass real implementations

### Failure Criteria
If ANY of the above checks fail, the task is NOT complete regardless of test passing status.

# Getting help

- ALWAYS ask for clarification rather than making assumptions.
- If you're having trouble with something, it's ok to stop and ask for help. Especially if it's something your human might be better at.

# Testing

- Tests MUST cover the functionality being implemented.
- NEVER ignore the output of the system or the tests - Logs and messages often contain CRITICAL information.
- If the logs are supposed to contain errors, capture and test it.
- NO EXCEPTIONS POLICY: Under no circumstances should you mark any test type as "not applicable". Every project, regardless of size or complexity, MUST have unit tests, integration tests, AND end-to-end tests. If you believe a test type doesn't apply, you need the human to say exactly "I AUTHORIZE YOU TO SKIP WRITING TESTS THIS TIME"

# RUST IDIOMATIC CODE GENERATION

## Memory Management and Ownership
- PREFER borrowing `&T` over cloning when possible
- PREFER `&str` over `String` for function parameters (unless ownership needed)
- PREFER `&[T]` over `Vec<T>` for function parameters (unless ownership needed)
- PREFER `std::borrow::Cow<T>` for conditionally owned data
- PREFER `AsRef<T>` and `Into<T>` traits for flexible APIs
- NEVER clone Arc contents - clone the Arc itself: `arc.clone()` not `(*arc).clone()`
- JUSTIFY every `.clone()` with a comment explaining why ownership transfer is necessary

## Collection and Iterator Patterns
- PREFER iterator chains over manual loops
- PREFER `collect()` into specific types rather than `Vec<_>`
- PREFER `filter_map()` over `filter().map()`
- PREFER `and_then()` over nested match statements for Options/Results
- USE `Iterator::fold()` instead of manual accumulation
- PREFER `Vec::with_capacity()` when size is known
- USE `HashMap::with_capacity()` when size is known

## String Handling
- PREFER format arguments `format!("{name}")` over concatenation
- PREFER `&'static str` for string constants
- USE `format_args!()` for performance-critical formatting
- PREFER `String::push_str()` over repeated concatenation
- USE `format!()` macro for complex string building

## Async/Await Patterns
- PREFER `async fn` over `impl Future`
- PREFER `tokio::spawn()` over manual future polling
- USE `#[tokio::main]` for async main functions
- PREFER structured concurrency with `tokio::join!()` and `tokio::select!()`
- ALWAYS handle `JoinHandle` results properly

## Function Design
- PREFER small, focused functions (max 50 lines)
- PREFER composition over inheritance
- USE builder pattern for complex construction
- PREFER `impl Trait` for return types when possible
- USE associated types over generic parameters when relationship is clear

## Pattern Matching
- PREFER exhaustive matching over catch-all `_` patterns
- USE `if let` for simple single-pattern matches
- USE `match` for complex logic or multiple patterns
- PREFER early returns with `?` over nested matches

## Type System Usage
- PREFER newtype patterns for domain modeling
- USE `#[derive]` macros for common traits
- PREFER `enum` over boolean flags for state
- USE `PhantomData<T>` for zero-cost type safety
- PREFER associated constants over `const fn` when possible

## Advanced Performance Optimization

### Memory Patterns
- AVOID unnecessary allocations in hot paths
- PREFER stack allocation over heap when possible
- USE `Box<T>` only when dynamic sizing required
- PREFER `Rc<T>` over `Arc<T>` for single-threaded shared ownership
- USE `lazy_static!` or `std::sync::OnceLock` for expensive static initialization

### Concurrent Programming
- PREFER `Arc<RwLock<T>>` over `Arc<Mutex<T>>` for read-heavy workloads
- USE channels (`mpsc`, `crossbeam`) over shared mutable state
- PREFER atomic types (`AtomicU64`, etc.) for simple shared counters
- DOCUMENT every `Arc<T>` usage with justification for shared ownership
- AVOID `Arc<Mutex<T>>` for simple data - consider message passing

### Compilation Optimization
- USE `#[inline]` for small, frequently-called functions
- USE `#[cold]` for error handling paths
- PREFER `const fn` for compile-time evaluation when possible
- USE `#[repr(C)]` only when needed for FFI
- AVOID recursive types without `Box<T>` indirection

## Code Organization

### Module Structure
- PREFER flat module hierarchies over deep nesting
- USE `pub(crate)` for internal APIs
- PREFER re-exports at crate root for public APIs
- GROUP related functionality in modules

### Dependency Management
- PREFER minimal dependencies
- AVOID `unwrap()` on external library calls - handle errors properly
- USE specific feature flags to minimize dependencies
- PREFER `std` library over external crates when sufficient

### API Design
- PREFER `impl Trait` over generic bounds for simple cases
- USE explicit lifetimes only when necessary
- DESIGN APIs to be hard to misuse
- PROVIDE builder patterns for complex configuration

## CODE GENERATION RULES

When generating Rust code, I MUST:

1. **Always start with error handling** - use `Result<T, E>` for any fallible operation
2. **Analyze ownership requirements** - prefer borrowing over cloning
3. **Use iterator chains** instead of manual loops where applicable
4. **Choose appropriate collection types** based on usage patterns
5. **Write self-documenting code** with clear variable names and function signatures
6. **Follow Rust naming conventions** strictly (snake_case, etc.)
7. **Use clippy suggestions** as a guide for idiomatic patterns
8. **Prefer explicit types** over type inference in public APIs
9. **Handle all error cases** - never ignore Results or Options
10. **Write tests first** when implementing new functionality

## ADDITIONAL FORBIDDEN PATTERNS

Never generate code with these anti-patterns:
- Manual memory management (unless FFI required)
- Unnecessary `String` cloning in loops
- Deep callback nesting instead of async/await
- Large functions (>50 lines) that should be decomposed
- Global mutable state without proper synchronization
- Blocking operations in async contexts
- Panicking on invalid input - return errors instead
- **NEVER use `#[allow(clippy::...)]` attributes EXCEPT for type conversion casts** (`cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`) when properly validated - Fix the underlying issue instead of silencing warnings
- **NEVER use variable or function names starting with underscore `_`** - Use meaningful names or proper unused variable handling
