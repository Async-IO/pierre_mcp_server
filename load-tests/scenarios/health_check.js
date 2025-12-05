// ABOUTME: k6 load test for health check endpoint baseline performance
// ABOUTME: Establishes baseline latency metrics for the simplest endpoint
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const healthCheckDuration = new Trend('health_check_duration');

// Default options (can be overridden via CLI or config file)
export const options = {
  stages: [
    { duration: '30s', target: 20 },   // Ramp up to 20 VUs
    { duration: '1m', target: 50 },    // Sustained load at 50 VUs
    { duration: '30s', target: 100 },  // Peak load at 100 VUs
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    // SLA targets
    http_req_duration: ['p(95)<50', 'p(99)<100'],  // 95th < 50ms, 99th < 100ms
    http_req_failed: ['rate<0.01'],                // < 1% error rate
    errors: ['rate<0.01'],
  },
};

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8081';

export default function () {
  const startTime = Date.now();

  const response = http.get(`${BASE_URL}/health`, {
    tags: { name: 'health_check' },
  });

  const duration = Date.now() - startTime;
  healthCheckDuration.add(duration);

  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'has status field': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status !== undefined;
      } catch {
        return false;
      }
    },
    'status is healthy': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status === 'healthy' || body.status === 'ok';
      } catch {
        return false;
      }
    },
    'response time < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(!checkResult);

  // Small pause between requests
  sleep(0.1);
}

export function handleSummary(data) {
  return {
    'stdout': JSON.stringify(data, null, 2),
    'load-tests/results/health_check_summary.json': JSON.stringify(data, null, 2),
  };
}
