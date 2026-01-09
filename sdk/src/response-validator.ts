// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Runtime response validation for MCP tool calls
// ABOUTME: Validates tool responses against Zod schemas for type safety

import {
  validateToolResponse,
  hasResponseSchema,
  type ToolName,
  type ValidationResult,
} from "./response-schemas.js";

/**
 * MCP tool response structure from the SDK
 */
interface McpToolResult {
  content?: Array<{
    type: string;
    text?: string;
    [key: string]: unknown;
  }>;
  isError?: boolean;
  [key: string]: unknown;
}

/**
 * Configuration for response validation behavior
 */
export interface ResponseValidatorConfig {
  /**
   * Whether validation is enabled (default: true in development, false in production)
   */
  enabled: boolean;

  /**
   * Whether to throw on validation errors (default: false - just log warnings)
   */
  strict: boolean;

  /**
   * Custom logger function (default: console.warn)
   */
  logger?: (message: string, details?: unknown) => void;

  /**
   * Whether to include raw response data in error logs (default: false for privacy)
   */
  logRawData: boolean;
}

const defaultConfig: ResponseValidatorConfig = {
  enabled: process.env.NODE_ENV !== "production",
  strict: false,
  logRawData: false,
};

let globalConfig: ResponseValidatorConfig = { ...defaultConfig };

/**
 * Configure the response validator
 */
export function configureValidator(config: Partial<ResponseValidatorConfig>): void {
  globalConfig = { ...globalConfig, ...config };
}

/**
 * Get the current validator configuration
 */
export function getValidatorConfig(): Readonly<ResponseValidatorConfig> {
  return { ...globalConfig };
}

/**
 * Extract the actual response data from MCP tool result.
 * MCP responses wrap data in content[].text as JSON string.
 */
function extractResponseData(result: McpToolResult): unknown {
  if (result.isError) {
    // Don't validate error responses - they have a different structure
    return null;
  }

  if (!result.content || result.content.length === 0) {
    return {};
  }

  // Find the first text content item
  const textContent = result.content.find(c => c.type === "text" && c.text);

  if (!textContent?.text) {
    // No text content - might be resource or other type
    return result;
  }

  // Try to parse the text as JSON (most responses are JSON in text field)
  try {
    return JSON.parse(textContent.text);
  } catch {
    // Not JSON - return as-is
    return { text: textContent.text };
  }
}

/**
 * Validation result with the original and validated data
 */
export interface ValidatedToolResult<T = unknown> {
  /** The original MCP result (unchanged) */
  result: McpToolResult;

  /** Whether validation passed */
  valid: boolean;

  /** The parsed and validated response data (if valid) */
  data?: T;

  /** Validation errors (if invalid) */
  errors?: string[];

  /** Tool name that was called */
  toolName: string;
}

/**
 * Validate an MCP tool response against its Zod schema.
 *
 * This function:
 * 1. Extracts the response data from the MCP content wrapper
 * 2. Validates against the tool's Zod schema
 * 3. Returns the result with validation status
 *
 * @param toolName - The name of the tool that was called
 * @param result - The MCP tool result from callTool()
 * @returns ValidatedToolResult with validation status and parsed data
 */
export function validateMcpToolResponse<T = unknown>(
  toolName: string,
  result: McpToolResult
): ValidatedToolResult<T> {
  const log = globalConfig.logger ?? console.warn;

  // If validation is disabled, return result as-is
  if (!globalConfig.enabled) {
    return {
      result,
      valid: true,
      toolName,
    };
  }

  // Check if we have a schema for this tool
  if (!hasResponseSchema(toolName)) {
    // No schema defined - can't validate, but don't fail
    if (process.env.NODE_ENV !== "production") {
      log(`[ResponseValidator] No schema defined for tool: ${toolName}`);
    }
    return {
      result,
      valid: true, // Pass by default when no schema
      toolName,
    };
  }

  // Extract response data from MCP wrapper
  const responseData = extractResponseData(result);

  // Skip validation for error responses
  if (responseData === null) {
    return {
      result,
      valid: true,
      toolName,
    };
  }

  // Validate against schema
  const validation = validateToolResponse(toolName as ToolName, responseData);

  if (validation.success) {
    return {
      result,
      valid: true,
      data: validation.data as T,
      toolName,
    };
  }

  // Validation failed
  const errorMessages = validation.error!.issues.map(
    issue => `${issue.path.join(".")}: ${issue.message}`
  );

  const errorSummary = `[ResponseValidator] Tool "${toolName}" response validation failed:\n  - ${errorMessages.join("\n  - ")}`;

  if (globalConfig.strict) {
    throw new Error(errorSummary);
  }

  // Log warning (non-strict mode)
  log(errorSummary, globalConfig.logRawData ? responseData : undefined);

  return {
    result,
    valid: false,
    errors: errorMessages,
    toolName,
  };
}

/**
 * Create a validation wrapper for tool calls.
 * Use this to wrap callTool() for automatic validation.
 *
 * @example
 * ```typescript
 * const validatedCall = createValidatedToolCall(client.callTool.bind(client));
 * const result = await validatedCall("get_activities", { provider: "strava" });
 * if (result.valid) {
 *   console.log(result.data.activities);
 * }
 * ```
 */
export function createValidatedToolCall(
  callTool: (params: { name: string; arguments?: Record<string, unknown> }) => Promise<McpToolResult>
): <T = unknown>(
  toolName: string,
  args?: Record<string, unknown>
) => Promise<ValidatedToolResult<T>> {
  return async <T = unknown>(
    toolName: string,
    args?: Record<string, unknown>
  ): Promise<ValidatedToolResult<T>> => {
    const result = await callTool({ name: toolName, arguments: args });
    return validateMcpToolResponse<T>(toolName, result);
  };
}

/**
 * Type guard to check if validation passed
 */
export function isValidResponse<T>(
  result: ValidatedToolResult<T>
): result is ValidatedToolResult<T> & { valid: true; data: T } {
  return result.valid && result.data !== undefined;
}

/**
 * Statistics about validation results (for monitoring/debugging)
 */
export interface ValidationStats {
  totalCalls: number;
  validResponses: number;
  invalidResponses: number;
  skippedResponses: number;
  errorsByTool: Record<string, number>;
}

let stats: ValidationStats = {
  totalCalls: 0,
  validResponses: 0,
  invalidResponses: 0,
  skippedResponses: 0,
  errorsByTool: {},
};

/**
 * Get validation statistics
 */
export function getValidationStats(): Readonly<ValidationStats> {
  return { ...stats };
}

/**
 * Reset validation statistics
 */
export function resetValidationStats(): void {
  stats = {
    totalCalls: 0,
    validResponses: 0,
    invalidResponses: 0,
    skippedResponses: 0,
    errorsByTool: {},
  };
}

/**
 * Validate with statistics tracking
 */
export function validateWithStats<T = unknown>(
  toolName: string,
  result: McpToolResult
): ValidatedToolResult<T> {
  stats.totalCalls++;

  const validated = validateMcpToolResponse<T>(toolName, result);

  if (!globalConfig.enabled || !hasResponseSchema(toolName)) {
    stats.skippedResponses++;
  } else if (validated.valid) {
    stats.validResponses++;
  } else {
    stats.invalidResponses++;
    stats.errorsByTool[toolName] = (stats.errorsByTool[toolName] || 0) + 1;
  }

  return validated;
}
