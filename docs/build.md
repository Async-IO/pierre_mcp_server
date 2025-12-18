# Build Configuration

Technical documentation for build system configuration, linting enforcement, and compilation settings.

## Rust Toolchain Management

**File**: `rust-toolchain`
**Current version**: `1.92.0`

### Version Pinning Strategy

The project pins the exact Rust version to ensure reproducible builds across development and CI/CD environments. This eliminates "works on my machine" issues and enforces consistent compiler behavior.

**Rationale for 1.92.0**:
- Stable rust 2021 edition support
- clippy lint groups fully stabilized
- sqlx compile-time query checking compatibility
- tokio 1.x runtime stability

### Updating Rust Version

Update process requires validation across:
1. clippy lint compatibility (all/pedantic/nursery groups)
2. sqlx macro compatibility (database query verification)
3. tokio runtime stability
4. dependency compatibility check via `cargo tree`

**Command**: Update `rust-toolchain` file and run full validation:
```bash
echo "1.XX.0" > rust-toolchain
./scripts/lint-and-test.sh
```

## Cargo.toml Linting Configuration

### Zero-Tolerance Enforcement Model

Lines 148-208 define compile-time error enforcement via `[lints.rust]` and `[lints.clippy]`.

**Design decision**: All clippy warnings are build errors via `level = "deny"`. This eliminates the "fix it later" anti-pattern and prevents technical debt accumulation.

### Clippy Lint Groups

```toml
[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
```

**Rationale**:
- `all`: Standard correctness lints (memory safety, logic errors)
- `pedantic`: Code quality lints (style, readability)
- `nursery`: Experimental lints (cutting-edge analysis)
- `priority = -1`: Apply base groups first, allow specific overrides

**Trade-off**: Nursery lints may change behavior between rust versions. Accepted for early detection of potential issues.

### Unsafe Code Policy

```toml
[lints.rust]
unsafe_code = "deny"
```

**Enforcement model**: deny-by-default with whitelist validation.

**Approved locations**:
- `src/health.rs`: Windows FFI for system health metrics (`GlobalMemoryStatusEx`, `GetDiskFreeSpaceExW`)

**Validation**: `scripts/architectural-validation.sh` fails build if unsafe code appears outside approved locations.

**Rationale**: Unsafe code eliminates rust's memory safety guarantees. Whitelist approach ensures:
1. All unsafe usage is justified and documented
2. Unsafe code is isolated to specific modules
3. Code review focuses on unsafe boundaries
4. FFI interactions are contained

### Error Handling Enforcement

```toml
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

**Acceptable contexts**:
- Test code with documented failure expectations
- Static data known valid at compile time (e.g., regex compilation in const context)
- Binary `main()` functions where failure should terminate process

**Production code requirements**:
- All fallible operations return `Result<T, E>`
- Error propagation via `?` operator
- Structured error types (AppError, DatabaseError, ProviderError)
- No string-based errors

**Rationale**: `unwrap()` causes panics on `None`/`Err`, crashing the server. Production services must handle errors gracefully and return structured error responses.

### Type Conversion Safety

```toml
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
```

**Rationale**: Type conversions are validated at call sites via context analysis. Blanket denial creates false positives for:
- `u64` → `usize` (safe on 64-bit systems)
- `f64` → `f32` (acceptable precision loss for display)
- `i64` → `u64` (validated non-negative before cast)

**Requirement**: Casts must be documented with safety justification when non-obvious.

### Function Size Policy

```toml
too_many_lines = "allow"
```

**Policy**: Functions over 100 lines trigger manual review but don't fail build.

**Validation**: Scripts detect functions >100 lines and verify documentation comment explaining complexity. Functions >100 lines require:
- `// Long function:` comment with rationale, OR
- Decomposition into helper functions

**Rationale**: Some functions have legitimate complexity (e.g., protocol parsers, error handling dispatchers). Blanket 50-line limit creates artificial decomposition that reduces readability.

