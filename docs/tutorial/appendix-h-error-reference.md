<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Appendix H: Error Code Reference

This appendix provides a comprehensive reference of all error codes, their HTTP status mappings, and recommended handling strategies.

## Error Code Categories

Pierre uses three primary error enums:
- `ErrorCode` - Application-level error codes with HTTP mapping
- `DatabaseError` - Database operation errors
- `ProviderError` - Fitness provider API errors

## ErrorCode → HTTP Status Mapping

**Source**: `src/errors.rs:17-86`

### Authentication & Authorization (4xx)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `AuthRequired` | 401 | No authentication provided | Prompt user to login |
| `AuthInvalid` | 401 | Invalid credentials | Re-authenticate |
| `AuthExpired` | 403 | Token has expired | Refresh token or re-login |
| `AuthMalformed` | 403 | Token is corrupted | Re-authenticate |
| `PermissionDenied` | 403 | Insufficient permissions | Request access or escalate |

### Rate Limiting (429)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `RateLimitExceeded` | 429 | Too many requests | Implement exponential backoff |
| `QuotaExceeded` | 429 | Monthly quota exceeded | Upgrade tier or wait for reset |

### Validation (400)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `InvalidInput` | 400 | Input validation failed | Fix input and retry |
| `MissingRequiredField` | 400 | Required field missing | Include required fields |
| `InvalidFormat` | 400 | Data format incorrect | Check API documentation |
| `ValueOutOfRange` | 400 | Value outside bounds | Use valid value range |

### Resource Management (4xx)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `ResourceNotFound` | 404 | Resource doesn't exist | Check resource ID |
| `ResourceAlreadyExists` | 409 | Duplicate resource | Use existing or rename |
| `ResourceLocked` | 409 | Resource is locked | Wait and retry |
| `ResourceUnavailable` | 503 | Temporarily unavailable | Retry with backoff |

### External Services (5xx)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `ExternalServiceError` | 502 | Provider returned error | Retry or report issue |
| `ExternalServiceUnavailable` | 502 | Provider is down | Retry later |
| `ExternalAuthFailed` | 503 | Provider auth failed | Re-connect provider |
| `ExternalRateLimited` | 503 | Provider rate limited | Wait for provider reset |

### Configuration (500)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `ConfigError` | 500 | Configuration error | Contact administrator |
| `ConfigMissing` | 500 | Missing configuration | Contact administrator |
| `ConfigInvalid` | 500 | Invalid configuration | Contact administrator |

### Internal Errors (500)

| Error Code | HTTP Status | Description | Client Action |
|------------|-------------|-------------|---------------|
| `InternalError` | 500 | Unexpected server error | Retry, then report |
| `DatabaseError` | 500 | Database operation failed | Retry, then report |
| `StorageError` | 500 | Storage operation failed | Retry, then report |
| `SerializationError` | 500 | JSON parsing failed | Check request format |

## DatabaseError Variants

**Source**: `src/database/errors.rs:10-140`

| Variant | Context Fields | Typical Cause |
|---------|---------------|---------------|
| `NotFound` | `entity_type`, `entity_id` | Query returned no rows |
| `TenantIsolationViolation` | `entity_type`, `entity_id`, `requested_tenant`, `actual_tenant` | Cross-tenant access attempt |
| `EncryptionFailed` | `context` | Encryption key issue |
| `DecryptionFailed` | `context` | AAD mismatch or corrupt data |
| `ConstraintViolation` | `constraint`, `details` | Unique/foreign key violation |
| `ConnectionError` | message | Pool exhausted or network |
| `QueryError` | `context` | SQL syntax or type error |
| `MigrationError` | `version`, `details` | Schema migration failed |
| `InvalidData` | `field`, `reason` | Data type mismatch |
| `PoolExhausted` | `max_connections`, `wait_time_ms` | Too many concurrent queries |
| `TransactionRollback` | `reason` | Explicit rollback |
| `SchemaMismatch` | `expected`, `actual` | Database version mismatch |
| `Timeout` | `operation`, `timeout_secs` | Query took too long |
| `TransactionConflict` | `details` | Deadlock or serialization failure |

