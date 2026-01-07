<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 14: Type Generation & Tools-to-Types System

This chapter explores Pierre's automated type generation system that converts Rust tool schemas to TypeScript interfaces, ensuring type safety between the server and SDK. You'll learn about schema-driven development, synthetic data generation for testing, and the complete tools-to-types workflow.

## What You'll Learn

- Automated type generation from server schemas
- JSON Schema to TypeScript conversion
- Schema-driven development workflow
- Synthetic data generation for testing
- Tools-to-types script implementation
- Type safety guarantees across language boundaries
- Deterministic testing patterns
- Builder pattern for test data

## Type Generation Overview

Pierre generates TypeScript types directly from server tool schemas:

```
┌──────────────┐  tools/list   ┌──────────────┐  generate  ┌──────────────┐
│ Rust Tool    │──────────────►│ JSON Schema  │───────────►│ TypeScript   │
│ Definitions  │  (runtime)    │ (runtime)    │  (script)  │ Interfaces   │
└──────────────┘               └──────────────┘            └──────────────┘
    src/mcp/                     inputSchema                sdk/src/types.ts
    schema.rs                    properties

  Single Source of Truth: Rust definitions generate both runtime API and TypeScript types
```

**Key insight**: Tool schemas defined in Rust become the single source of truth for both runtime validation and TypeScript type safety.

## Tools-to-Types Script

The type generator fetches schemas from a running Pierre server and converts them to TypeScript:

**Source**: scripts/generate-sdk-types.js:1-16
```javascript
#!/usr/bin/env node
// ABOUTME: Auto-generates TypeScript type definitions from Pierre server tool schemas
// ABOUTME: Fetches MCP tool schemas and converts them to TypeScript interfaces for SDK usage

const http = require('http');
const fs = require('fs');
const path = require('path');

/**
 * Configuration
 */
const SERVER_URL = process.env.PIERRE_SERVER_URL || 'http://localhost:8081';
const SERVER_PORT = process.env.HTTP_PORT || '8081';
const OUTPUT_FILE = path.join(__dirname, '../sdk/src/types.ts');
const JWT_TOKEN = process.env.PIERRE_JWT_TOKEN || null;
```

**Configuration**:
- `SERVER_URL`: Pierre server endpoint (default: localhost:8081)
- `OUTPUT_FILE`: Generated TypeScript output (sdk/src/types.ts)
- `JWT_TOKEN`: Optional authentication for protected servers

## Fetching Tool Schemas

The script calls `tools/list` to retrieve all tool schemas:

**Source**: scripts/generate-sdk-types.js:20-74
```javascript
/**
 * Fetch tool schemas from Pierre server
 */
async function fetchToolSchemas() {
  return new Promise((resolve, reject) => {
    const requestData = JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/list',
      params: {}
    });

    const options = {
      hostname: 'localhost',
      port: SERVER_PORT,
      path: '/mcp',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(requestData),
        ...(JWT_TOKEN ? { 'Authorization': `Bearer ${JWT_TOKEN}` } : {})
      }
    };

    const req = http.request(options, (res) => {
      let data = '';

      res.on('data', (chunk) => {
        data += chunk;
      });

      res.on('end', () => {
        if (res.statusCode !== 200) {
          reject(new Error(`Server returned ${res.statusCode}: ${data}`));
          return;
        }

        try {
          const parsed = JSON.parse(data);
          if (parsed.error) {
            reject(new Error(`MCP error: ${JSON.stringify(parsed.error)}`));
            return;
          }
          resolve(parsed.result.tools || []);
        } catch (err) {
          reject(new Error(`Failed to parse response: ${err.message}`));
        }
      });
    });

    req.on('error', (err) => {
      reject(new Error(`Failed to connect to server: ${err.message}`));
    });

    req.write(requestData);
    req.end();
  });
}
```

**Fetch flow**:
1. **JSON-RPC request**: POST to `/mcp` with `tools/list` method
2. **Authentication**: Include JWT token if available
3. **Parse response**: Extract `result.tools` array
4. **Error handling**: Validate status code and JSON-RPC errors

## JSON Schema to Typescript Conversion