### Additional Quality Lints

```toml
clone_on_copy = "warn"      # Cloning Copy types is inefficient
redundant_clone = "warn"     # Unnecessary allocations
await_holding_lock = "warn"  # Deadlock prevention
str_to_string = "deny"       # Prefer .to_owned() for clarity
```

## Build Profiles

### Dev Profile

```toml
[profile.dev]
debug = 1            # line number information for backtraces
opt-level = 0        # no optimization, fastest compilation
overflow-checks = true   # catch integer overflow in debug builds
```

**Use case**: Development iteration speed. Prioritizes compilation time over runtime performance.

### Release Profile

```toml
[profile.release]
lto = "thin"         # link-time optimization (intra-crate)
codegen-units = 1    # single codegen unit for better optimization
panic = "abort"      # reduce binary size, no unwinding
strip = true         # remove debug symbols
```

**Binary size impact**: ~40% size reduction vs unoptimized
**Compilation time**: +30% vs dev profile
**Runtime performance**: 2-5x faster than dev builds

**Rationale**:
- `lto = "thin"`: Balance between compilation time and optimization
- `codegen-units = 1`: Maximum intra-crate optimization
- `panic = "abort"`: Production services should crash on panic (no recovery)
- `strip = true`: Debug symbols not needed in production

### Release-LTO Profile

```toml
[profile.release-lto]
inherits = "release"
lto = "fat"          # cross-crate optimization
```

**Binary size impact**: Additional 10-15% size reduction
**Compilation time**: 2-3x slower than thin LTO
**Runtime performance**: Marginal improvement (5-10%) over thin LTO

**Use case**: Distribution builds where binary size critical. Not used in CI/CD due to compilation time.

## Feature Flags

```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
testing = []
telemetry = []
```

**Design decision**: Compile-time feature selection eliminates runtime configuration complexity.

**sqlite (default)**: Development and single-instance deployments
**postgresql**: Production multi-instance deployments with shared state
**testing**: Test utilities and mock implementations
**telemetry**: OpenTelemetry instrumentation (production observability)

**Binary size impact**:
- sqlite-only: ~45MB
- sqlite+postgresql: ~48MB
- All features: ~50MB

## Dependency Strategy

### Principle: Minimal Dependencies

Each dependency increases:
- Binary size (transitive dependencies)
- Compilation time
- Supply chain attack surface
- Maintenance burden (version conflicts)

**Review process**: New dependencies require justification:
1. What stdlib/existing dependency could solve this?
2. What's the binary size impact? (`cargo bloat`)
3. Is the crate maintained? (recent commits, issue response)
4. What's the transitive dependency count? (`cargo tree`)

### Pinned Dependencies

```toml
base64ct = "=1.6.0"
```

**Rationale**: base64ct 1.7.0+ requires rust edition 2024, incompatible with dependencies still on edition 2021. Pin eliminates upgrade-time breakage.

### Feature-Gated Dependencies

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"], default-features = false }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres", ...], default-features = false }
```

**Rationale**: `default-features = false` eliminates unused functionality:
- reqwest: Exclude native-tls (prefer rustls for pure-rust stack)
- sqlx: Exclude mysql/mssql drivers

**Binary size savings**: ~5MB from feature pruning

## Validation Commands

### Pre-Commit Checks

```bash
# Linting (zero warnings)
cargo clippy --all-targets --all-features

# Type checking
cargo check --all-features

# Tests
cargo test --release

# Binary size
cargo build --release && ls -lh target/release/pierre-mcp-server

# Security audit
cargo deny check

