# Pierre MCP Server - Technical Debt and Future Improvements

## Priority 1: First Release Requirements

### PostgreSQL Support (IN PROGRESS)
Complete PostgreSQL implementation for multi-tenant architecture.

**Tenant Management:**
- [ ] `create_tenant()` - Store tenant records with proper constraints
- [ ] `get_tenant_by_id()` - Retrieve tenant by UUID
- [ ] `get_tenant_by_slug()` - Retrieve tenant by slug (unique constraint)
- [ ] `list_tenants_for_user()` - List all tenants a user belongs to

**OAuth Credentials Management:**
- [ ] `store_tenant_oauth_credentials()` - Store encrypted OAuth credentials per tenant
- [ ] `get_tenant_oauth_credentials()` - Retrieve all credentials for a tenant
- [ ] `get_tenant_oauth_credential()` - Get specific provider credentials

**OAuth App Registration:**
- [ ] `create_oauth_app()` - Register OAuth applications
- [ ] `get_oauth_app_by_client_id()` - Retrieve app by client ID
- [ ] `list_oauth_apps_for_user()` - List user's OAuth apps
- [ ] `store_authorization_code()` - Store OAuth authorization codes
- [ ] `get_authorization_code()` - Retrieve and validate auth codes
- [ ] `delete_authorization_code()` - Clean up used auth codes

## Priority 2: Security Enhancements

### Key Versioning System
Implement proper key versioning for encryption manager to support safe key rotation.

**Requirements:**
- Track key version per encrypted data item
- Support multiple active key versions during rotation
- Database schema for key_versions table
- Automatic re-encryption with new keys
- Key version metadata in EncryptedData structure

**Implementation needed in:**
- `src/security/mod.rs`: Remove hardcoded version 1
- Database: Add key_versions table
- Add migration scripts for key rotation

### Audit Event Persistence
Store security audit events in database for compliance and forensics.

**Requirements:**
- Database table for audit_events
- Structured storage of AuditEvent data
- Query interface for audit log analysis
- Retention policies (90 days minimum)
- Export capabilities for compliance reports

**Implementation needed in:**
- `src/security/audit.rs`: Replace TODO in store_audit_event()
- Database: Add audit_events table with indexes
- Add audit query APIs

### Security Alerting System
Implement real-time alerting for critical security events.

**Requirements:**
- Email notifications for critical events
- Webhook support for external monitoring
- Rate limiting on alerts to prevent spam
- Configurable alert thresholds

**Implementation needed in:**
- `src/security/audit.rs`: Implement trigger_security_alert()
- Configuration for alert destinations
- Alert templating system

## Priority 3: Performance Optimizations

### Connection Pooling
Implement database connection pooling for better resource utilization.

**Requirements:**
- Configure pool size based on workload
- Connection health checks
- Automatic reconnection on failure
- Per-tenant connection limits

### Caching Layer
Add caching to reduce database load.

**Cache targets:**
- Tenant configurations
- User sessions
- Rate limit counters
- OAuth credentials (encrypted in cache)

**Technology options:**
- In-memory cache with TTL
- Redis for distributed deployments

### Query Optimization
Optimize database queries for multi-tenant scale.

**Optimizations needed:**
- Batch insert operations
- Prepared statement caching
- Index optimization for tenant_id columns
- Query plan analysis and tuning

## Priority 4: Feature Enhancements

### Key Rotation Automation
Automate the key rotation process.

**Requirements:**
- Scheduled rotation jobs
- Progress tracking for re-encryption
- Zero-downtime rotation
- Rollback capabilities

**Implementation needed in:**
- `src/security/key_rotation.rs`: Implement get_all_tenants()
- Background job system
- Progress tracking in database

### Multi-Region Support
Support for geographically distributed deployments.

**Requirements:**
- Region-specific encryption keys
- Cross-region replication
- Latency-based routing
- Data residency compliance

## Not Planned (Documented Limitations)

### Single Database Type per Instance
The current architecture supports either SQLite OR PostgreSQL per instance, not both simultaneously. This is by design to reduce complexity.

### Manual Key Rotation for v1
For the first release, key rotation will be a manual process with documented procedures rather than fully automated.

## Technical Debt

### Code Cleanup
- Remove stub implementations once PostgreSQL is complete
- Consolidate error handling patterns
- Standardize logging formats

### Testing
- Add integration tests for PostgreSQL
- Performance benchmarks for multi-tenant operations
- Security penetration testing
- Load testing for rate limiters

### Documentation
- API documentation generation
- Architecture decision records (ADRs)
- Deployment guides for production
- Security best practices guide

## Notes

This document tracks planned improvements and known limitations. Items are prioritized based on:
1. Security impact
2. User experience impact  
3. Operational requirements
4. Technical complexity

Last updated: 2025-08-10