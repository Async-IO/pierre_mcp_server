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

no panics in production code:
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

every feature needs:
1. **unit tests**: test individual functions
2. **integration tests**: test component interactions
3. **e2e tests**: test complete workflows

no exceptions. if you think a test doesn't apply, ask first.

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

1. create feature branch:
```bash
git checkout -b feature/my-feature
```

2. implement feature with tests
3. run validation:
```bash
./scripts/lint-and-test.sh
```

4. commit:
```bash
git add .
git commit -m "feat: add my feature"
```

5. push and create pr:
```bash
git push origin feature/my-feature
```

### fixing bugs

bug fixes go directly to main branch:
```bash
git checkout main
# fix bug
git commit -m "fix: correct issue with X"
git push origin main
```

### commit messages

follow conventional commits:
- `feat:` - new feature
- `fix:` - bug fix
- `refactor:` - code refactoring
- `docs:` - documentation changes
- `test:` - test additions/changes
- `chore:` - maintenance tasks

no ai assistant references in commits (automated text removed).

## validation

### pre-commit checks

```bash
./scripts/lint-and-test.sh
```

runs:
1. clippy with strict lints
2. pattern validation (no unwrap, no placeholders)
3. all tests
4. format check

### clippy

```bash
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings
```

zero tolerance for warnings.

### pattern validation

checks for banned patterns:
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

install pre-commit hook:
```bash
./scripts/install-hooks.sh
```

runs validation automatically before commits.

## architecture guidelines

### dependency injection

use `Arc<T>` for shared resources:
```rust
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    // ...
}
```

pass resources to components, not global state.

### protocol abstraction

business logic in `src/protocols/universal/`. protocol handlers (mcp, a2a) just translate requests/responses.

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

every request needs tenant context:
```rust
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role: TenantRole,
}
```

database queries filter by tenant_id.

### error handling

use thiserror for custom errors:
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error")]
    Database(#[from] DatabaseError),
}
```

propagate with `?` operator.

## adding new features

### new fitness provider

1. implement `FitnessProvider` trait in `src/providers/`:
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

2. register in `src/providers/registry.rs`
3. add oauth configuration in `src/oauth/`
4. add tests

### new mcp tool

1. define tool in `src/protocols/universal/tool_registry.rs`:
```rust
pub const TOOL_MY_FEATURE: ToolDefinition = ToolDefinition {
    name: "my_feature",
    description: "Description of what it does",
    input_schema: ...,
};
```

2. implement handler in `src/protocols/universal/handlers/`:
```rust
pub async fn handle_my_feature(
    context: &UniversalContext,
    params: Value,
) -> Result<Value> {
    // implementation
}
```

3. register in tool executor
4. add unit + integration tests

### new database backend

1. implement `DatabaseProvider` trait in `src/database_plugins/`:
```rust
pub struct MyDbProvider { /* ... */ }

#[async_trait]
impl DatabaseProvider for MyDbProvider {
    // implement all methods
}
```

2. add to factory in `src/database_plugins/factory.rs`
3. add migration support
4. add comprehensive tests

## documentation

### code documentation

all public items need doc comments:
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

after significant changes:
1. update relevant docs in `docs/`
2. keep docs concise and accurate
3. remove outdated information
4. test all code examples

## getting help

- check existing code for examples
- read rust documentation: https://doc.rust-lang.org/
- ask in github discussions
- open issue for bugs/questions

## review process

1. automated checks must pass (ci)
2. code review by maintainer
3. all feedback addressed
4. tests added/updated
5. documentation updated
6. merge to main

## release process

handled by maintainers:
1. version bump in `Cargo.toml`
2. update changelog
3. create git tag
4. publish to crates.io
5. publish sdk to npm

## code of conduct

- be respectful
- focus on technical merit
- welcome newcomers
- assume good intentions
- provide constructive feedback
