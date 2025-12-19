# Changelog

All notable changes to Pierre Fitness Platform will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
And this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Redis cache backend** support for distributed caching in multi-instance deployments
  - Pluggable cache architecture with in-memory and Redis implementations
  - Automatic fallback to in-memory cache when Redis URL not configured
  - Connection pooling with automatic reconnection handling
  - Pattern-based cache invalidation using Redis SCAN
  - Namespace isolation for shared Redis instances
  - Health check support via Redis PING

### Changed

### Fixed

## [0.2.0] - 2025-12-19

### Added

#### Intelligence and Analytics
- **Real intelligence system** with scientific algorithms replacing placeholder logic
  - Training Load Analysis: TSS (Training Stress Score), CTL (Chronic Training Load), ATL (Acute Training Load), TSB (Training Stress Balance)
  - Race Predictions: VDOT-based predictions using Jack Daniels' VO2max formula, Riegel formula for distance scaling
  - Statistical Analysis: Linear regression for performance trends, R² coefficient for fit quality
  - Pattern Detection: Weekly training consistency, hard/easy workout alternation, volume progression
  - Physiological Validation: Bounds checking for heart rate, power, VO2 max
- **Sleep and recovery intelligence system** with NSF/AASM-validated scoring
  - 5 new MCP tools for sleep analysis and recovery tracking
  - Sleep quality scoring based on National Sleep Foundation guidelines
  - Recovery readiness calculations
  - 82 comprehensive tests with scientific methodology documentation
- **Nutrition analysis module** with USDA FoodData Central integration
  - Macro and micronutrient tracking
  - Integration with USDA nutritional database
  - Meal logging and analysis tools
- **Automated intelligence testing framework** with 30 integration tests using synthetic data
  - Tests for all intelligence tools without OAuth dependencies
  - Comprehensive test coverage documentation

#### Authentication & Security
- **OAuth2 Authorization Server** enhancements
  - PKCE (Proof Key for Code Exchange) enforcement for security
  - JWKS (JSON Web Key Set) endpoint with RS256 key rotation
  - Per-IP rate limiting with token bucket protection (RFC-compliant headers)
  - ETag caching for JWKS endpoint optimization
  - Server-side OAuth2 state validation
  - HTTPS issuer validation
- **JWT infrastructure** migration from HS256 to RS256 asymmetric signing
  - RSA key pair generation and persistence
  - RFC 7519 compliance (iss, jti, iat claims)
  - Automatic OAuth token refresh via `/api/oauth/validate-and-refresh` endpoint
  - Token expiration validation and renewal
- **Privacy and data protection**
  - PII-safe logging with automatic redaction middleware
  - Sensitive data masking in logs (tokens, passwords, API keys)
- **Structured error handling** improvements
  - Eliminated all `anyhow!()` macro violations (29 files updated)
  - Proper `AppError`, `DatabaseError`, `ProviderError` usage throughout
  - Zero-tolerance enforcement in CI pipeline

#### Data Access & APIs
- **Cursor-based pagination** for efficient large dataset traversal
  - Complete feature documentation
  - Performance optimization for large result sets
- **Detailed Strava activity data** with opt-in fetching
  - Extended activity metadata support
  - Granular data control for bandwidth optimization

#### Infrastructure & Reliability
- **Plugin lifecycle management** system
  - Structured plugin initialization and teardown
  - Resource cleanup and state management
- **Resilience improvements**
  - Automatic retries for transient failures
  - Configurable timeouts across all external calls
  - SSE (Server-Sent Events) buffer management for connection stability

#### Performance Optimizations
- **String to &str parameter optimization** in config and progress tracking modules
  - Reduced allocations and improved memory efficiency
  - Eliminated 34 runtime `env::var()` calls via centralized configuration
- **Async bcrypt** with `spawn_blocking` for non-blocking password hashing
- **Rate limiting** with DashMap replacing Mutex for concurrent access