The core conversion logic maps JSON Schema types to TypeScript:

**Source**: scripts/generate-sdk-types.js:79-127
```javascript
/**
 * Convert JSON schema property to TypeScript type
 */
function jsonSchemaToTypeScript(property, propertyName, required = false) {
  if (!property) {
    return 'any';
  }

  const isOptional = !required;
  const optionalMarker = isOptional ? '?' : '';

  // Handle type arrays (e.g., ["string", "null"])
  if (Array.isArray(property.type)) {
    const types = property.type
      .filter(t => t !== 'null')
      .map(t => jsonSchemaToTypeScript({ type: t }, propertyName, true));
    const typeStr = types.length > 1 ? types.join(' | ') : types[0];
    return property.type.includes('null') ? `${typeStr} | null` : typeStr;
  }

  switch (property.type) {
    case 'string':
      if (property.enum) {
        return property.enum.map(e => `"${e}"`).join(' | ');
      }
      return 'string';
    case 'number':
    case 'integer':
      return 'number';
    case 'boolean':
      return 'boolean';
    case 'array':
      if (property.items) {
        const itemType = jsonSchemaToTypeScript(property.items, propertyName, true);
        return `${itemType}[]`;
      }
      return 'any[]';
    case 'object':
      if (property.properties) {
        return generateInterfaceFromProperties(property.properties, property.required || []);
      }
      if (property.additionalProperties) {
        const valueType = jsonSchemaToTypeScript(property.additionalProperties, propertyName, true);
        return `Record<string, ${valueType}>`;
      }
      return 'Record<string, any>';
    case 'null':
      return 'null';
    default:
      return 'any';
  }
}
```

**Type mapping**:
- `string` → `string` (with enum support for union types)
- `number`/`integer` → `number`
- `boolean` → `boolean`
- `array` → `T[]` (with item type inference)
- `object` → inline interface or `Record<string, T>`
- Union types: `["string", "null"]` → `string | null`

## Typescript Idioms: Union Types and Literal Types

**Union types for enums**:

**Source**: scripts/generate-sdk-types.js:98-100
```javascript
case 'string':
  if (property.enum) {
    return property.enum.map(e => `"${e}"`).join(' | ');
  }
```

**Example generated type**:
```typescript
provider: "strava" | "fitbit" | "garmin"  // from enum in JSON Schema
```

This is **idiomatic TypeScript** - using literal union types instead of `enum` provides better type narrowing and inline values.

## Interface Generation

The script generates named interfaces for each tool's parameters:

**Source**: scripts/generate-sdk-types.js:185-205
```javascript
const paramTypes = tools.map(tool => {
  const interfaceName = `${toPascalCase(tool.name)}Params`;
  const description = tool.description ? `\n/**\n * ${tool.description}\n */` : '';

  if (!tool.inputSchema || !tool.inputSchema.properties || Object.keys(tool.inputSchema.properties).length === 0) {
    return `${description}\nexport interface ${interfaceName} {}\n`;
  }

  const properties = tool.inputSchema.properties;
  const required = tool.inputSchema.required || [];

  const fields = Object.entries(properties).map(([name, prop]) => {
    const isRequired = required.includes(name);
    const tsType = jsonSchemaToTypeScript(prop, name, isRequired);
    const optional = isRequired ? '' : '?';
    const propDescription = prop.description ? `\n  /** ${prop.description} */` : '';
    return `${propDescription}\n  ${name}${optional}: ${tsType};`;
  });

  return `${description}\nexport interface ${interfaceName} {\n${fields.join('\n')}\n}\n`;
}).join('\n');
```

**Generated output example** (sdk/src/types.ts:69-81):
```typescript
/**
 * Get fitness activities from a provider
 */
export interface GetActivitiesParams {

  /** Maximum number of activities to return */
  limit?: number;

  /** Number of activities to skip (for pagination) */
  offset?: number;

  /** Fitness provider name (e.g., 'strava', 'fitbit') */
  provider: string;
}
```

**Naming convention**: `tool_name` → `ToolNameParams` (PascalCase conversion)

## Type-Safe Tool Mapping

The script generates a union type of all tool names and parameter mapping:

**Source**: scripts/generate-sdk-types.js:237-253
```javascript
const toolNamesUnion = `
// ============================================================================
// TOOL NAME TYPES
// ============================================================================

/**
 * Union type of all available tool names
 */
export type ToolName = ${tools.map(t => `"${t.name}"`).join(' | ')};

/**
 * Map of tool names to their parameter types
 */
export interface ToolParamsMap {
${tools.map(t => `  "${t.name}": ${toPascalCase(t.name)}Params;`).join('\n')}
}

`;
```

**Generated output** (sdk/src/types.ts - conceptual):
```typescript
export type ToolName = "get_activities" | "get_athlete" | "get_stats" | /* 42 more... */;

export interface ToolParamsMap {
  "get_activities": GetActivitiesParams;
  "get_athlete": GetAthleteParams;
  "get_stats": GetStatsParams;
  // ... 42 more tools
}
```

**Type safety benefit**: TypeScript can validate tool names and infer correct parameter types at compile time.

## Common Data Types

The generator includes manually-defined domain types for fitness data:

**Source**: scripts/generate-sdk-types.js:265-309
```javascript
/**
 * Fitness activity data structure
 */
export interface Activity {
  id: string;
  name: string;
  type: string;
  distance?: number;
  duration?: number;
  moving_time?: number;
  elapsed_time?: number;
  total_elevation_gain?: number;
  start_date?: string;
  start_date_local?: string;
  timezone?: string;
  average_speed?: number;
  max_speed?: number;
  average_cadence?: number;
  average_heartrate?: number;
  max_heartrate?: number;
  average_watts?: number;
  kilojoules?: number;
  device_watts?: boolean;
  has_heartrate?: boolean;
  calories?: number;
  description?: string;
  trainer?: boolean;
  commute?: boolean;
  manual?: boolean;
  private?: boolean;
  visibility?: string;
  flagged?: boolean;
  gear_id?: string;
  from_accepted_tag?: boolean;
  upload_id?: number;
  external_id?: string;
  achievement_count?: number;
  kudos_count?: number;
  comment_count?: number;
  athlete_count?: number;
  photo_count?: number;
  map?: {
    id?: string;
    summary_polyline?: string;
    polyline?: string;
  };
  [key: string]: any;
}
```

**Design choice**: While tool parameter types are auto-generated, domain types like `Activity`, `Athlete`, and `Stats` are manually maintained for stability and documentation.

## Running Type Generation

Invoke the generator via npm script:

**Source**: sdk/package.json:14
```json
"scripts": {
  "generate-types": "node ../scripts/generate-sdk-types.js"
}
```

**Workflow**:
```bash
# 1. Start Pierre server (required - provides tool schemas)
cargo run --bin pierre-mcp-server

# 2. Generate types from running server
cd sdk
npm run generate-types

# 3. Generated output: sdk/src/types.ts (45+ tool interfaces)
```

**Output example**:
```
🔧 Pierre SDK Type Generator
==============================

📡 Fetching tool schemas from http://localhost:8081/mcp...
✅ Fetched 47 tool schemas

🔨 Generating TypeScript definitions...
💾 Writing to sdk/src/types.ts...
✅ Successfully generated types for 47 tools!

📋 Generated interfaces:
   - ConnectToPierreParams
   - ConnectProviderParams
   - GetActivitiesParams
   ... (42 more)

✨ Type generation complete!

💡 Import types in your code:
   import { GetActivitiesParams, Activity } from './types';
```

## Synthetic Data Generation

Pierre includes a synthetic data generator for testing without OAuth connections:

