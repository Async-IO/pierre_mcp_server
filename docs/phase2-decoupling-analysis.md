# Phase 2: Framework Decoupling Analysis

## Architecture Understanding

Pierre MCP Server is transitioning to a three-repository architecture:

1. **pierre_mcp_server** → **pierre-framework** (Generic MCP/A2A framework, open-source)
2. **pierre-fitness-app** (Proprietary fitness intelligence, depends on framework)
3. **pierre-fitness-providers** (Proprietary provider implementations, used by app)

**Critical Constraint:** Framework CANNOT import from fitness-app (would create circular dependency)

## Current State

### Fitness-Specific Code in Framework (To Remove)

1. **src/intelligence/** (22 files) - Fitness algorithms and analysis
   - Imported by: 27 files including handlers, configuration, tools
   - Status: Duplicated in pierre-fitness-app/src/intelligence/

2. **src/models.rs** - Fitness data models (Activity, Athlete, etc.)
   - Imported by: 40+ files
   - Status: Duplicated in pierre-fitness-app/src/models.rs

3. **src/configuration/** - Fitness-specific configuration
   - vo2_max.rs (352 lines) - VO2 max calculations
   - profiles.rs - Fitness level profiles
   - validation.rs - Uses intelligence algorithms
   - Status: Partially fitness-specific

4. **src/protocols/universal/handlers/** - Fitness-specific tool handlers
   - fitness_api.rs, goals.rs, intelligence.rs, nutrition.rs, sleep_recovery.rs
   - Status: Likely duplicated in pierre-fitness-app/src/handlers/

5. **HTTP Routes** - Fitness-specific API endpoints
   - configuration_routes.rs - VO2 max zone calculations
   - fitness_configuration_routes.rs - Fitness config management
   - dashboard_routes.rs - Fitness dashboard data

### Generic Framework Code (To Keep)

1. **src/mcp/** - MCP protocol implementation
2. **src/a2a/** - A2A protocol implementation
3. **src/jsonrpc/** - JSON-RPC foundation
4. **src/database_plugins/** - Database abstraction
5. **src/cache/** - Cache abstraction
6. **src/auth/** - Authentication system
7. **src/oauth2_server/** - OAuth2 provider functionality
8. **src/oauth2_client/** - OAuth2 client functionality
9. **src/providers/core.rs** - Generic provider traits
10. **src/providers/registry.rs** - Provider registration system
11. **src/providers/spi.rs** - Service Provider Interface

## Decoupling Dependencies

### Circular Dependency Challenges

Several files create circular dependency problems:
- configuration_routes.rs imports from src/configuration/vo2_max.rs
- But vo2_max.rs is fitness-specific and should move to pierre-fitness-app
- Framework routes can't import from fitness-app

### Solution Approach

**Option A: Move routes to pierre-fitness-app**
- Move configuration_routes.rs to pierre-fitness-app
- Register routes from fitness-app at framework startup
- Keeps clear separation

**Option B: Generic route handlers with callbacks**
- Make routes generic, accept callbacks for domain logic
- Fitness-app provides implementations
- More complex but cleaner separation

## Phase 2 Incremental Steps

1. **Document current coupling** ✅ (this document)
2. **Identify circular dependencies** ✅ (configuration routes → vo2_max)
3. **Plan removal strategy** - Next session: Remove one isolated fitness file
4. **Extract generic patterns** - Separate framework patterns from fitness content
5. **Move fitness handlers** - Update handler registration
6. **Remove duplicated modules** - After all imports updated

## Recommendations for Next Session

The decoupling is deeply entangled and requires careful planning. Recommendations:

1. **Start with handlers**: Move `src/protocols/universal/handlers/fitness_api.rs` and related handlers to pierre-fitness-app first
2. **Update tool registry**: Make tool registration dynamic so fitness-app can register its tools
3. **Extract configuration framework**: Keep generic ConfigCatalog pattern, move fitness parameters
4. **Gradual removal**: Remove one module at a time, updating all imports

## Blocked Items

Cannot proceed with removing src/intelligence or src/models until:
- All handlers moved to pierre-fitness-app
- All routes moved to pierre-fitness-app
- Tool registration system supports external registration
- Configuration system separated into generic framework + fitness content
