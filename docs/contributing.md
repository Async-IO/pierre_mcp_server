# contributing

## development setup

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server

# install direnv (optional but recommended)
brew install direnv
direnv allow

# build
cargo build

# run tests
cargo test

# run validation
./scripts/lint-and-test.sh
```

## code standards

### rust idiomatic code

- prefer borrowing (`&T`) over cloning
- use `Result<T, E>` for all fallible operations
- never use `unwrap()` in production code (tests ok)
- document all public apis with `///` comments
- follow rust naming conventions (snake_case)

### error handling

No panics in production code:
```rust
// bad
let value = some_option.unwrap();

// good
let value = some_option.ok_or(MyError::NotFound)?;
```

### forbidden patterns

- `unwrap()`, `expect()`, `panic!()` in src/ (except tests)
- `#[allow(clippy::...)]` attributes
- variables/functions starting with `_` (use meaningful names)
- hardcoded magic values
- `todo!()`, `unimplemented!()` placeholders

### required patterns

- all modules start with aboutme comments:
```rust
// ABOUTME: Brief description of what this module does
// ABOUTME: Second line of description if needed
```

- every `.clone()` must be justified with comment:
```rust
let db = database.clone(); // clone for tokio::spawn thread safety
```

## testing

### test requirements

Every feature needs:
1. **unit tests**: test individual functions
2. **integration tests**: test component interactions
3. **e2e tests**: test complete workflows

No exceptions. If you think a test doesn't apply, ask first.

### running tests

```bash
# all tests
cargo test

# specific test
cargo test test_name

# integration tests
cargo test --test mcp_multitenant_complete_test

# with output
cargo test -- --nocapture

# quiet mode
cargo test --quiet
```

### test patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        // arrange
        let input = setup_test_data();

        // act
        let result = function_under_test(input).await;

        // assert
        assert!(result.is_ok());
    }
}
```

### test location

- unit tests: in same file as code (`#[cfg(test)] mod tests`)
- integration tests: in `tests/` directory
- avoid `#[cfg(test)]` in src/ (tests only)

## workflow

### creating features

1. Create feature branch:
```bash
git checkout -b feature/my-feature
```

2. Implement feature with tests
3. Run validation:
```bash
./scripts/lint-and-test.sh
```

4. Commit:
```bash
git add .
git commit -m "feat: add my feature"
```

5. Push and create pr:
```bash
git push origin feature/my-feature
```

### fixing bugs

Bug fixes go directly to main branch:
```bash
git checkout main
# fix bug
git commit -m "fix: correct issue with X"
git push origin main
```

### commit messages

Follow conventional commits:
- `feat:` - new feature
- `fix:` - bug fix
- `refactor:` - code refactoring
- `docs:` - documentation changes
- `test:` - test additions/changes
- `chore:` - maintenance tasks

No ai assistant references in commits (automated text removed).

## validation

### pre-commit checks

```bash
./scripts/lint-and-test.sh
```

Runs:
1. Clippy with strict lints
2. Pattern validation (no unwrap, no placeholders)
3. All tests
4. Format check

### clippy

```bash
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
```

Zero tolerance for warnings.

### pattern validation

Checks for banned patterns:
```bash
# no unwrap/expect/panic
rg "unwrap\(\)|expect\(|panic!\(" src/

# no placeholders
rg -i "placeholder|todo|fixme" src/

# no clippy allows
rg "#\[allow\(clippy::" src/

# no underscore prefixes
rg "fn _|let _[a-zA-Z]|struct _|enum _" src/
```

### git hooks

Install pre-commit hook:
```bash
./scripts/install-hooks.sh
```

Runs validation automatically before commits.

## architecture guidelines

### dependency injection

Use `Arc<T>` for shared resources:
```rust
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    // ...
}
```

Pass resources to components, not global state.

### protocol abstraction

Business logic in `src/protocols/universal/`. Protocol handlers (mcp, a2a) just translate requests/responses.

