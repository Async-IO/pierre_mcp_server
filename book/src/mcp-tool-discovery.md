<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# MCP Tool Discovery & Visibility

Pierre gates which tools appear in MCP `tools/list` responses based on the caller's authentication state. This prevents sensitive tools from being exposed to unauthenticated clients while still letting MCP clients discover Pierre's capabilities.

## Why Gated Discovery

MCP clients (Claude Desktop, VS Code extensions, Cursor, etc.) call `tools/list` to discover what a server can do. Without gating, every tool -- including admin operations, connection management, and social features -- would appear to any caller, even unauthenticated ones.

Gated discovery solves this by returning different tool sets depending on who is asking:

- Unauthenticated callers see a curated public subset that describes Pierre's core capabilities
- Authenticated users see the full set of tools available to them based on tenant plan and role
- Admin users see everything, including admin-only tools

Tools in the public set are *discoverable but not executable*. Calling them without authentication returns an authentication error. The public list exists so MCP clients can present Pierre's value proposition before the user authenticates.

## Visibility Tiers

```
tools/list request
      │
      ▼
┌─────────────┐
│ Has auth     │──── no ──── PUBLIC_DISCOVERY_TOOLS (17 tools)
│ token?       │
└──────┬──────┘
       │ yes
       ▼
┌─────────────┐
│ Token valid? │──── no ──── PUBLIC_DISCOVERY_TOOLS (graceful fallback)
└──────┬──────┘
       │ yes
       ▼
┌─────────────┐
│ Has tenant   │──── no ──── user_visible_schemas (all non-admin tools)
│ context?     │
└──────┬──────┘
       │ yes
       ▼
┌─────────────┐
│ Is admin or  │──── yes ─── all_schemas (every registered tool)
│ owner?       │
└──────┬──────┘
       │ no
       ▼
tenant_filtered_tools
(ToolSelectionService + uncatalogued feature-flag tools, minus admin tools)
```

| auth state | what tools/list returns | source |
|---|---|---|
| no token | `PUBLIC_DISCOVERY_TOOLS` (17 tools) | hardcoded const in `src/constants/tools/identifiers.rs` |
| invalid/expired token | `PUBLIC_DISCOVERY_TOOLS` (graceful fallback) | same const, no error returned |
| authenticated, no tenant | all non-admin tools from registry | `ToolRegistry::user_visible_schemas()` |
| authenticated + tenant (member) | tenant-filtered tools minus admin tools | `ToolSelectionService` + uncatalogued tools |
| authenticated + tenant (admin/owner) | every registered tool | `ToolRegistry::all_schemas()` |

## Public Discovery Tools

The `PUBLIC_DISCOVERY_TOOLS` constant (`src/constants/tools/identifiers.rs`) defines the 17 tools visible without authentication:

**Core data retrieval** (4 tools):
`get_activities`, `get_athlete`, `get_stats`, `get_activity_intelligence`

**Analytics** (5 tools):
`analyze_activity`, `calculate_metrics`, `analyze_performance_trends`, `compare_activities`, `detect_patterns`

**Goal suggestions** (1 tool):
`suggest_goals`

**Nutrition** (4 tools):
`calculate_daily_nutrition`, `search_food`, `get_food_details`, `analyze_meal_nutrition`

**Configuration** (3 tools):
`get_configuration_catalog`, `get_configuration_profiles`, `validate_configuration`

### What is excluded from public discovery

- **Connection management** (`connect_provider`, `get_connection_status`, `disconnect_provider`) -- these manage OAuth tokens and are sensitive
- **Admin tools** (`admin_*` prefix) -- system administration
- **Write/mutation tools** (`set_goal`, `save_recipe`, `update_*`, `delete_*`) -- state-changing operations
- **Social/friends tools** -- not yet implemented, will require auth when added

### Rationale for specific inclusions

The three configuration tools (`get_configuration_catalog`, `get_configuration_profiles`, `validate_configuration`) are included because they are auth-exempt per `ToolId::requires_auth()` -- they can be called without authentication to let clients discover Pierre's configuration schema.

## Token Extraction

The auth token is extracted from two sources, in priority order:

1. **HTTP Authorization header** -- set by the MCP HTTP transport layer (`src/routes/mcp.rs`) from the `Authorization: Bearer <token>` header
2. **MCP params.token** -- read from `request.params.token` in the JSON-RPC request body, useful for MCP transports that don't support HTTP headers (e.g., stdio)

If both are present, the HTTP header takes precedence.

Implementation: `src/mcp/mcp_request_processor.rs`, `resolve_tools_for_request()`.

## Tenant-Filtered Tools

For authenticated non-admin users with a tenant context, the tool list comes from two sources combined:

### 1. ToolSelectionService (catalog-based)

`ToolSelectionService` (`src/mcp/tool_selection.rs`) computes the effective tool list for a tenant by applying rules in precedence order:

1. **Global Disabled** -- `PIERRE_DISABLED_TOOLS` environment variable disables tools for all tenants
2. **Plan Restriction** -- tools require a minimum plan level (starter, professional, enterprise)
3. **Tenant Override** -- admin-configured per-tenant enable/disable with optional reason
4. **Catalog Default** -- default enablement from the `tool_catalog` database table

Only tools where `is_enabled` is true after this cascade are included.

### 2. Uncatalogued feature-flag tools

Tools registered via feature flags (`tools-coaches`, `tools-mobility`) exist in the `ToolRegistry` but may not have entries in `tool_catalog`. The `uncatalogued_user_schemas()` method on `ToolRegistry` returns these tools so they are not lost when filtering through the catalog.

Admin-only tools are excluded from both paths for non-admin users.

### Fallback behavior

If `ToolSelectionService` fails (e.g., database error), the system falls back to `user_visible_schemas()` -- all non-admin tools from the registry, without tenant filtering. This ensures tools/list always returns a usable response.

## Implementation References

- Public tool list constant: `src/constants/tools/identifiers.rs` (`PUBLIC_DISCOVERY_TOOLS`)
- Visibility gating logic: `src/mcp/mcp_request_processor.rs` (`resolve_tools_for_request`, `resolve_tools_for_authenticated_user`, `tenant_filtered_tools`, `public_discovery_tools`)
- Tool selection service: `src/mcp/tool_selection.rs` (`ToolSelectionService`)
- Registry methods: `src/tools/registry.rs` (`list_schemas_by_names`, `list_schemas_by_name_set`, `uncatalogued_user_schemas`, `user_visible_schemas`, `all_schemas`)
- Auth extraction from HTTP: `src/routes/mcp.rs` (line 331-337)
- Tenant context resolution: `src/mcp/tenant_isolation.rs` (`extract_tenant_context_internal`)

## Tests

- `tests/routes_mcp_http_test.rs`:
  - `test_tools_list_unauthenticated_returns_public_tools_only` -- verifies only public tools returned, no sensitive tools leak
  - `test_tools_list_authenticated_returns_more_tools` -- verifies authenticated users see more than the public set
  - `test_tools_list_invalid_token_falls_back_to_public` -- verifies graceful fallback on invalid JWT
  - `test_tools_list_admin_user_sees_all_tools_including_admin` -- verifies admin users see `admin_*` tools
  - `test_tools_list_params_token_auth_path` -- verifies token via `params.token` works for discovery
- `tests/mcp_multitenant_complete_test.rs`:
  - `test_mcp_authentication_required` -- verifies unauthenticated tools/list returns only public subset