### Changed
- **Project Rebranding**: "Pierre MCP Server" → "Pierre Fitness Platform"
  - Updated all documentation to reflect new branding
  - Name better represents the multi-protocol nature (MCP, A2A, OAuth2, REST)
  - "Platform" emphasizes extensibility and comprehensive fitness data infrastructure
  - All user-facing documentation, templates, and assets updated
  - Technical identifiers (binary names, environment variables) unchanged for backward compatibility
- **OAuth callback URL corrections** throughout documentation
  - Standardized to `/api/oauth/callback/{provider}` path
  - Updated authentication flow documentation

### Fixed
- **Security vulnerabilities** in OAuth2 and JWT implementation
  - Token redaction in API request/response logs
  - Atomic token operations to prevent TOCTOU race conditions
  - Encryption and JWT persistence issues (separate OAuth nonces, persist RSA keys across restarts)
  - CVE-2025-62522 path traversal vulnerability (updated Vite to 6.4.1)
- **Intelligence calculations**
  - TSS (Training Stress Score) calculation accuracy
  - Intelligence tool response field name corrections
- **Cross-platform compatibility**
  - RSA key sorting for Windows timestamp resolution
  - Key rotation timing for Windows second-precision timestamps
- **Build and CI issues**
  - CI timeout issues in MCP compliance and PostgreSQL tests
  - GitHub Actions disk space issues with clean builds
  - Test regressions from config refactoring
- **Code quality improvements**
  - String validation for edge cases
  - Clippy warnings across codebase
  - Eliminated mock implementations from production code
- **Developer experience**
  - TTY support for interactive terminal features
  - Commit guard performance optimization

### Documentation
- **Intelligence system methodology** documentation with scientific references
  - Detailed formula explanations and implementation notes
  - Sports science validation and bounds checking
- **OAuth client documentation** improvements
  - Simplified README OAuth section
  - Technical details moved to `oauth-client.md`
  - Remote MCP configuration updates
- **Testing framework documentation**
  - Comprehensive guide for intelligence testing
  - Synthetic data generation patterns

### Architecture & Code Quality
- **Dependency injection** architecture
  - Replaced provider global singleton with DI pattern
  - Comprehensive ServerConfig dependency injection across codebase
  - HTTP client, API endpoint, and SSE timeout configuration via DI
  - Eliminated 34 runtime `env::var()` calls with centralized configuration
- **Memory safety** improvements
  - Replaced unsafe FFI with `sysinfo` crate for health monitoring
  - Eliminated all unsafe code blocks in core functionality
- **Module organization**
  - OAuth modules renamed to role-based structure (`oauth2_server`/`oauth2_client`)
  - OAuth callback HTML templates extracted to dedicated files with 30-second auto-close
  - Documentation reorganized for better discoverability
- **Type safety** enhancements
  - Type-safe newtypes for domain modeling
  - Dead code removal and idiomatic Rust patterns
  - Enhanced clone usage validation (743 clones analyzed, 0 warnings)
- **Branding and UI**
  - Energy wave logo design replacing activity rings
  - SVG logo for scalability, PNG fallback for compatibility
  - Unified OAuth template design system with Pierre branding
- **CI/CD optimizations**
  - Faster builds with improved caching
  - Optimized test execution times

## [0.1.0] - 2025-10-14

### Added

#### Core Protocol Support
- **MCP (Model Context Protocol)** implementation with 25 tools for fitness data access
  - Tool registry with activity retrieval, analysis, and intelligence tools
  - Goal tracking and progress monitoring tools
  - Performance analysis and training recommendations
  - Configuration management tools
- **OAuth 2.0 Authorization Server** with RFC 7591 dynamic client registration
  - Server metadata endpoint (RFC 8414)
  - Dynamic client registration for MCP clients
  - Authorization and token endpoints
  - JSON Web Key Set (JWKS) endpoint
- **A2A (Agent-to-Agent) Protocol** for autonomous AI system communication
  - Agent card capability discovery
  - Cryptographic authentication between agents
  - Asynchronous messaging protocol
  - Protocol versioning (A2A 1.0.0)

#### Authentication & Security
- **Multi-tenant architecture** with isolated data per organization
- **JWT authentication** for REST API access
- **API key authentication** for service-to-service integration
- **Two-tier encryption system** for OAuth credentials
  - Master encryption key for at-rest encryption
  - Tenant-specific encryption keys