```rust
// business logic - protocol agnostic
impl UniversalToolExecutor {
    pub async fn execute_tool(&self, tool: &str, params: Value) -> Result<Value> {
        // implementation
    }
}

// protocol handler - translation only
impl McpHandler {
    pub async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = self.executor.execute_tool(&request.tool, request.params).await;
        // translate to json-rpc response
    }
}
```

### multi-tenant isolation

Every request needs tenant context:
```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
}
```

Database queries filter by tenant_id.

### error handling

Use thiserror for custom errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error")]
    Database(#[from] DatabaseError),
}
```

Propagate with `?` operator.

## adding new features

### new fitness provider

1. Implement `FitnessProvider` trait in `src/providers/`:
```rust
pub struct NewProvider {
    config: ProviderConfig,
    credentials: Option<OAuth2Credentials>,
}

#[async_trait]
impl FitnessProvider for NewProvider {
    fn name(&self) -> &'static str { "new_provider" }
    // ... implement other methods
}
```

2. Register in `src/providers/registry.rs`
3. Add oauth configuration in `src/oauth/`
4. Add tests

### new mcp tool

1. Define tool in `src/protocols/universal/tool_registry.rs`:
```rust
pub const TOOL_MY_FEATURE: ToolDefinition = ToolDefinition {
    name: "my_feature",
    description: "Description of what it does",
    input_schema: ...,
};
```

2. Implement handler in `src/protocols/universal/handlers/`:
```rust
pub async fn handle_my_feature(
    context: &UniversalContext,
    params: Value,
) -> Result<Value> {
    // implementation
}
```

3. Register in tool executor
4. Add unit + integration tests
5. Regenerate SDK types:
```bash
# Ensure server is running
cargo run --bin pierre-mcp-server

# Generate TypeScript types
cd sdk
npm run generate-types
git add src/types.ts
```

**Why**: SDK type definitions are auto-generated from server tool schemas. This ensures TypeScript clients have up-to-date parameter types for the new tool.

### new database backend

1. Implement `DatabaseProvider` trait in `src/database_plugins/`:
```rust
pub struct MyDbProvider { /* ... */ }

#[async_trait]
impl DatabaseProvider for MyDbProvider {
    // implement all methods
}
```

2. Add to factory in `src/database_plugins/factory.rs`
3. Add migration support
4. Add comprehensive tests

## documentation

### code documentation

All public items need doc comments:
```rust
/// Brief description of function
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function errors
pub fn my_function(param: Type) -> Result<Type> {
    // implementation
}
```

### updating docs

After significant changes:
1. Update relevant docs in `docs/`
2. Keep docs concise and accurate
3. Remove outdated information
4. Test all code examples

## getting help

- check existing code for examples
- read rust documentation: https://doc.rust-lang.org/
- ask in github discussions
- open issue for bugs/questions

## review process

1. Automated checks must pass (ci) - see [ci/cd documentation](ci-cd.md)
2. Code review by maintainer
3. All feedback addressed
4. Tests added/updated
5. Documentation updated
6. Merge to main

### ci/cd requirements

All GitHub Actions workflows must pass before merge:
- **Rust**: Core quality gate (formatting, clippy, tests)
- **Backend CI**: Multi-database validation (SQLite + PostgreSQL)
- **Cross-Platform**: OS compatibility (Linux, macOS, Windows)
- **SDK Tests**: TypeScript SDK bridge validation
- **MCP Compliance**: Protocol specification conformance

See [ci/cd.md](ci-cd.md) for detailed workflow documentation, troubleshooting, and local validation commands.

## release process

Handled by maintainers:
1. Version bump in `Cargo.toml`
2. Update changelog
3. Create git tag
4. Publish to crates.io
5. Publish sdk to npm

## code of conduct

- be respectful
- focus on technical merit
- welcome newcomers
- assume good intentions
- provide constructive feedback
