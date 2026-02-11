#!/usr/bin/env bun
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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
// Output to shared mcp-types package (SDK re-exports from there)
const OUTPUT_DIR = path.join(__dirname, '../packages/mcp-types/src');
const OUTPUT_TOOLS_FILE = path.join(OUTPUT_DIR, 'tools.ts');
const OUTPUT_COMMON_FILE = path.join(OUTPUT_DIR, 'common.ts');
const JWT_TOKEN = process.env.PIERRE_JWT_TOKEN || null;

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

/**
 * Generate inline interface from properties
 */
function generateInterfaceFromProperties(properties, requiredFields = []) {
  if (!properties || Object.keys(properties).length === 0) {
    return '{}';
  }

  const fields = Object.entries(properties).map(([name, prop]) => {
    const isRequired = requiredFields.includes(name);
    const tsType = jsonSchemaToTypeScript(prop, name, isRequired);
    const optional = isRequired ? '' : '?';
    const description = prop.description ? `\n  /** ${prop.description} */` : '';
    return `${description}\n  ${name}${optional}: ${tsType};`;
  });

  return `{\n${fields.join('\n')}\n}`;
}

/**
 * Convert tool name to PascalCase for interface names
 */
function toPascalCase(str) {
  return str
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join('');
}

/**
 * Generate TypeScript tool types from tool schemas
 */
