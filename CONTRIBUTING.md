# Contributing to Pierre MCP Server

Thank you for your interest in contributing to Pierre MCP Server! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Adding New Providers](#adding-new-providers)
- [Security](#security)
- [Community](#community)

## Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please read and follow our Code of Conduct:

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on what is best for the community
- Show empathy towards other community members
- Be constructive in your feedback

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/pierre_mcp_server.git
   cd pierre_mcp_server
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/jfarcand/pierre_mcp_server.git
   ```
4. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## How to Contribute

### Reporting Issues

- Check existing issues before creating a new one
- Use issue templates when available
- Provide clear descriptions and steps to reproduce
- Include relevant system information (OS, Rust version, etc.)
- Add logs or error messages when applicable

### Suggesting Features

- Open an issue with the `enhancement` label
- Clearly describe the feature and its benefits
- Provide use cases and examples
- Be open to feedback and alternative approaches

### Submitting Code

- Fork the repository and create a feature branch
- Write clean, well-documented code
- Add tests for new functionality
- Ensure all tests pass
- Submit a pull request with a clear description

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- A code editor (VS Code with rust-analyzer recommended)
- Optional: Docker for containerized testing

### Initial Setup

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/jfarcand/pierre_mcp_server.git
cd pierre_mcp_server

# Build the project
cargo build

# Run tests
cargo test

# Run with example configuration
cp .envrc.example .envrc
# Edit .envrc with your credentials
cargo run
```

### Development Tools

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests with output
cargo test -- --nocapture

# Check for security vulnerabilities
cargo audit

# Generate documentation
cargo doc --open
```

## Project Structure

```
pierre_mcp_server/
├── src/
│   ├── main.rs              # Server entry point
│   ├── lib.rs               # Library root
│   ├── models.rs            # Data models (Activity, Athlete, etc.)
│   ├── config.rs            # Configuration management
│   ├── oauth2_client.rs     # OAuth2 implementation
│   ├── mcp/                 # MCP protocol implementation
│   │   └── mod.rs
│   ├── providers/           # Fitness provider implementations
│   │   ├── mod.rs           # Provider trait and factory
│   │   ├── strava.rs        # Strava integration
│   │   ├── fitbit.rs        # Fitbit integration
│   │   └── ...              # Future providers
│   └── bin/                 # Binary utilities
│       ├── auth_setup.rs    # OAuth setup tool
│       └── find_*.rs        # Example implementations
├── tests/                   # Integration tests
├── examples/                # Usage examples
└── docs/                    # Additional documentation
```

## Coding Standards

### Rust Style Guide

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/README.html)
- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Prefer explicit error handling over `unwrap()`
- Use meaningful variable and function names

### Code Organization

```rust
// Use clear module organization
use std::collections::HashMap;
use anyhow::{Result, Context};

// Group related imports
use crate::models::{Activity, Athlete};
use crate::providers::FitnessProvider;

// Document public APIs
/// Retrieves activities from the specified provider.
///
/// # Arguments
/// * `provider` - The fitness provider name
/// * `limit` - Optional limit on number of activities
///
/// # Returns
/// A vector of activities or an error
pub async fn get_activities(
    provider: &str,
    limit: Option<usize>,
) -> Result<Vec<Activity>> {
    // Implementation
}
```

### Error Handling

```rust
// Use anyhow for error propagation
use anyhow::{Result, Context};

// Provide context for errors
let config = Config::load()
    .context("Failed to load configuration")?;

// Use custom errors when appropriate
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("API rate limit exceeded")]
    RateLimitExceeded,
}
```

## Testing Guidelines

### Test Organization

- Unit tests in the same file as the code
- Integration tests in `tests/` directory
- End-to-end tests for complete workflows

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_creation() {
        // Arrange
        let activity = create_test_activity();
        
        // Act
        let result = process_activity(activity);
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Activity");
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Test async functions
    }
}
```

### Test Coverage

- Aim for >80% code coverage
- Test error cases and edge conditions
- Use property-based testing for complex logic
- Mock external dependencies

## Documentation

### Code Documentation

```rust
/// A fitness activity from any provider.
///
/// Activities represent individual workouts or exercises,
/// containing timing, distance, and performance metrics.
///
/// # Examples
///
/// ```
/// use pierre_mcp_server::models::{Activity, SportType};
/// 
/// let activity = Activity {
///     id: "12345".to_string(),
///     name: "Morning Run".to_string(),
///     sport_type: SportType::Run,
///     // ... other fields
/// };
/// ```
pub struct Activity {
    /// Unique identifier for the activity
    pub id: String,
    // ... other fields
}
```

### README Updates

- Update README.md when adding features
- Include usage examples for new functionality
- Keep configuration examples current
- Document breaking changes

## Pull Request Process

### Before Submitting

1. **Update your branch**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run all checks**:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   cargo doc --no-deps
   ```

3. **Update documentation** as needed

4. **Write a clear PR description**:
   - What changes were made
   - Why the changes are necessary
   - Any breaking changes
   - Related issues

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] All tests pass
- [ ] Added new tests
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No security vulnerabilities
```

### Review Process

1. Automated checks must pass
2. At least one maintainer review required
3. Address all feedback constructively
4. Squash commits if requested
5. Ensure branch is up to date before merging

## Adding New Providers

### Provider Implementation Guide

1. **Create provider file**:
   ```rust
   // src/providers/yourprovider.rs
   use async_trait::async_trait;
   use anyhow::Result;
   use crate::models::{Activity, Athlete, Stats};
   use super::{FitnessProvider, AuthData};

   pub struct YourProvider {
       client: reqwest::Client,
       access_token: Option<String>,
   }

   #[async_trait]
   impl FitnessProvider for YourProvider {
       // Implement required methods
   }
   ```

2. **Add to provider factory**:
   ```rust
   // src/providers/mod.rs
   pub fn create_provider(provider_type: &str) -> Result<Box<dyn FitnessProvider>> {
       match provider_type.to_lowercase().as_str() {
           "yourprovider" => Ok(Box::new(yourprovider::YourProvider::new())),
           // ... other providers
       }
   }
   ```

3. **Implement OAuth2 if needed**:
   - Add provider-specific OAuth2 configuration
   - Implement PKCE for enhanced security
   - Add to auth-setup binary

4. **Add tests**:
   ```rust
   // tests/provider_integration.rs
   #[tokio::test]
   async fn test_yourprovider_authentication() {
       // Test implementation
   }
   ```

5. **Update documentation**:
   - Add setup instructions to README
   - Document API limitations
   - Provide example usage

### Provider Requirements

- Implement all methods in `FitnessProvider` trait
- Handle rate limiting appropriately
- Implement proper error handling
- Add comprehensive tests
- Document any API limitations
- Follow existing patterns for consistency

## Security

### Reporting Security Issues

**Do not open public issues for security vulnerabilities.**

Email security concerns to: [security@pierre-mcp-server.dev]

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fixes (if any)

### Security Best Practices

- Never commit credentials or tokens
- Use environment variables for secrets
- Implement proper token rotation
- Validate all user inputs
- Use HTTPS for all external requests
- Keep dependencies updated

## Community

### Getting Help

- Open an issue for bugs or questions
- Join discussions in GitHub Discussions
- Check existing issues and PRs
- Read the documentation thoroughly

### Contributing to Discussions

- Be respectful and constructive
- Help newcomers get started
- Share your use cases and experiences
- Provide feedback on proposed features

### Recognition

Contributors will be recognized in:
- The project README
- Release notes
- Special thanks section

## License

By contributing to Pierre MCP Server, you agree that your contributions will be licensed under the same dual license as the project:

- Apache License, Version 2.0
- MIT License

Contributors maintain copyright over their contributions while granting the project maintainers and users the rights specified in these licenses.

---

Thank you for contributing to Pierre MCP Server! Your efforts help make fitness data more accessible to AI assistants and developers worldwide. 🏃‍♂️🚴‍♀️🏊‍♂️