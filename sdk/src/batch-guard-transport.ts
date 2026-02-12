// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Batch request handler for MCP stdio transport
// ABOUTME: Intercepts JSON-RPC batch requests and handles them appropriately for MCP protocol

import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

/**
 * Installs a batch request guard on a StdioServerTransport.
 * 
 * The MCP SDK's JSONRPCMessageSchema does not support arrays (batch requests),
 * so batch requests are rejected during deserialization. This guard intercepts
 * raw buffer processing to handle batch requests at the protocol level.
 * 
 * @param transport The StdioServerTransport to patch
 * @param log Logging function for debugging
 */
export function installBatchGuard(
  transport: StdioServerTransport,
  log: (message: string, ...args: any[]) => void,
): void {
  // Access private internals via any cast
  const transportAny = transport as any;
  const originalProcessReadBuffer = transportAny.processReadBuffer.bind(transport);

  transportAny.processReadBuffer = function (this: any) {
    const readBuffer = this._readBuffer;
    if (!readBuffer || !readBuffer._buffer) {
      return;
    }

    // Check for newline
    const index = readBuffer._buffer.indexOf("\n");
    if (index === -1) {
      return;
    }

    // Extract the line
    const line = readBuffer._buffer
      .toString("utf8", 0, index)
      .replace(/\r$/, "");
    readBuffer._buffer = readBuffer._buffer.subarray(index + 1);

    try {
      const parsed = JSON.parse(line);

      // Handle batch requests specially (arrays)
      if (Array.isArray(parsed)) {
        // Trigger our onmessage handler directly with the array
        if (this.onmessage) {
          this.onmessage(parsed);
        }
        return;
      }

      // For non-batch messages, use the original processing
      // Put the line back in the buffer for normal processing
      readBuffer._buffer = Buffer.concat([
        Buffer.from(line + "\n"),
        readBuffer._buffer,
      ]);
      originalProcessReadBuffer();
    } catch (_error) {
      // JSON parse error - let original handler deal with it
      readBuffer._buffer = Buffer.concat([
        Buffer.from(line + "\n"),
        readBuffer._buffer,
      ]);
      originalProcessReadBuffer();
    }
  };

  log("Batch request guard installed on transport");
}

/**
 * Creates a message handler wrapper that intercepts batch requests.
 * 
 * This wrapper processes incoming MCP messages and:
 * - Rejects batch requests with appropriate JSON-RPC errors (per 2025-06-18 spec)
 * - Handles server/info requests
 * - Forwards other messages to the original handler
 * 
 * @param transport The transport for sending responses
 * @param originalOnMessage The original message handler from MCP Server
 * @param log Logging function
 * @returns A wrapped message handler
 */
export function createBatchGuardMessageHandler(
  transport: StdioServerTransport,
  originalOnMessage: ((message: any) => void) | undefined,
  log: (message: string, ...args: any[]) => void,
): (message: any) => void {
  return (message: any) => {
    // Log message details for debugging
    const messageMethod = message?.method || "unknown";
    const messageId =
      message?.id !== undefined ? `id: ${message.id}` : "notification";
    const messagePreview = Array.isArray(message)
      ? `batch[${message.length}]`
      : messageMethod;
    log(`Received MCP message: ${messagePreview} (${messageId})`);

    // Handle server/info requests
    if (message.method === "server/info" && message.id !== undefined) {
      log(`Handling server/info request with ID: ${message.id}`);
      const response = {
        jsonrpc: "2.0" as const,
        id: message.id,
        result: {
          name: "pierre-mcp-client",
          version: "1.0.0",
          protocolVersion: "2025-06-18",
          supportedVersions: ["2024-11-05", "2025-03-26", "2025-06-18"],
          capabilities: {
            tools: {},
            resources: {},
            prompts: {},
            logging: {},
          },
        },
      };
      transport.send(response).catch((err: any) => {
        log(`Failed to send server/info response: ${err.message}`);
      });
      return;
    }

    // Handle JSON-RPC batch requests (should be rejected in 2025-06-18)
    // Batch requests come as arrays at the JSON level, not after parsing
    if (Array.isArray(message)) {
      log(
        `Rejecting JSON-RPC batch request (${message.length} requests, not supported in 2025-06-18)`,
      );
      log(
        `Batch request IDs: ${message.map((r: any) => r.id).join(", ")}`,
      );

      // For batch requests, the validator expects a JSON array response on a SINGLE line
      // Each request in the batch gets an individual error response
      const responses = message.map((req: any) => ({
        jsonrpc: "2.0" as const,
        id: req.id,
        error: {
          code: -32600,
          message:
            "Batch requests are not supported in protocol version 2025-06-18",
        },
      }));

      log(
        `Sending batch response array with ${responses.length} responses`,
      );
      log(
        `Response structure: ${JSON.stringify(responses).substring(0, 200)}...`,
      );

      // The MCP SDK's send() method serializes objects/arrays and adds newline
      // For batch responses, we need to send the array itself, not individual items
      // Cast to any to bypass TypeScript type checking for the array
      transport.send(responses as any)
        .then(() => {
          log(`Batch response sent successfully`);
        })
        .catch((err: any) => {
          log(`Failed to send batch rejection response: ${err.message}`);
        });
      return;
    }

    // Handle client/log notifications gracefully
    if (message.method === "client/log" && message.id === undefined) {
      log(
        `Client log [${message.params?.level}]: ${message.params?.message}`,
      );
      return;
    }

    // Protect against malformed messages that crash the server
    try {
      // Forward other messages to the MCP Server handler
      if (originalOnMessage) {
        originalOnMessage(message);
      }
    } catch (error: any) {
      log(`Error handling message: ${error.message}`);
      // Send error response if message had an ID
      if (message.id !== undefined) {
        const errorResponse = {
          jsonrpc: "2.0" as const,
          id: message.id,
          error: {
            code: -32603,
            message: `Internal error: ${error.message}`,
          },
        };
        transport.send(errorResponse);
      }
    }
  };
}