**Source**: tests/helpers/synthetic_data.rs:11-35
```rust
/// Builder for creating synthetic fitness activity data
///
/// Provides deterministic, reproducible generation of realistic fitness activities
/// for testing intelligence algorithms without requiring real OAuth connections.
///
/// # Examples
///
/// ```
/// use tests::synthetic_data::SyntheticDataBuilder;
/// use chrono::Utc;
///
/// let builder = SyntheticDataBuilder::new(42); // Deterministic seed
/// let activity = builder.generate_run()
///     .duration_minutes(30)
///     .distance_km(5.0)
///     .start_date(Utc::now())
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SyntheticDataBuilder {
    // Reserved for future algorithmic tests requiring seed reproducibility verification
    #[allow(dead_code)]
    seed: u64,
    rng: ChaCha8Rng,
}
```

**Key features**:
- **Deterministic**: Seeded RNG (`ChaCha8Rng`) ensures reproducible test data
- **Builder pattern**: Fluent API for constructing activities
- **Realistic data**: Generates physiologically plausible metrics

## Rust Idioms: Builder Pattern for Test Data

**Source**: tests/helpers/synthetic_data.rs:47-67
```rust
impl SyntheticDataBuilder {
    /// Create new builder with deterministic seed for reproducibility
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Generate a synthetic running activity
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: uses &mut self.rng
    pub fn generate_run(&mut self) -> ActivityBuilder<'_> {
        ActivityBuilder::new(SportType::Run, &mut self.rng)
    }

    /// Generate a synthetic cycling activity
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: uses &mut self.rng
    pub fn generate_ride(&mut self) -> ActivityBuilder<'_> {
        ActivityBuilder::new(SportType::Ride, &mut self.rng)
    }
}
```

**Rust idioms**:
1. **`#[must_use]`**: Ensures builder methods aren't called without using the result
2. **Borrowing `&mut self.rng`**: Shares RNG state across builders without cloning
3. **Clippy pragmas**: Documents why `const fn` isn't applicable (mutable state)

## Training Pattern Generation

The builder generates realistic training patterns for testing intelligence algorithms:

**Source**: tests/helpers/synthetic_data.rs:69-132
```rust
/// Generate a series of activities following a specific pattern
#[must_use]
pub fn generate_pattern(&mut self, pattern: TrainingPattern) -> Vec<Activity> {
    match pattern {
        TrainingPattern::BeginnerRunnerImproving => self.beginner_runner_improving(),
        TrainingPattern::ExperiencedCyclistConsistent => self.experienced_cyclist_consistent(),
        TrainingPattern::Overtraining => self.overtraining_scenario(),
        TrainingPattern::InjuryRecovery => self.injury_recovery(),
    }
}

/// Beginner runner improving 35% over 6 weeks
/// Realistic progression for new runner building fitness
fn beginner_runner_improving(&mut self) -> Vec<Activity> {
    let mut activities = Vec::new();
    let base_date = Utc::now() - Duration::days(42); // 6 weeks ago

    // Week 1-2: 3 runs/week, 20 min @ 6:30/km pace
    for week in 0..2 {
        for run in 0..3 {
            let date = base_date + Duration::days(week * 7 + run * 2);
            let activity = self
                .generate_run()
                .duration_minutes(20)
                .pace_min_per_km(6.5)
                .start_date(date)
                .heart_rate(150, 165)
                .build();
            activities.push(activity);
        }
    }

    // Week 3-4: 4 runs/week, 25 min @ 6:00/km pace (improving)
    for week in 2..4 {
        for run in 0..4 {
            let date = base_date + Duration::days(week * 7 + (run * 2));
            let activity = self
                .generate_run()
                .duration_minutes(25)
                .pace_min_per_km(6.0)
                .start_date(date)
                .heart_rate(145, 160)
                .build();
            activities.push(activity);
        }
    }

    // Week 5-6: 4 runs/week, 30 min @ 5:30/km pace (improved 35%)
    for week in 4..6 {
        for run in 0..4 {
            let date = base_date + Duration::days(week * 7 + (run * 2));
            let activity = self
                .generate_run()
                .duration_minutes(30)
                .pace_min_per_km(5.5)
                .start_date(date)
                .heart_rate(140, 155)
                .build();
            activities.push(activity);
        }
    }

    activities
}
```

**Pattern characteristics**:
- **Realistic progression**: 35% improvement over 6 weeks (physiologically plausible)
- **Gradual adaptation**: Increasing volume (20→25→30 min) and intensity (6.5→6.0→5.5 min/km)
- **Heart rate efficiency**: Lower HR at faster paces indicates improved fitness

## Synthetic Provider for Testing

The synthetic provider implements the `FitnessProvider` trait without OAuth:

**Source**: tests/helpers/synthetic_provider.rs:16-75
```rust
/// Synthetic provider for testing intelligence algorithms without OAuth
///
/// Provides pre-loaded activity data for automated testing, allowing
/// validation of metrics calculations, trend analysis, and predictions
/// without requiring real API connections or OAuth tokens.
///
/// # Thread Safety
///
/// All data access is protected by `RwLock` for thread-safe concurrent access.
/// Multiple tests can safely use the same provider instance.
pub struct SyntheticProvider {
    /// Pre-loaded activities for testing
    activities: Arc<RwLock<Vec<Activity>>>,
    /// Activity lookup by ID for fast access
    activity_index: Arc<RwLock<HashMap<String, Activity>>>,
    /// Provider configuration
    config: ProviderConfig,
}

impl SyntheticProvider {
    /// Create a new synthetic provider with given activities
    #[must_use]
    pub fn with_activities(activities: Vec<Activity>) -> Self {
        // Build activity index for O(1) lookup by ID
        let mut index = HashMap::new();
        for activity in &activities {
            index.insert(activity.id.clone(), activity.clone());
        }

        Self {
            activities: Arc::new(RwLock::new(activities)),
            activity_index: Arc::new(RwLock::new(index)),
            config: ProviderConfig {
                name: "synthetic".to_owned(),
                auth_url: "http://localhost/synthetic/auth".to_owned(),
                token_url: "http://localhost/synthetic/token".to_owned(),
                api_base_url: "http://localhost/synthetic/api".to_owned(),
                revoke_url: None,
                default_scopes: vec!["activity:read_all".to_owned()],
            },
        }
    }

    /// Create an empty provider (no activities)
    #[must_use]
    pub fn new() -> Self {
        Self::with_activities(Vec::new())
    }

    /// Add an activity to the provider dynamically
    pub fn add_activity(&self, activity: Activity) {
        {
            let mut activities = self
                .activities
                .write()
                .expect("Synthetic provider activities RwLock poisoned");

            {
                let mut index = self
                    .activity_index
                    .write()
                    .expect("Synthetic provider index RwLock poisoned");
                index.insert(activity.id.clone(), activity.clone());
            } // Drop index early

            activities.push(activity);
        } // RwLock guards dropped here
    }
}
```

**Design patterns**:
- **`Arc<RwLock<T>>`**: Thread-safe shared ownership with interior mutability
- **Dual indexing**: Vec for ordering + HashMap for O(1) ID lookups
- **Early lock release**: Explicit scopes to drop `RwLock` guards before outer scope

## Rust Idioms: Rwlock Scoping

**Source**: tests/helpers/synthetic_provider.rs:84-101
```rust
pub fn add_activity(&self, activity: Activity) {
    {
        let mut activities = self
            .activities
            .write()
            .expect("Synthetic provider activities RwLock poisoned");

        {
            let mut index = self
                .activity_index
                .write()
                .expect("Synthetic provider index RwLock poisoned");
            index.insert(activity.id.clone(), activity.clone());
        } // Drop index early

        activities.push(activity);
    } // RwLock guards dropped here
}
```

**Idiom**: Nested scopes force early lock release. The inner `index` write lock drops before updating `activities`, preventing unnecessary lock contention.

**Why this matters**: Holding multiple locks simultaneously can cause deadlocks. Explicit scoping ensures locks are released in correct order.

## Type Safety Guarantees

The tools-to-types system provides multiple layers of type safety:

```
┌─────────────────────────────────────────────────────────────┐
│                    TYPE SAFETY LAYERS                        │
├─────────────────────────────────────────────────────────────┤
│ 1. Rust Schema Definitions (compile-time)                   │
│    - ToolSchema struct enforces valid JSON Schema           │
│    - Serde validates serialization correctness               │
├─────────────────────────────────────────────────────────────┤
│ 2. JSON-RPC Runtime Validation                              │
│    - Server validates arguments against inputSchema          │
│    - Invalid params return -32602 error code                 │
├─────────────────────────────────────────────────────────────┤
│ 3. TypeScript Interface Generation (build-time)             │
│    - Generated types match server schemas exactly            │
│    - TypeScript compiler validates SDK usage                 │
├─────────────────────────────────────────────────────────────┤
│ 4. Synthetic Testing (test-time)                            │
│    - Deterministic data validates algorithm correctness      │
│    - No OAuth dependencies for unit tests                    │
└─────────────────────────────────────────────────────────────┘
```

