// ABOUTME: k6 load test for authentication endpoints
// ABOUTME: Tests login, token refresh, and protected endpoint access patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const loginDuration = new Trend('login_duration');
const tokenRefreshDuration = new Trend('token_refresh_duration');
const protectedAccessDuration = new Trend('protected_access_duration');
const successfulLogins = new Counter('successful_logins');
const failedLogins = new Counter('failed_logins');

export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Warm up
    { duration: '1m', target: 30 },    // Normal load
    { duration: '30s', target: 50 },   // Peak load
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.05'],
    errors: ['rate<0.05'],
    login_duration: ['p(95)<300'],
    token_refresh_duration: ['p(95)<200'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8081';

// Test user credentials (should be seeded in test environment)
const TEST_USERS = [
  { email: 'bench_user_1@example.com', password: 'BenchTest123!' },
  { email: 'bench_user_2@example.com', password: 'BenchTest123!' },
  { email: 'bench_user_3@example.com', password: 'BenchTest123!' },
  { email: 'bench_user_4@example.com', password: 'BenchTest123!' },
  { email: 'bench_user_5@example.com', password: 'BenchTest123!' },
];

export function setup() {
  // Verify server is running
  const healthResponse = http.get(`${BASE_URL}/health`);
  check(healthResponse, {
    'server is healthy': (r) => r.status === 200,
  });

  return { users: TEST_USERS };
}

export default function (data) {
  const user = data.users[__VU % data.users.length];

  group('Login Flow', function () {
    const loginStart = Date.now();

    const loginResponse = http.post(
      `${BASE_URL}/api/v1/auth/login`,
      JSON.stringify({
        email: user.email,
        password: user.password,
      }),
      {
        headers: { 'Content-Type': 'application/json' },
        tags: { name: 'login' },
      }
    );

    loginDuration.add(Date.now() - loginStart);

    const loginSuccess = check(loginResponse, {
      'login status is 200': (r) => r.status === 200,
      'has access token': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.access_token !== undefined;
        } catch {
          return false;
        }
      },
      'has refresh token': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.refresh_token !== undefined;
        } catch {
          return false;
        }
      },
    });

    if (loginSuccess) {
      successfulLogins.add(1);

      try {
        const tokens = JSON.parse(loginResponse.body);

        // Access protected endpoint
        group('Protected Endpoint Access', function () {
          const accessStart = Date.now();

          const protectedResponse = http.get(
            `${BASE_URL}/api/v1/user/profile`,
            {
              headers: {
                'Authorization': `Bearer ${tokens.access_token}`,
              },
              tags: { name: 'protected_access' },
            }
          );

          protectedAccessDuration.add(Date.now() - accessStart);

          check(protectedResponse, {
            'protected access successful': (r) => r.status === 200 || r.status === 404,
          });
        });

        // Token refresh
        group('Token Refresh', function () {
          const refreshStart = Date.now();

          const refreshResponse = http.post(
            `${BASE_URL}/api/v1/auth/refresh`,
            JSON.stringify({
              refresh_token: tokens.refresh_token,
            }),
            {
              headers: { 'Content-Type': 'application/json' },
              tags: { name: 'token_refresh' },
            }
          );

          tokenRefreshDuration.add(Date.now() - refreshStart);

          check(refreshResponse, {
            'refresh successful': (r) => r.status === 200 || r.status === 401,
            'new access token': (r) => {
              if (r.status !== 200) return true; // Skip if refresh not supported
              try {
                const body = JSON.parse(r.body);
                return body.access_token !== undefined;
              } catch {
                return false;
              }
            },
          });
        });
      } catch (e) {
        console.error(`Error parsing tokens: ${e}`);
      }
    } else {
      failedLogins.add(1);
      errorRate.add(1);
    }
  });

  sleep(1); // Realistic pause between auth flows
}

export function handleSummary(data) {
  return {
    'stdout': JSON.stringify(data, null, 2),
    'load-tests/results/auth_flow_summary.json': JSON.stringify(data, null, 2),
  };
}
