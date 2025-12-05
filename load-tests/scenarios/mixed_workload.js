// ABOUTME: k6 load test simulating realistic mixed traffic patterns
// ABOUTME: Combines health checks, auth, and API calls with weighted distribution
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';

// Custom metrics
const errorRate = new Rate('errors');
const healthDuration = new Trend('health_duration');
const apiDuration = new Trend('api_duration');
const requestsTotal = new Counter('requests_total');

// Realistic traffic distribution:
// 60% health checks (monitoring systems)
// 25% API calls (applications)
// 15% auth operations (user sessions)
const TRAFFIC_DISTRIBUTION = {
  health: 60,
  api: 25,
  auth: 15,
};

export const options = {
  scenarios: {
    // Steady state traffic
    steady_traffic: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '1m', target: 30 },
        { duration: '3m', target: 50 },
        { duration: '1m', target: 30 },
        { duration: '30s', target: 0 },
      ],
      gracefulRampDown: '30s',
    },
    // Spike traffic (simulating traffic bursts)
    spike_traffic: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 5 },
        { duration: '10s', target: 100 },  // Spike
        { duration: '30s', target: 5 },
        { duration: '10s', target: 100 },  // Another spike
        { duration: '30s', target: 0 },
      ],
      startTime: '1m',  // Start after steady traffic begins
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.05'],
    errors: ['rate<0.05'],
    health_duration: ['p(95)<50'],
    api_duration: ['p(95)<500'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8081';
const API_KEY = __ENV.API_KEY || 'test_api_key';

function performHealthCheck() {
  const startTime = Date.now();

  const response = http.get(`${BASE_URL}/health`, {
    tags: { name: 'health', type: 'monitoring' },
  });

  healthDuration.add(Date.now() - startTime);
  requestsTotal.add(1);

  const success = check(response, {
    'health check ok': (r) => r.status === 200,
  });

  if (!success) errorRate.add(1);
  return success;
}

function performApiCall() {
  const startTime = Date.now();

  // Simulate various API endpoints
  const endpoints = [
    '/api/v1/status',
    '/api/v1/info',
  ];

  const endpoint = endpoints[randomIntBetween(0, endpoints.length - 1)];

  const response = http.get(`${BASE_URL}${endpoint}`, {
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
    },
    tags: { name: 'api_call', type: 'application' },
  });

  apiDuration.add(Date.now() - startTime);
  requestsTotal.add(1);

  // Accept 200, 401 (no auth), or 404 (endpoint might not exist)
  const success = check(response, {
    'api call responded': (r) => [200, 401, 404].includes(r.status),
  });

  if (!success) errorRate.add(1);
  return success;
}

function performAuthOperation() {
  const startTime = Date.now();

  // Simulate auth check (lightweight operation)
  const response = http.get(`${BASE_URL}/api/v1/auth/status`, {
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
    },
    tags: { name: 'auth_check', type: 'auth' },
  });

  apiDuration.add(Date.now() - startTime);
  requestsTotal.add(1);

  // Accept various responses
  const success = check(response, {
    'auth operation responded': (r) => r.status < 500,
  });

  if (!success) errorRate.add(1);
  return success;
}

export default function () {
  // Determine operation type based on traffic distribution
  const roll = randomIntBetween(1, 100);

  if (roll <= TRAFFIC_DISTRIBUTION.health) {
    group('Health Checks', function () {
      performHealthCheck();
    });
    sleep(randomIntBetween(1, 3) / 10); // 0.1-0.3s (monitoring systems poll frequently)
  } else if (roll <= TRAFFIC_DISTRIBUTION.health + TRAFFIC_DISTRIBUTION.api) {
    group('API Operations', function () {
      performApiCall();
    });
    sleep(randomIntBetween(2, 5) / 10); // 0.2-0.5s (application calls)
  } else {
    group('Auth Operations', function () {
      performAuthOperation();
    });
    sleep(randomIntBetween(5, 10) / 10); // 0.5-1s (user sessions less frequent)
  }
}

export function handleSummary(data) {
  const summary = {
    timestamp: new Date().toISOString(),
    duration: data.state.testRunDurationMs,
    vus_max: data.metrics.vus_max ? data.metrics.vus_max.values.max : 0,
    requests: {
      total: data.metrics.http_reqs ? data.metrics.http_reqs.values.count : 0,
      rate: data.metrics.http_reqs ? data.metrics.http_reqs.values.rate : 0,
    },
    latency: {
      p50: data.metrics.http_req_duration ? data.metrics.http_req_duration.values.med : 0,
      p95: data.metrics.http_req_duration ? data.metrics.http_req_duration.values['p(95)'] : 0,
      p99: data.metrics.http_req_duration ? data.metrics.http_req_duration.values['p(99)'] : 0,
    },
    errors: {
      rate: data.metrics.http_req_failed ? data.metrics.http_req_failed.values.rate : 0,
      count: data.metrics.http_req_failed ? data.metrics.http_req_failed.values.passes : 0,
    },
  };

  return {
    'stdout': JSON.stringify(summary, null, 2),
    'load-tests/results/mixed_workload_summary.json': JSON.stringify(data, null, 2),
  };
}
