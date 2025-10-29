# Changelog

All notable changes to Pierre Fitness Platform will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Rebranding**: Project renamed from "Pierre MCP Server" to "Pierre Fitness Platform"
  - Updated all documentation to reflect new branding
  - Name better represents the multi-protocol nature (MCP, A2A, OAuth2, REST)
  - "Platform" emphasizes extensibility and comprehensive fitness data infrastructure
  - All user-facing documentation, templates, and assets updated
  - Technical identifiers (binary names, environment variables) unchanged for backward compatibility

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

[0.1.0]: https://github.com/Async-IO/pierre_mcp_server/releases/tag/v0.1.0