#### Data Integration
- **Strava provider** integration with OAuth 2.0 flow
  - Activity retrieval and analysis
  - Athlete profile and statistics
  - Webhook support for real-time updates
- **Garmin Connect** provider integration
  - OAuth 1.0a authentication
  - Activity data synchronization
  - Shared utility functions for provider management
- **OpenWeather API** integration for environmental data
- **Pluggable cache system** with in-memory LRU backend
  - Configurable max entries and cleanup intervals
  - TTL-based expiration
  - Cache invalidation patterns
  - Prepared for Redis backend integration

#### Intelligence & Analysis
- **Activity intelligence** with AI-powered insights
  - Performance trend analysis
  - Pattern detection in training data
  - Personalized training recommendations
  - Fitness score calculation
  - Performance prediction models
  - Training load analysis
- **Goal management system**
  - Goal setting and tracking
  - AI-suggested goals based on history
  - Feasibility analysis
  - Progress monitoring
- **Configuration profiles** for training zones and parameters
  - Pre-defined profiles (endurance, speed work, recovery)
  - Personalized zone calculation
  - Parameter validation

#### Real-Time Features
- **Server-Sent Events (SSE)** for real-time notifications
  - OAuth authorization completion events
  - OAuth error notifications
  - System status updates
  - Sequential event IDs for reconnection
  - 15-second keepalive intervals
- **Automatic OAuth credential validation** and cleanup

#### Developer Tools
- **Pierre SDK** for MCP client integration
  - Automatic OAuth 2.0 flow handling
  - Token management and refresh
  - Command-line interface
- **Management dashboard** (React + TypeScript)
  - User management and approval
  - API key monitoring
  - Usage analytics
  - Real-time request monitoring
- **Comprehensive test suite**
  - Unit tests for all modules
  - Integration tests for MCP protocol
  - Multi-tenant workflow tests
  - Database plugin tests (SQLite + PostgreSQL)
  - MCP compliance tests
- **Code quality automation**
  - TOML-based validation patterns
  - Pre-commit hooks
  - Clippy linting with strict warnings
  - Automated CI/CD pipelines

#### Documentation
- Complete developer guide with 17 sections
- Installation guides for Claude Desktop and ChatGPT
- API reference documentation
- Architecture documentation with diagrams
- Security best practices guide
- Plugin development guide
- Logging and observability guide

### Database Support
- **SQLite** backend for development and testing
- **PostgreSQL** backend for production deployments
- Connection pooling with configurable limits
- Automatic schema migrations
- Database plugin architecture

### Performance & Optimization
- **LRU cache** with bounded memory usage (DoS prevention)
- **Connection pooling** for database efficiency
- **Optimized release builds** with LTO and code stripping
- **Binary size optimization** (target: <50MB)

### Infrastructure
- **GitHub Actions CI/CD** with comprehensive workflows
  - Backend tests (SQLite + PostgreSQL)
  - Frontend tests with coverage
  - Cross-platform tests (Linux, macOS, Windows)
  - MCP compliance tests
  - Database tests
  - Code quality validation
- **Docker support** for PostgreSQL in CI
- **Multi-platform builds** preparation

### Configuration
- Environment-based configuration system
- `.envrc` support with direnv integration
- Comprehensive environment variable documentation
- Configuration validation and defaults

### Security
- No known vulnerabilities in initial release
- Encrypted OAuth credentials at rest
- Secure JWT token generation and validation
- API rate limiting preparation
- Input validation and sanitization

## [Unreleased]

### Planned Features
- Redis cache backend support
- Additional fitness provider integrations
- Enhanced AI analysis models
- Webhook support for more providers
- Multi-language SDK support
- Performance benchmarking suite

---

**Version Format**: MAJOR.MINOR.PATCH
- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

[0.2.0]: https://github.com/Async-IO/pierre_mcp_server/releases/tag/v0.2.0
[0.1.0]: https://github.com/Async-IO/pierre_mcp_server/releases/tag/v0.1.0