# Full validation
./scripts/lint-and-test.sh
```

### CI/CD Validation

The project uses five GitHub Actions workflows for comprehensive validation:

1. **Rust** (`.github/workflows/rust.yml`): Core quality gate
   - clippy zero-warning check
   - Test suite execution with coverage
   - Security audit (cargo-deny)
   - Architecture validation (unsafe code, algorithm patterns)

2. **Backend CI** (`.github/workflows/ci.yml`): Multi-database validation
   - SQLite + PostgreSQL test execution
   - Frontend tests (Node.js/TypeScript)
   - Secret pattern validation
   - Separate coverage for each database

3. **Cross-Platform** (`.github/workflows/cross-platform.yml`): OS compatibility
   - Linux (PostgreSQL), macOS (SQLite), Windows (SQLite)
   - Platform-specific optimizations

4. **SDK Tests** (`.github/workflows/sdk-tests.yml`): TypeScript SDK bridge
   - Unit, integration, and E2E tests
   - SDK ↔ Rust server communication validation

5. **MCP Compliance** (`.github/workflows/mcp-compliance.yml`): Protocol specification
   - MCP protocol conformance testing
   - TypeScript type validation

**See [ci/cd.md](ci-cd.md) for comprehensive workflow documentation, troubleshooting guides, and local validation commands.**

## Cargo-Deny Configuration

**File**: `deny.toml`

### Security Advisory Scanning

```toml
[advisories]
ignore = [
    "RUSTSEC-2023-0071",  # Legacy ignore
    "RUSTSEC-2024-0384",  # instant crate unmaintained (no safe upgrade path)
    "RUSTSEC-2024-0387",  # opentelemetry_api merged (used by opentelemetry-stdout)
]
```

**Rationale**: Ignored advisories have no safe upgrade path or are false positives for our usage. Requires periodic review.

### License Compliance

```toml
[licenses]
allow = [
    "MIT", "Apache-2.0",        # Standard permissive licenses
    "BSD-3-Clause",             # Crypto libraries
    "ISC",                      # ring, untrusted
    "Unicode-3.0",              # ICU unicode data
    "CDLA-Permissive-2.0",      # TLS root certificates
    "MPL-2.0", "Zlib",          # Additional OSI-approved
]
```

**Policy**: Only OSI-approved permissive licenses allowed. Copyleft licenses (GPL, AGPL) prohibited due to distribution restrictions.

### Supply Chain Protection

```toml
[sources]
unknown-git = "deny"
unknown-registry = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

**Rationale**: Only crates.io dependencies allowed. Prevents supply chain attacks via malicious git repositories or alternate registries.

## Compilation Optimization Notes

### LTO Trade-offs

**thin lto**: Optimizes within each crate, respects crate boundaries
- Compilation time: Moderate
- Optimization level: Good
- Incremental compilation: Partially supported

**fat lto**: Optimizes across all crate boundaries
- Compilation time: Slow (2-3x thin LTO)
- Optimization level: Best
- Incremental compilation: Not supported

**Decision**: Use thin LTO for CI/CD (balance), fat LTO for releases (when available).

### Codegen-Units

`codegen-units = 1` forces single-threaded LLVM optimization.

**Trade-off**:
- ❌ Slower compilation (no parallel codegen)
- ✅ Better optimization (more context for LLVM)
- ✅ Smaller binary size

**Rationale**: CI/CD runs in parallel on GitHub Actions. Single-codegen-unit optimization per build is acceptable.

### Panic Handling

`panic = "abort"` eliminates unwinding machinery.

**Binary size savings**: ~1-2MB
**Runtime impact**: Panics terminate process immediately (no `Drop` execution)

**Rationale**: Production services using structured error handling should never panic. If panic occurs, it's a bug requiring process restart.

## Historical Notes

### Removed Configurations

**toml dependency** (line 91): Removed in favor of environment-only configuration
- Rationale: Environment variables eliminate config file complexity
- No runtime config file parsing
- 12-factor app compliance

**auth-setup binary** (lines 19-21): Commented out, replaced by admin-setup
- Migration: Consolidated authentication setup into admin CLI tool
- Maintains backward compatibility via admin-setup commands
