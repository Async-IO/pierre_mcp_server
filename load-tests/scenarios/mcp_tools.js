// ABOUTME: k6 load test for MCP tool invocation endpoints
// ABOUTME: Tests JSON-RPC 2.0 tool execution throughput and latency
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const toolInvokeDuration = new Trend('tool_invoke_duration');
const listToolsDuration = new Trend('list_tools_duration');
const successfulInvocations = new Counter('successful_invocations');

export const options = {
  stages: [
    { duration: '30s', target: 20 },
    { duration: '2m', target: 50 },
    { duration: '30s', target: 100 },
    { duration: '30s', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(95)<1000', 'p(99)<2000'],
    http_req_failed: ['rate<0.05'],
    tool_invoke_duration: ['p(95)<500'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8081';
const API_KEY = __ENV.API_KEY || 'test_api_key';

// MCP JSON-RPC 2.0 message helper
function createMcpRequest(method, params, id) {
  return JSON.stringify({
    jsonrpc: '2.0',
    method: method,
    params: params,
    id: id || Date.now(),
  });
}

// List of tools to test (read-only operations for safe load testing)
const TOOLS_TO_TEST = [
  { name: 'get_health', params: {} },
  { name: 'list_tools', params: {} },
];

export function setup() {
  // Verify MCP endpoint is available
  const healthResponse = http.get(`${BASE_URL}/health`);
  check(healthResponse, {
    'server is healthy': (r) => r.status === 200,
  });

  return { tools: TOOLS_TO_TEST };
}

export default function (data) {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${API_KEY}`,
  };

  group('List Tools', function () {
    const startTime = Date.now();

    const listRequest = createMcpRequest('tools/list', {});
    const response = http.post(
      `${BASE_URL}/mcp`,
      listRequest,
      {
        headers: headers,
        tags: { name: 'mcp_list_tools' },
      }
    );

    listToolsDuration.add(Date.now() - startTime);

    const success = check(response, {
      'list tools status ok': (r) => r.status === 200,
      'valid JSON-RPC response': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.jsonrpc === '2.0';
        } catch {
          return false;
        }
      },
      'has tools array': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.result && Array.isArray(body.result.tools);
        } catch {
          return false;
        }
      },
    });

    if (!success) {
      errorRate.add(1);
    }
  });

  group('Tool Invocations', function () {
    // Test each tool
    for (const tool of data.tools) {
      const startTime = Date.now();

      const toolRequest = createMcpRequest('tools/call', {
        name: tool.name,
        arguments: tool.params,
      });

      const response = http.post(
        `${BASE_URL}/mcp`,
        toolRequest,
        {
          headers: headers,
          tags: { name: `mcp_tool_${tool.name}` },
        }
      );

      toolInvokeDuration.add(Date.now() - startTime);

      const success = check(response, {
        [`${tool.name} status ok`]: (r) => r.status === 200,
        [`${tool.name} valid response`]: (r) => {
          try {
            const body = JSON.parse(r.body);
            return body.jsonrpc === '2.0' && (body.result !== undefined || body.error !== undefined);
          } catch {
            return false;
          }
        },
      });

      if (success) {
        successfulInvocations.add(1);
      } else {
        errorRate.add(1);
      }

      sleep(0.1); // Small pause between tool calls
    }
  });

  // Simulate batch tool calls (common MCP pattern)
  group('Batch Tool Calls', function () {
    const batchRequests = data.tools.map((tool, idx) =>
      createMcpRequest('tools/call', {
        name: tool.name,
        arguments: tool.params,
      }, idx + 1)
    );

    const startTime = Date.now();

    // Send requests in parallel
    const responses = http.batch(
      batchRequests.map((req, idx) => ({
        method: 'POST',
        url: `${BASE_URL}/mcp`,
        body: req,
        params: {
          headers: headers,
          tags: { name: 'mcp_batch' },
        },
      }))
    );

    toolInvokeDuration.add(Date.now() - startTime);

    for (const response of responses) {
      const success = check(response, {
        'batch request ok': (r) => r.status === 200,
      });
      if (success) {
        successfulInvocations.add(1);
      }
    }
  });

  sleep(0.5); // Pause between iterations
}

export function handleSummary(data) {
  return {
    'stdout': JSON.stringify(data, null, 2),
    'load-tests/results/mcp_tools_summary.json': JSON.stringify(data, null, 2),
  };
}
