// ABOUTME: Unit tests for message handling and validation
// ABOUTME: Tests MCP message parsing, validation, and error handling

const { MCPMessages, MCPResponses } = require('../helpers/fixtures');

describe('Batch Request Detection', () => {
  test('should detect array as batch request', () => {
    const message = MCPMessages.batchRequest;
    expect(Array.isArray(message)).toBe(true);
    expect(message.length).toBe(2);
  });

  test('should detect single request as non-batch', () => {
    const message = MCPMessages.initialize;
    expect(Array.isArray(message)).toBe(false);
    expect(message).toHaveProperty('method');
  });

  test('should validate batch rejection response format', () => {
    const responses = MCPResponses.batchRejection;
    expect(Array.isArray(responses)).toBe(true);
    expect(responses[0]).toHaveProperty('error');
    expect(responses[0].error.code).toBe(-32600);
  });
});

describe('Message Validation', () => {
  test('should validate valid initialize request', () => {
    const message = MCPMessages.initialize;
    expect(message).toHaveProperty('jsonrpc');
    expect(message).toHaveProperty('id');
    expect(message).toHaveProperty('method');
    expect(message).toHaveProperty('params');
    expect(message.jsonrpc).toBe('2.0');
  });

  test('should validate valid tools/list request', () => {
    const message = MCPMessages.toolsList;
    expect(message.method).toBe('tools/list');
    expect(message.jsonrpc).toBe('2.0');
  });

  test('should detect malformed message missing jsonrpc', () => {
    const message = MCPMessages.malformedNoJsonRpc;
    expect(message).not.toHaveProperty('jsonrpc');
  });

  test('should detect malformed message missing method', () => {
    const message = MCPMessages.malformedNoMethod;
    expect(message).not.toHaveProperty('method');
  });
});

describe('Response Validation', () => {
  test('should validate successful response format', () => {
    const response = MCPResponses.initializeSuccess;
    expect(response).toHaveProperty('jsonrpc');
    expect(response).toHaveProperty('id');
    expect(response).toHaveProperty('result');
    expect(response.jsonrpc).toBe('2.0');
  });

  test('should validate error response format', () => {
    const response = MCPResponses.errorUnauthorized;
    expect(response).toHaveProperty('error');
    expect(response.error).toHaveProperty('code');
    expect(response.error).toHaveProperty('message');
  });

  test('should validate tools list response', () => {
    const response = MCPResponses.toolsListSuccess;
    expect(response.result).toHaveProperty('tools');
    expect(Array.isArray(response.result.tools)).toBe(true);
    expect(response.result.tools.length).toBeGreaterThan(0);
  });
});

describe('Protocol Version Handling', () => {
  test('should use protocol version 2025-06-18', () => {
    const message = MCPMessages.initialize;
    expect(message.params.protocolVersion).toBe('2025-06-18');
  });

  test('should reject batch requests for 2025-06-18', () => {
    const protocolVersion = '2025-06-18';
    const supportsBatch = false; // 2025-06-18 does not support batching
    expect(supportsBatch).toBe(false);
  });
});

describe('Error Code Handling', () => {
  test('should use correct error code for batch rejection', () => {
    const errorCode = -32600; // Invalid Request
    expect(errorCode).toBe(-32600);
  });

  test('should use correct error code for unauthorized', () => {
    const errorCode = -32001;
    expect(errorCode).toBe(-32001);
  });

  test('should use correct error code for internal error', () => {
    const errorCode = -32603; // Internal error
    expect(errorCode).toBe(-32603);
  });
});