## Schema-Driven Development Workflow

The complete workflow ensures server and client stay synchronized:

```
┌────────────────────────────────────────────────────────────┐
│                  SCHEMA-DRIVEN WORKFLOW                     │
└────────────────────────────────────────────────────────────┘

1. Define tool in Rust (src/mcp/schema.rs)
   ↓
   pub fn create_get_activities_tool() -> ToolSchema { ... }

2. Add to tool registry (src/mcp/schema.rs)
   ↓
   pub fn get_tools() -> Vec<ToolSchema> {
       vec![create_get_activities_tool(), ...]
   }

3. Start Pierre server
   ↓
   cargo run --bin pierre-mcp-server

4. Generate TypeScript types
   ↓
   cd sdk && npm run generate-types

5. TypeScript SDK uses generated types
   ↓
   import { GetActivitiesParams } from './types';
   const params: GetActivitiesParams = { provider: "strava", limit: 10 };

6. Compile-time type checking
   ↓
   // TypeScript compiler validates:
   // ✅ provider is required
   // ✅ limit is optional number
   // ❌ invalid_field causes compile error
```

**Key benefit**: Changes to Rust tool schemas automatically propagate to TypeScript SDK after regeneration.

## Testing with Synthetic Data

Combine synthetic data with the provider for comprehensive tests:

**Conceptual usage** (from tests/intelligence_synthetic_helpers_test.rs):
```rust
#[tokio::test]
async fn test_beginner_progression_detection() {
    // Generate realistic training data
    let mut builder = SyntheticDataBuilder::new(42);
    let activities = builder.generate_pattern(TrainingPattern::BeginnerRunnerImproving);

    // Load into synthetic provider
    let provider = SyntheticProvider::with_activities(activities);

    // Test intelligence algorithms without OAuth
    let result = provider.get_activities(Some(50), None).await.unwrap();

    // Verify progression pattern detected
    assert_eq!(result.items.len(), 24); // 6 weeks * 4 runs/week
    // ... validate metrics, trends, etc.
}
```

**Testing benefits**:
- **No OAuth**: Tests run without network or external APIs
- **Deterministic**: Seeded RNG ensures reproducible results
- **Realistic**: Patterns match real-world training data
- **Fast**: In-memory provider, no database required

## Key Takeaways

1. **Single source of truth**: Rust tool schemas generate both runtime validation and TypeScript types.

2. **Automated workflow**: `npm run generate-types` fetches schemas from running server and generates interfaces.

3. **JSON Schema to TypeScript**: Script maps JSON Schema types to idiomatic TypeScript (union types, optional properties, generics).

4. **Type-safe tooling**: Generated `ToolParamsMap` enables compile-time validation of tool calls.

5. **Synthetic data**: Deterministic builder pattern generates realistic fitness data for testing without OAuth.

6. **Builder pattern**: Fluent API with `#[must_use]` prevents common test setup errors.

7. **Thread-safe testing**: Synthetic provider uses `Arc<RwLock<T>>` for concurrent test access.

8. **Schema-driven development**: Changes to server tools automatically flow to SDK after regeneration.

9. **Training patterns**: Pre-built scenarios (beginner progression, overtraining, injury recovery) test intelligence algorithms.

10. **Type safety layers**: Compile-time (Rust + TypeScript), runtime (JSON-RPC validation), and test-time (synthetic data) guarantee correctness.

---

**End of Part IV: SDK & Type System**

You've completed the SDK and type system implementation. You now understand:
- SDK bridge architecture (Chapter 13)
- Automated type generation from server schemas (Chapter 14)

**Next Chapter**: [Chapter 15: OAuth 2.0 Server Implementation](./chapter-15-oauth-server.md) - Begin Part V by learning how Pierre implements OAuth 2.0 server functionality for fitness provider authentication.