function generateToolsTypeScript(tools) {
  // Sort tools alphabetically by name for deterministic output
  const sortedTools = [...tools].sort((a, b) => a.name.localeCompare(b.name));

  const header = `// ABOUTME: Auto-generated TypeScript type definitions for Pierre MCP tool parameters
// ABOUTME: Generated from server tool schemas - DO NOT EDIT MANUALLY
//
// Generated: ${new Date().toISOString()}
// Tool count: ${sortedTools.length}
// To regenerate: bun run generate (from packages/mcp-types)

/* eslint-disable @typescript-eslint/no-explicit-any */

// ============================================================================
// TOOL PARAMETER TYPES
// ============================================================================

// Note: connect_to_pierre removed - SDK bridge handles authentication locally via RFC 8414 discovery

`;

  const paramTypes = sortedTools.map(tool => {
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

  const responseTypesHeader = `
// ============================================================================
// TOOL RESPONSE TYPES
// ============================================================================

/**
 * Generic MCP tool response wrapper
 */
export interface McpToolResponse {
  content?: Array<{
    type: string;
    text?: string;
    [key: string]: any;
  }>;
  isError?: boolean;
  [key: string]: any;
}

/**
 * MCP error response
 */
export interface McpErrorResponse {
  code: number;
  message: string;
  data?: any;
}

`;

  // Generate a union type of all tool names (already sorted)
  const toolNamesUnion = `
// ============================================================================
// TOOL NAME TYPES
// ============================================================================

/**
 * Union type of all available tool names
 */
export type ToolName = ${sortedTools.map(t => `"${t.name}"`).join(' | ')};

/**
 * Map of tool names to their parameter types
 */
export interface ToolParamsMap {
${sortedTools.map(t => `  "${t.name}": ${toPascalCase(t.name)}Params;`).join('\n')}
}
`;

  return header + paramTypes + responseTypesHeader + toolNamesUnion;
}

/**
 * Generate common data types (Activity, Athlete, etc.)
 */
function generateCommonTypeScript() {
  return `// ABOUTME: Common data types for Pierre MCP tools (Activity, Athlete, Stats, etc.)
// ABOUTME: Generated from server tool schemas - DO NOT EDIT MANUALLY
//
// Generated: ${new Date().toISOString()}

/* eslint-disable @typescript-eslint/no-explicit-any */

// ============================================================================
// COMMON DATA TYPES
// ============================================================================

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

/**
 * Athlete profile data structure
 */
export interface Athlete {
  id: string;
  username?: string;
  resource_state?: number;
  firstname?: string;
  lastname?: string;
  bio?: string;
  city?: string;
  state?: string;
  country?: string;
  sex?: string;
  premium?: boolean;
  summit?: boolean;
  created_at?: string;
  updated_at?: string;
  badge_type_id?: number;
  weight?: number;
  profile_medium?: string;
  profile?: string;
  friend?: any;
  follower?: any;
  ftp?: number;
  [key: string]: any;
}

/**
 * Athlete statistics data structure
 */
export interface Stats {
  biggest_ride_distance?: number;
  biggest_climb_elevation_gain?: number;
  recent_ride_totals?: ActivityTotals;
  recent_run_totals?: ActivityTotals;
  recent_swim_totals?: ActivityTotals;
  ytd_ride_totals?: ActivityTotals;
  ytd_run_totals?: ActivityTotals;
  ytd_swim_totals?: ActivityTotals;
  all_ride_totals?: ActivityTotals;
  all_run_totals?: ActivityTotals;
  all_swim_totals?: ActivityTotals;
  [key: string]: any;
}

/**
 * Activity totals for statistics
 */
export interface ActivityTotals {
  count?: number;
  distance?: number;
  moving_time?: number;
  elapsed_time?: number;
  elevation_gain?: number;
  achievement_count?: number;
}

/**
 * Fitness configuration profile
 */
export interface FitnessConfig {
  athlete_info?: {
    age?: number;
    weight?: number;
    height?: number;
    sex?: string;
    ftp?: number;
    max_heart_rate?: number;
    resting_heart_rate?: number;
    vo2_max?: number;
  };
  training_zones?: {
    heart_rate?: Zone[];
    power?: Zone[];
    pace?: Zone[];
  };
  goals?: Goal[];
  preferences?: {
    distance_unit?: string;
    weight_unit?: string;
    [key: string]: any;
  };
  [key: string]: any;
}

/**
 * Training zone definition
 */
export interface Zone {
  zone: number;
  name: string;
  min: number;
  max: number;
  description?: string;
}

/**
 * Fitness goal definition
 */
export interface Goal {
  id?: string;
  type: string;
  target_value: number;
  target_date: string;
  activity_type?: string;
  description?: string;
  progress?: number;
  status?: string;
  created_at?: string;
  updated_at?: string;
}

/**
 * Provider connection status
 */
export interface ConnectionStatus {
  provider: string;
  connected: boolean;
  last_sync?: string;
  expires_at?: string;
  scopes?: string[];
  [key: string]: any;
}

/**
 * Notification data structure
 */
export interface Notification {
  id: string;
  type: string;
  message: string;
  provider?: string;
  success?: boolean;
  created_at: string;
  read: boolean;
  [key: string]: any;
}
`;
}

/**
 * Main execution
 */
async function main() {
  console.log('🔧 Pierre MCP Types Generator');
  console.log('==============================\n');

  console.log(`📡 Fetching tool schemas from ${SERVER_URL}:${SERVER_PORT}/mcp...`);

  try {
    const tools = await fetchToolSchemas();
    console.log(`✅ Fetched ${tools.length} tool schemas\n`);

    // Ensure output directory exists
    if (!fs.existsSync(OUTPUT_DIR)) {
      fs.mkdirSync(OUTPUT_DIR, { recursive: true });
    }

    console.log('🔨 Generating TypeScript definitions...');

    // Generate and write tools.ts
    const toolsTs = generateToolsTypeScript(tools);
    console.log(`💾 Writing to ${OUTPUT_TOOLS_FILE}...`);
    fs.writeFileSync(OUTPUT_TOOLS_FILE, toolsTs, 'utf8');

    // Generate and write common.ts
    const commonTs = generateCommonTypeScript();
    console.log(`💾 Writing to ${OUTPUT_COMMON_FILE}...`);
    fs.writeFileSync(OUTPUT_COMMON_FILE, commonTs, 'utf8');

    console.log(`\n✅ Successfully generated types for ${tools.length} tools!\n`);
    console.log('📋 Generated files:');
    console.log(`   - ${OUTPUT_TOOLS_FILE} (${tools.length} tool parameter interfaces)`);
    console.log(`   - ${OUTPUT_COMMON_FILE} (common data types)`);

    console.log('\n✨ Type generation complete!');
    console.log(`\n💡 Import types in your code:`);
    console.log(`   import { GetActivitiesParams, Activity } from '@pierre/mcp-types';\n`);

  } catch (error) {
    console.error('❌ Error generating types:', error.message);
    console.error('\n🔍 Troubleshooting:');
    console.error('   1. Ensure Pierre server is running on port', SERVER_PORT);
    console.error('   2. Check if JWT token is valid (set PIERRE_JWT_TOKEN env var)');
    console.error('   3. Verify server is accessible at', `${SERVER_URL}:${SERVER_PORT}/mcp`);
    console.error('\n💡 Start server with: cargo run --bin pierre-mcp-server');
    process.exit(1);
  }
}

// Run if called directly
if (require.main === module) {
  main();
}

module.exports = { fetchToolSchemas, generateToolsTypeScript, generateCommonTypeScript };