## ProviderError Variants

**Source**: `src/providers/errors.rs:11-80`

| Variant | Context Fields | Retry Strategy |
|---------|---------------|----------------|
| `ApiError` | `provider`, `status_code`, `message`, `retryable` | Check `retryable` field |
| `RateLimitExceeded` | `provider`, `retry_after_secs`, `limit_type` | Wait `retry_after_secs` |
| `AuthenticationFailed` | `provider`, `reason` | Re-authenticate user |
| `TokenExpired` | `provider` | Auto-refresh token |
| `InvalidResponse` | `provider`, `context` | Log and skip activity |
| `NetworkError` | `provider`, `message` | Retry with backoff |
| `Timeout` | `provider`, `timeout_secs` | Increase timeout or retry |
| `NotSupported` | `provider`, `feature` | Feature unavailable |

## JSON Error Response Format

All API errors return a consistent JSON structure:

```json
{
  "error": {
    "code": "auth_expired",
    "message": "The authentication token has expired",
    "details": {
      "expired_at": "2025-01-15T10:30:00Z",
      "token_type": "access_token"
    },
    "request_id": "req_abc123"
  }
}
```

**Fields**:
- `code`: Machine-readable error code (snake_case)
- `message`: Human-readable description
- `details`: Optional context-specific data
- `request_id`: Correlation ID for debugging

## MCP Error Response Format

For MCP protocol, errors follow JSON-RPC 2.0 spec:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": {
      "pierre_code": "invalid_input",
      "details": "Missing required field: provider"
    }
  }
}
```

**JSON-RPC Error Codes**:
| Code | Meaning | Pierre Mapping |
|------|---------|----------------|
| -32700 | Parse error | `SerializationError` |
| -32600 | Invalid Request | `InvalidInput` |
| -32601 | Method not found | `ResourceNotFound` |
| -32602 | Invalid params | `InvalidInput` |
| -32603 | Internal error | `InternalError` |
| -32000 to -32099 | Server error | Application-specific |

## Retry Strategies

### Exponential Backoff

```rust
// Standard retry with exponential backoff
let delays = [100, 200, 400, 800, 1600]; // milliseconds

for (attempt, delay) in delays.iter().enumerate() {
    match operation().await {
        Ok(result) => return Ok(result),
        Err(e) if e.is_retryable() => {
            tokio::time::sleep(Duration::from_millis(*delay)).await;
        }
        Err(e) => return Err(e),
    }
}
```

### Rate Limit Handling

```rust
match provider.get_activities().await {
    Err(ProviderError::RateLimitExceeded { retry_after_secs, .. }) => {
        // Respect Retry-After header
        tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
        provider.get_activities().await
    }
    result => result,
}
```

## Error Logging

All errors are logged with structured context:

```rust
tracing::error!(
    error_code = %error.code(),
    http_status = error.http_status(),
    request_id = %request_id,
    user_id = %user_id,
    "Operation failed: {}", error
);
```

## Key Takeaways

1. **Consistent HTTP mapping**: `ErrorCode::http_status()` provides standardized status codes.
2. **Structured context**: All errors include relevant context fields for debugging.
3. **Retry guidance**: `retryable` field and `retry_after_secs` guide client behavior.
4. **Tenant isolation**: `TenantIsolationViolation` is a security-critical error.
5. **JSON-RPC compliance**: MCP errors follow JSON-RPC 2.0 specification.
6. **Request correlation**: All errors include `request_id` for distributed tracing.

---

**Related Chapters**:
- Chapter 2: Error Handling (structured error patterns)
- Chapter 9: JSON-RPC Foundation (MCP error codes)
- Appendix E: Rate Limiting (quota errors)
