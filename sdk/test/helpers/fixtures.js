// ABOUTME: Test fixtures with sample MCP protocol messages and responses
// ABOUTME: Reusable test data for validating bridge MCP protocol compliance
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/**
 * Sample MCP protocol messages
 */
const MCPMessages = {
  initialize: {
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2025-06-18',
      capabilities: {},
      clientInfo: {
        name: 'test-client',
        version: '1.0.0'
      }
    }
  },

  toolsList: {
    jsonrpc: '2.0',
    id: 2,
    method: 'tools/list',
    params: {}
  },

  toolsCall: {
    jsonrpc: '2.0',
    id: 3,
    method: 'tools/call',
    params: {
      name: 'get_athlete',
      arguments: {}
    }
  },

  resourcesList: {
    jsonrpc: '2.0',
    id: 4,
    method: 'resources/list',
    params: {}
  },

  promptsList: {
    jsonrpc: '2.0',
    id: 5,
    method: 'prompts/list',
    params: {}
  },

  ping: {
    jsonrpc: '2.0',
    id: 6,
    method: 'ping',
    params: {}
  },

  batchRequest: [
    {
      jsonrpc: '2.0',
      id: 'batch_1',
      method: 'tools/list',
      params: {}
    },
    {
      jsonrpc: '2.0',
      id: 'batch_2',
      method: 'resources/list',
      params: {}
    }
  ],

  malformedNoJsonRpc: {
    id: 1,
    method: 'tools/list'
  },

  malformedNoMethod: {
    jsonrpc: '2.0',
    id: 1
  }
};

/**
 * Sample responses
 */
const MCPResponses = {
  initializeSuccess: {
    jsonrpc: '2.0',
    id: 1,
    result: {
      protocolVersion: '2025-06-18',
      capabilities: {
        tools: {},
        resources: {},
        prompts: {},
        logging: {}
      },
      serverInfo: {
        name: 'pierre-fitness',
        version: '1.0.0'
      }
    }
  },

  toolsListSuccess: {
    jsonrpc: '2.0',
    id: 2,
    result: {
      tools: [
        {
          name: 'get_activities',
          description: 'Get user activities from fitness providers',
          inputSchema: {
            type: 'object',
            properties: {}
          }
        },
        {
          name: 'get_athlete',
          description: 'Get athlete profile information',
          inputSchema: {
            type: 'object',
            properties: {}
          }
        }
      ]
    }
  },

  batchRejection: [
    {
      jsonrpc: '2.0',
      id: 'batch_1',
      error: {
        code: -32600,
        message: 'Batch requests are not supported in protocol version 2025-06-18'
      }
    },
    {
      jsonrpc: '2.0',
      id: 'batch_2',
      error: {
        code: -32600,
        message: 'Batch requests are not supported in protocol version 2025-06-18'
      }
    }
  ],

  errorUnauthorized: {
    jsonrpc: '2.0',
    id: 3,
    error: {
      code: -32001,
      message: 'Unauthorized'
    }
  }
};

/**
 * Test configuration
 */
const TestConfig = {
  defaultServerPort: 8888,
  defaultServerUrl: 'http://localhost:8888',
  testEncryptionKey: 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo=',
  testDatabase: 'sqlite::memory:',
  healthCheckTimeout: 30000,
  requestTimeout: 30000
};

module.exports = {
  MCPMessages,
  MCPResponses,
  TestConfig
};
