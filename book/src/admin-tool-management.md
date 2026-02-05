<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Admin Tool Configuration Guide

Pierre provides per-tenant tool management so administrators can control which MCP tools are available to each tenant. This works in conjunction with the [MCP Tool Discovery](mcp-tool-discovery.md) visibility system.

## How Tool Enablement Works

When a non-admin user in a tenant calls `tools/list`, Pierre computes the effective tool list by applying rules in this precedence order:

```
1. Global Disabled (PIERRE_DISABLED_TOOLS)     ← highest priority
2. Plan Restriction (starter/professional/enterprise)
3. Tenant Override (admin-configured per-tenant)
4. Catalog Default (tool_catalog table)         ← lowest priority
```

A tool is only visible to the user if it passes all four checks and `is_enabled` is true. Admin-only tools are excluded regardless.

## Global Tool Disabling

Disable tools for all tenants using the `PIERRE_DISABLED_TOOLS` environment variable:

```bash
# Comma-separated tool names
export PIERRE_DISABLED_TOOLS="predict_performance,analyze_goal_feasibility"
```

This takes the highest priority -- no tenant override can re-enable a globally disabled tool.

To check which tools are globally disabled:

```bash
curl -H "Authorization: Bearer <admin_token>" \
  http://localhost:8081/admin/tools/global-disabled
```

## Per-Tenant Overrides

Administrators can enable or disable specific tools for individual tenants. This is useful for:

- Granting beta testers access to experimental tools
- Disabling tools that a tenant should not have access to
- Customizing the tool set per customer agreement

### Setting an Override

```bash
curl -X POST http://localhost:8081/admin/tools/tenant/<tenant_id>/override \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "tool_name": "predict_performance",
    "is_enabled": false,
    "reason": "Beta feature not ready for this tenant"
  }'
```

The `reason` field is optional but recommended for audit trail.

### Removing an Override

Removing an override causes the tool to fall back to its catalog default or plan-based enablement:

```bash
curl -X DELETE http://localhost:8081/admin/tools/tenant/<tenant_id>/override/predict_performance \
  -H "Authorization: Bearer <admin_token>"
```

## Viewing Tool Status

### Full Tool Catalog

List all tools and their default enablement status:

```bash
curl -H "Authorization: Bearer <admin_token>" \
  http://localhost:8081/admin/tools/catalog
```

### Single Tool Details

```bash
curl -H "Authorization: Bearer <admin_token>" \
  http://localhost:8081/admin/tools/catalog/predict_performance
```

### Effective Tools for a Tenant

Shows the computed result after applying all precedence rules for a specific tenant:

```bash
curl -H "Authorization: Bearer <admin_token>" \
  http://localhost:8081/admin/tools/tenant/<tenant_id>
```

Each tool in the response includes:
- `tool_name` -- tool identifier
- `is_enabled` -- whether the tool is available to this tenant
- `source` -- why the tool has its current status (catalog default, plan restriction, tenant override, or global disabled)

### Availability Summary

A high-level view of how many tools are enabled for a tenant, broken down by category:

```bash
curl -H "Authorization: Bearer <admin_token>" \
  http://localhost:8081/admin/tools/tenant/<tenant_id>/summary
```

Response includes:
- `total_tools` -- total tools in catalog
- `enabled_tools` -- how many are enabled for this tenant
- `categories` -- per-category breakdown with enabled/total counts

## API Reference

All endpoints require an admin JWT token with appropriate permissions.

| method | endpoint | permission | description |
|--------|----------|------------|-------------|
| GET | `/admin/tools/catalog` | ViewConfiguration | list all tools in catalog |
| GET | `/admin/tools/catalog/:tool_name` | ViewConfiguration | get single tool details |
| GET | `/admin/tools/tenant/:tenant_id` | ViewConfiguration | get effective tools for tenant |
| POST | `/admin/tools/tenant/:tenant_id/override` | ManageConfiguration | set tool override |
| DELETE | `/admin/tools/tenant/:tenant_id/override/:tool_name` | ManageConfiguration | remove override |
| GET | `/admin/tools/tenant/:tenant_id/summary` | ViewConfiguration | get availability summary |
| GET | `/admin/tools/global-disabled` | ViewConfiguration | list globally disabled tools |

### Override Request Body

```json
{
  "tool_name": "string (required)",
  "is_enabled": "boolean (required)",
  "reason": "string (optional)"
}
```

## Frontend Admin UI

The web frontend includes a tool management interface at the Admin Configuration page. The `ToolAvailability` component (`frontend/src/components/ToolAvailability.tsx`) provides:

- A summary bar showing enabled/total tool counts
- Per-category tool listing with enable/disable toggles
- Override reason input when disabling a tool
- Visual indicators for override source (catalog default vs tenant override vs global disabled)

## Use Cases

### Disable a beta tool for all tenants except testers

1. Globally disable the tool:
   ```bash
   export PIERRE_DISABLED_TOOLS="new_beta_tool"
   ```

2. Override for the tester tenant to re-enable:
   ```bash
   curl -X POST http://localhost:8081/admin/tools/tenant/<tester_tenant_id>/override \
     -H "Authorization: Bearer <admin_token>" \
     -H "Content-Type: application/json" \
     -d '{"tool_name": "new_beta_tool", "is_enabled": true, "reason": "Beta tester"}'
   ```

   Note: globally disabled tools cannot be overridden. For this use case, use plan restrictions or catalog defaults instead, and override only the tester tenant.

### Restrict a tool to enterprise plan only

The `tool_catalog` table supports a `min_plan` column. Tools with `min_plan: "enterprise"` are automatically disabled for starter and professional tenants without needing per-tenant overrides.

### Temporarily disable a misbehaving tool

Set a global disable via environment variable and restart the server. This takes effect immediately for all tenants:

```bash
export PIERRE_DISABLED_TOOLS="broken_tool"
./bin/stop-server.sh && ./bin/start-server.sh
```

## Caching

The `ToolSelectionService` caches effective tool lists per tenant with a 5-minute TTL (configurable). Setting or removing an override invalidates the cache for that tenant. Global disabling requires a server restart to take effect.

## Implementation References

- Tool selection service: `src/mcp/tool_selection.rs`
- Admin API routes: `src/routes/tool_selection.rs`
- Frontend component: `frontend/src/components/ToolAvailability.tsx`
- Configuration: `src/config/mod.rs` (`ToolSelectionConfig`)
- Database models: `src/models/tool_catalog.rs`
