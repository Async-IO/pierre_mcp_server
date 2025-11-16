# Pierre Fitness Platform - Comprehensive Rust Tutorial
## Table of Contents (Revised)

> **Target Audience**: Junior Rust developers with 6-12 months experience
>
> **Prerequisites**: Basic knowledge of ownership, async/await, traits, and error handling
>
> **Estimated Duration**: 60-80 hours of learning content

---

## Part I: Foundation & Project Structure (Chapters 1-4)

### Chapter 1: Project Architecture & Module Organization
- Understanding the Pierre codebase structure (src/, sdk/, tests/, templates/)
- Rust module system: `lib.rs` as the central hub
- Public API design with `pub mod` declarations
- Directory structure and file organization patterns
- **Rust Idioms**:
  - Module organization patterns (src/lib.rs:57-189)
  - Feature flags and conditional compilation (#[cfg])
  - Documentation comments with //! and ///
  - Absolute vs relative imports
- **Code Examples**:
  - src/lib.rs module declarations
  - Crate structure analysis

### Chapter 2: Error Handling & Type-Safe Errors
- Moving from `anyhow` to structured errors
- The `thiserror` crate for custom error types
- `Result<T, E>` propagation patterns
- Error hierarchies: AppError, DatabaseError, ProviderError, ProtocolError
- **Rust Idioms**:
  - Never use `anyhow::anyhow!()` in production (CLAUDE.md directive)
  - Error enum design with `thiserror` derive macro
  - `?` operator for error propagation
  - From/Into trait implementations for error conversion
  - Error context with `.map_err()`
- **Code Examples**:
  - src/errors.rs - AppError enum
  - src/database/errors.rs - DatabaseError
  - src/providers/errors.rs - ProviderError
  - src/constants/errors/codes.rs - Error code system

### Chapter 3: Configuration Management & Environment Variables
- Environment-driven configuration with `dotenvy`
- Type-safe configuration with `clap` and serde
- Static configuration initialization patterns
- Algorithm configuration system (VDOT, TRIMP, TSS variants)
- **Rust Idioms**:
  - Builder pattern for configuration structs
  - `OnceLock` for global state initialization
  - Environment variable parsing with fallback defaults
  - Validation at compile-time vs runtime
- **Code Examples**:
  - src/config/environment.rs - ServerConfig
  - src/config/intelligence_config.rs - Algorithm variants
  - src/constants/mod.rs - Static initialization pattern
  - src/bin/pierre-mcp-server.rs:65-79 - Configuration setup

### Chapter 4: Dependency Injection with Context Pattern
- Focused dependency injection contexts
- Avoiding monolithic "AppState" anti-patterns
- Sharing state with Arc<T>
- The ServerResources pattern
- **Rust Idioms**:
  - Smart pointers: Arc for shared ownership across threads
  - Clone is cheap for Arc (reference counting)
  - Interior mutability patterns (Mutex, RwLock, DashMap)
  - Type-state pattern for compile-time guarantees
- **Code Examples**:
  - src/context/ module structure (auth.rs, config.rs, data.rs)
  - src/mcp/resources.rs - ServerResources (lines 20-89)
  - src/bin/pierre-mcp-server.rs:182-220 - Resource initialization

---

## Part II: Authentication & Security (Chapters 5-8)

### Chapter 5: Cryptographic Key Management
- Two-tier key management system (MEK + DEK)
- RSA key generation for JWT signing (RS256)
- JWKS (JSON Web Key Set) management and rotation
- Database encryption key derivation
- **Rust Idioms**:
  - `zeroize` crate for secure memory cleanup
  - const generics for fixed-size buffers
  - Type-state pattern for key lifecycle
  - Drop trait for automatic cleanup
- **Code Examples**:
  - src/key_management.rs - Two-tier bootstrap (lines 40-120)
  - src/crypto/keys.rs - RSA key generation
  - src/admin/jwks.rs - JWKS manager (lines 30-180)
  - Secure key rotation patterns

### Chapter 6: JWT Authentication with RS256
- Asymmetric JWT signing (RS256 vs HS256)
- JWT token generation and validation
- Claims-based authorization (user_id, tenant_id, exp)
- Token refresh strategies
- **Rust Idioms**:
  - Struct field validation with custom types
  - Serialization with serde derive macros (#[serde(rename)])
  - Timestamp handling with chrono
  - Builder pattern for JWT claims
- **Code Examples**:
  - src/admin/jwt.rs - Token generation (lines 50-150)
  - src/auth.rs - AuthManager (lines 20-200)
  - src/middleware/auth.rs - JWT extraction from headers

### Chapter 7: Multi-Tenant Database Isolation
- Tenant-based data segregation
- Database encryption with per-tenant keys
- SQL injection prevention with sqlx
- Database abstraction layer (SQLite vs PostgreSQL)
- **Rust Idioms**:
  - Compile-time SQL verification with sqlx macros
  - Borrowing vs cloning in database queries (&str vs String)
  - async-trait for database abstraction
  - Error handling with sqlx::Error
- **Code Examples**:
  - src/database/users.rs - User queries with tenant isolation
  - src/database_plugins/ - Database factory pattern
  - src/database_plugins/sqlite.rs vs postgres.rs
  - Tenant context extraction (src/mcp/tenant_isolation.rs)

### Chapter 8: Middleware & Request Context
- Axum middleware architecture
- Request ID tracking and distributed tracing
- PII redaction in logs
- Rate limiting per tenant
- **Rust Idioms**:
  - Tower service layers and middleware composition
  - Extension types for request context
  - Async closures and Future combinators
  - Type-safe header extraction
- **Code Examples**:
  - src/middleware/auth.rs - JWT middleware
  - src/middleware/tracing.rs - Request tracing
  - src/middleware/redaction.rs - PII removal
  - src/middleware/rate_limiting.rs - Per-tenant limits

---

## Part III: MCP Protocol Implementation (Chapters 9-12)

### Chapter 9: JSON-RPC 2.0 Foundation
- Understanding the JSON-RPC 2.0 specification
- Request/response message structure
- Error handling in JSON-RPC (error codes -32700 to -32603)
- Shared foundation for MCP and A2A protocols
- **Rust Idioms**:
  - Serde derive for automatic serialization
  - #[serde(rename_all = "camelCase")] attributes
  - Generic type parameters for flexible responses
  - Option<T> for nullable fields in JSON
- **Code Examples**:
  - src/jsonrpc/mod.rs - JsonRpcRequest/JsonRpcResponse (lines 10-80)
  - Error code constants and types
  - JSON-RPC batch request handling

### Chapter 10: MCP Protocol Deep Dive - Request Flow
- Complete MCP protocol lifecycle (Initialize → Tools/List → Tools/Call)
- Protocol version negotiation (2024-11-05, 2025-06-18)
- Server state machine (Uninitialized → Initialized → Ready → Shutdown)
- Capability discovery and negotiation
- **Rust Idioms**:
  - Enum-based state machines
  - Pattern matching for request routing
  - Type-safe protocol versioning
  - Builder pattern for responses
- **Code Examples**:
  - src/mcp/protocol.rs - handle_initialize (lines 48-136)
  - src/mcp/server_lifecycle.rs - State machine
  - Protocol negotiation logic
  - Capability discovery implementation

### Chapter 11: MCP Transport Layers (HTTP, stdio, WebSocket, SSE)
- HTTP transport with Axum (POST /mcp)
- stdio bridge for subprocess communication
- WebSocket streaming support (ws://localhost:8081/mcp/ws)
- Server-Sent Events for notifications (GET /mcp/sse)
- **Rust Idioms**:
  - Protocol abstraction with traits
  - tokio::process for subprocess management
  - futures::stream::Stream for async iteration
  - Type-safe upgrade handlers (WebSocketUpgrade)
- **Code Examples**:
  - src/routes/mcp.rs - HTTP endpoint (lines 20-80)
  - src/routes/websocket.rs - WebSocket handler
  - src/routes/mcp_sse_routes.rs - SSE stream
  - src/mcp/transport_manager.rs - Transport abstraction

### Chapter 12: MCP Tool Registry & Type-Safe Routing
- Type-safe tool identification with ToolId enum
- Tool registration and discovery
- Dynamic tool dispatch without string matching
- Tool parameter validation and schemas
- **Rust Idioms**:
  - Enum-based tool identification (35+ variants)
  - Pattern matching for tool dispatch
  - Const fn for compile-time tool metadata
  - Function pointers for async handlers
- **Code Examples**:
  - src/protocols/universal/tool_registry.rs - ToolId enum (lines 14-99)
  - Tool registration patterns (lines 136-323 in executor.rs)
  - src/protocols/universal/executor.rs - Tool execution (lines 325-361)
  - Handler signature patterns (async vs sync tools)

---

## Part IV: SDK & Type System (Chapters 13-14)

### Chapter 13: SDK Bridge Architecture & stdio Transport
- TypeScript SDK for stdio transport clients (Claude Desktop, ChatGPT)
- Bidirectional MCP bridge: stdio ↔ HTTP + OAuth
- OAuth 2.0 client provider implementation
- Secure token storage with OS keychain (keytar)
- **TypeScript/Rust Patterns**:
  - Class-based architecture in TypeScript
  - Promise-based async patterns
  - Event-driven communication (stdio streams)
  - Cross-language protocol mapping
- **Code Examples**:
  - sdk/src/bridge.ts - PierreMcpClient class (lines 1040-2309)
  - sdk/src/bridge.ts - OAuth provider (lines 113-1038)
  - sdk/src/secure-storage.ts - Keychain integration
  - OAuth flow orchestration (lines 1785-2024)
  - SDK startup sequence (lines 1069-1094)

### Chapter 14: Type Generation & Tools-to-Types System
- Auto-generating TypeScript types from Rust tool schemas
- JSON Schema to TypeScript conversion
- Maintaining type safety across language boundaries
- CI/CD integration for type generation
- **TypeScript/JavaScript Idioms**:
  - AST transformation for code generation
  - Interface generation from JSON schemas
  - Type unions and discriminated unions
  - Generic type parameters
- **Code Examples**:
  - scripts/generate-sdk-types.js - Type generator (12,964 bytes)
  - sdk/src/types.ts - Auto-generated interfaces
  - Tool schema extraction via tools/list
  - npm run generate-types workflow
  - CI validation of type sync

---

## Part V: OAuth 2.0, A2A & Provider Integration (Chapters 15-18)

### Chapter 15: OAuth 2.0 Server Implementation (RFC 7591)
- Dynamic client registration (RFC 7591)
- Authorization code flow with PKCE
- Token exchange and refresh mechanisms
- JWKS endpoint for public key distribution
- **Rust Idioms**:
  - State machine pattern for OAuth flows
  - Secure random generation with `ring` crate
  - URL encoding and query parameter handling
  - Base64 encoding for code challenges
- **Code Examples**:
  - src/oauth2_server/endpoints.rs - Authorization endpoint
  - src/oauth2_server/client_registration.rs - RFC 7591 (lines 40-150)
  - Token exchange implementation
  - PKCE verification (code_challenge vs code_verifier)

### Chapter 16: OAuth 2.0 Client for Fitness Providers
- Multi-provider OAuth client architecture
- Strava, Garmin, Fitbit integration
- Token storage and automatic refresh
- Provider-specific OAuth quirks
- **Rust Idioms**:
  - Trait objects for provider abstraction
  - reqwest for HTTP client patterns
  - async/await error handling with ?
  - Retry logic with exponential backoff
- **Code Examples**:
  - src/oauth2_client/client.rs - Generic OAuth client
  - src/providers/strava_provider.rs - Strava implementation
  - src/providers/garmin_provider.rs - Garmin quirks
  - src/providers/fitbit.rs - Fitbit integration
  - Token refresh strategies (src/oauth2_client/flow_manager.rs)

### Chapter 17: Provider Data Models & Rate Limiting
- Mapping provider-specific data to common models
- Activity, athlete, and stats type normalization
- Cursor-based pagination for large datasets
- Per-tenant rate limiting strategies
- **Rust Idioms**:
  - From/Into traits for type conversions
  - Option<T> for nullable fields
  - Iterator adapters for data transformation
  - DashMap for concurrent rate limiting
- **Code Examples**:
  - src/models.rs - Common data models
  - src/pagination.rs - Cursor pagination
  - src/providers/core.rs - Provider trait
  - src/rate_limiting.rs - Rate limiter (lines 30-200)
  - src/middleware/rate_limiting.rs - Middleware integration

### Chapter 18: A2A Protocol - Agent-to-Agent Communication
- A2A protocol architecture and use cases
- Agent cards for capability discovery
- Task management for long-running operations
- A2A vs MCP: key differences
- **Rust Idioms**:
  - Shared JSON-RPC foundation with MCP
  - Enum-based task status tracking
  - Async task execution patterns
  - State machine for task lifecycle
- **Code Examples**:
  - src/a2a/protocol.rs - A2A implementation (lines 1-1023)
  - src/a2a/agent_card.rs - Capability discovery
  - src/a2a/auth.rs - System user authentication
  - Task creation and status tracking (lines 230-261)
  - .well-known/agent.json endpoint

---

## Part VI: Tools & Intelligence System (Chapters 19-22)

### Chapter 19: Comprehensive Tools Guide - All 35+ MCP Tools
- Complete catalog of all fitness tools
- Tool categorization (Core API, Intelligence, Goals, Configuration, Sleep, Nutrition)
- Tool parameter schemas and validation
- How to activate tools via natural language prompts
- **Tool Categories**:
  - **Core Strava API** (8 tools): get_activities, get_athlete, get_stats, analyze_activity, get_activity_intelligence, get_connection_status, connect_provider, disconnect_provider
  - **Goals & Planning** (4 tools): set_goal, suggest_goals, analyze_goal_feasibility, track_progress
  - **Intelligence & Analysis** (9 tools): calculate_metrics, analyze_performance_trends, compare_activities, detect_patterns, generate_recommendations, calculate_fitness_score, predict_performance, analyze_training_load
  - **Configuration** (6 tools): get_configuration_catalog, get_configuration_profiles, get_user_configuration, update_user_configuration, calculate_personalized_zones, validate_configuration
  - **Sleep & Recovery** (5 tools): analyze_sleep_quality, calculate_recovery_score, suggest_rest_day, track_sleep_trends, optimize_sleep_schedule
  - **Nutrition & USDA** (5 tools): calculate_daily_nutrition, get_nutrient_timing, search_food, get_food_details, analyze_meal_nutrition
- **Example Prompts for Each Tool**:
  - "Show me my last 10 activities" → get_activities
  - "Calculate my daily nutrition for marathon training" → calculate_daily_nutrition
  - "Do I need a rest day?" → suggest_rest_day
  - "Analyze my training load this month" → analyze_training_load
- **Code Examples**:
  - src/protocols/universal/tool_registry.rs - All ToolId variants (lines 14-99)
  - src/protocols/universal/handlers/ - Handler implementations
  - Tool descriptions and schemas (lines 190-269)
  - Natural language → tool mapping patterns

### Chapter 20: Sports Science Algorithms & Intelligence
- Training Stress Score (TSS) calculation from power/HR
- VO2max estimation (VDOT formulas: Daniels, Riegel)
- Heart rate zone calculation (Fox, Tanaka, Nes, Gulati)
- Algorithm variants and selection via configuration
- **Rust Idioms**:
  - f64 for floating-point calculations
  - num-traits for generic numeric operations
  - const fn for compile-time constants
  - Enum dispatch for algorithm selection
- **Code Examples**:
  - src/intelligence/algorithms/tss.rs - TSS calculation (avg_power, normalized_power, hybrid)
  - src/intelligence/algorithms/vdot.rs - VDOT estimation
  - src/intelligence/algorithms/maxhr.rs - Max HR algorithms
  - src/intelligence/algorithms/trimp.rs - TRIMP variants (Bannister, Edwards, Lucia, Hybrid)
  - Algorithm configuration (src/config/intelligence_config.rs)

### Chapter 21: Training Load, Recovery & Sleep Analysis
- CTL (Chronic Training Load) and ATL (Acute Training Load)
- TSB (Training Stress Balance) for form tracking
- NSF/AASM sleep quality scoring
- HRV-based recovery assessment
- **Rust Idioms**:
  - Vec<T> and iterator chains for data processing
  - Default trait for sensible defaults
  - Struct composition for complex scoring
  - Range patterns for scoring thresholds
- **Code Examples**:
  - src/intelligence/training_load.rs - CTL/ATL/TSB (lines 40-300)
  - src/intelligence/algorithms/training_load.rs - EMA, SMA, WMA, Kalman
  - src/intelligence/sleep_analysis.rs - Sleep scoring (lines 20-250)
  - src/intelligence/recovery_calculator.rs - Recovery aggregation
  - src/intelligence/algorithms/recovery_aggregation.rs - Multiple strategies

### Chapter 22: Nutrition System & USDA Integration
- BMR/TDEE calculation (Mifflin-St Jeor formula)
- Macronutrient recommendations by sport type
- USDA FoodData Central API integration (350,000+ foods)
- Nutrient timing (ISSN guidelines)
- **Rust Idioms**:
  - External API client patterns with reqwest
  - JSON parsing with serde_json
  - Error handling for external dependencies
  - Caching strategies for API responses
- **Code Examples**:
  - src/intelligence/nutrition_calculator.rs - BMR/TDEE (lines 30-200)
  - src/external/usda_client.rs - USDA API client
  - src/protocols/universal/handlers/nutrition.rs - Tool handlers
  - Nutrient timing calculations
  - Food search and analysis

---

## Part VII: Testing, Design & Deployment (Chapters 23-25)

### Chapter 23: Testing Framework - Synthetic Data & E2E Tests
- Synthetic data generation for algorithm testing
- Training pattern simulation (beginner improvement, overtraining, injury recovery)
- Tools-to-types validation in CI/CD
- Multi-tenant MCP testing infrastructure
- **Rust Idioms**:
  - #[cfg(test)] modules and test organization
  - Test-only features with #[cfg(any(test, feature = "testing"))]
  - assert_eq!, assert! macros with custom messages
  - #[tokio::test] for async tests
  - serial_test for test isolation
  - tempfile for temporary test databases
- **Code Examples**:
  - tests/helpers/synthetic_data.rs - SyntheticDataBuilder (lines 1-611)
  - Training patterns: BeginnerRunnerImproving, ExperiencedCyclistConsistent, Overtraining, InjuryRecovery
  - tests/helpers/test_utils.rs - Test utilities
  - tests/mcp_multitenant_sdk_e2e_test.rs - Multi-tenant E2E
  - SDK E2E tests: sdk/test/e2e/ (10 test scenarios)
  - Type generation validation: scripts/generate-sdk-types.js
  - CI workflows: .github/workflows/sdk-tests.yml

### Chapter 24: Design System - Templates, Frontend & User Experience
- OAuth HTML templates (success/error pages)
- Template variable substitution ({{PROVIDER}}, {{ERROR}})
- Frontend design patterns (React/Vue components)
- Admin dashboard architecture
- **Design Patterns**:
  - Responsive design for OAuth consent screens
  - Real-time updates with SSE
  - Token management UI
  - Analytics visualization
- **Code Examples**:
  - templates/oauth_success.html - OAuth completion page
  - templates/oauth_error.html - Error handling
  - sdk/templates/ - SDK template usage
  - frontend/ - Dashboard components (structure TBD)
  - SSE integration for real-time updates

### Chapter 25: Production Deployment, Clippy & Performance
- Docker containerization (multi-stage builds)
- Database migration strategies
- Health checks and observability
- Clippy configuration for zero-tolerance errors
- Performance optimization (LTO, codegen-units)
- **Rust Idioms**:
  - Release profile configuration (Cargo.toml lines 54-62)
  - LTO (Link-Time Optimization) settings: thin vs fat
  - tracing for structured logging
  - OpenTelemetry integration
  - Clippy lints configuration (Cargo.toml lines 140-212)
- **Code Examples**:
  - Dockerfile - Multi-stage build
  - docker-compose.yml - Service orchestration
  - src/health.rs - Health monitoring (lines 20-200)
  - src/logging/mod.rs - Structured logging
  - Cargo.toml [lints.clippy] - Zero-tolerance policies
  - scripts/architectural-validation.sh - Pre-commit validation
  - .github/workflows/ci.yml - CI/CD pipeline
  - Production configuration patterns

---

## Appendices

### Appendix A: Rust Idioms Reference
Quick reference for all Rust patterns used throughout the tutorial:
- **Ownership & Borrowing**: &T vs &mut T vs T, lifetime elision
- **Error Handling**: Result<T, E>, ?, thiserror, From/Into
- **Async/Await**: tokio runtime, async fn, .await, Pin, Future
- **Trait Design**: trait objects, associated types, generic constraints
- **Smart Pointers**: Arc<T>, Box<T>, Rc<T>, Cow<'a, T>
- **Concurrency**: Send + Sync, Mutex, RwLock, DashMap
- **Performance**: Zero-cost abstractions, inline, const fn
- **Serialization**: serde derive, custom serializers, #[serde(...)]

### Appendix B: CLAUDE.md Compliance Checklist
- **Zero-Tolerance Patterns**: No `anyhow::anyhow!()`, `unwrap()`, `expect()`, `panic!()` in production
- **Error Handling**: Use structured error types (AppError, DatabaseError)
- **Code Quality**: cargo fmt, clippy (deny level), architectural validation
- **Testing Requirements**: Unit + integration + E2E tests mandatory
- **Documentation**: Module-level //! comments, public API /// comments
- **Pre-Commit Hooks**: Format, lint, test, architectural validation

### Appendix C: Pierre Codebase Map
- **File-by-file overview** with line counts
- **Module dependency graph**
- **Key types and traits reference**
- **35+ Tool catalog** with handler locations
- **Configuration reference** (all environment variables)

### Appendix D: Natural Language to Tool Mapping
Examples of user prompts and corresponding tool activations:
- "Show me my activities" → `get_activities`
- "How's my fitness?" → `calculate_fitness_score`
- "Can I run a marathon in 6 months?" → `analyze_goal_feasibility`
- "Analyze my sleep" → `analyze_sleep_quality`
- "What should I eat before my run?" → `get_nutrient_timing`
- "Search for chicken breast nutrition" → `search_food` + `get_food_details`

---

## Summary

**Total**: 25 Chapters across 7 Parts
**Focus**: Rust idioms, real code examples, progressive complexity
**Code Citations**: All examples reference actual Pierre codebase with file:line numbers
**New Additions**: SDK chapter, detailed MCP implementation, A2A protocol, comprehensive tools guide, synthetic data testing, design system

**Coverage**:
- ✅ SDK bridge architecture and type generation (Chapter 13-14)
- ✅ Deep MCP protocol implementation (Chapters 9-12)
- ✅ A2A protocol dedicated chapter (Chapter 18)
- ✅ Comprehensive tools guide with 35+ tools and prompts (Chapter 19)
- ✅ Testing framework with synthetic data and tools-to-types (Chapter 23)
- ✅ Design system with templates and frontend (Chapter 24)
- ✅ Rust idiomatic patterns throughout all chapters
- ✅ CLAUDE.md compliance (no anyhow!, unwrap, expect)
