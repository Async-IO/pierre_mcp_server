# architecture

pierre is a multi-protocol fitness data server that connects AI assistants to strava, garmin, and fitbit. single binary, single port (8081), multiple protocols.

## system design

```
┌─────────────────┐
│   mcp clients   │ claude desktop, chatgpt, etc
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   pierre sdk    │ typescript bridge (stdio → http)
│   (npm package) │
└────────┬────────┘
         │ http + oauth2
         ▼
┌─────────────────────────────────────────┐
│   pierre server (rust)                  │
│   port 8081 (all protocols)             │
│                                          │
│   • mcp protocol (json-rpc 2.0)        │
│   • oauth2 server (rfc 7591)           │
│   • a2a protocol (agent-to-agent)      │
│   • rest api                            │
│   • sse (real-time notifications)      │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│   fitness providers                     │
│   • strava                              │
│   • garmin                              │
│   • fitbit                              │
└─────────────────────────────────────────┘
```

## core components

### protocols layer (`src/protocols/`)
- `universal/` - protocol-agnostic business logic
- shared by mcp and a2a protocols
- 25 fitness tools (activities, analysis, goals)

### mcp implementation (`src/mcp/`)
- json-rpc 2.0 over http
- sse transport for streaming
- tool registry and execution

### oauth2 server (`src/oauth2/`)
- rfc 7591 dynamic client registration
- rfc 7636 pkce support
- jwt access tokens for mcp clients

### providers (`src/providers/`)
- trait-based fitness provider abstraction
- strava, garmin, fitbit implementations
- unified oauth token management

### intelligence (`src/intelligence/`)
- activity analysis and insights
- performance trend detection
- training load calculation
- goal feasibility analysis

### database (`src/database_plugins/`)
- pluggable backend (sqlite, postgresql)
- encrypted token storage
- multi-tenant isolation

### authentication (`src/auth.rs`)
- jwt token generation/validation
- api key management
- rate limiting per tenant

## request flow

```
client request
    ↓
[security middleware] → cors, headers, csrf
    ↓
[authentication] → jwt or api key
    ↓
[tenant context] → load user/tenant data
    ↓
[rate limiting] → check quotas
    ↓
[protocol router]
    ├─ mcp → universal protocol → tools
    ├─ a2a → universal protocol → tools
    └─ rest → direct handlers
    ↓
[tool execution]
    ├─ providers (strava/garmin/fitbit)
    ├─ intelligence (analysis)
    └─ configuration
    ↓
[database + cache]
    ↓
response
```

## multi-tenancy

every request operates within tenant context:
- isolated data per tenant
- tenant-specific encryption keys
- custom rate limits
- feature flags

## key design decisions

### single port architecture
all protocols share port 8081. simplified deployment, easier oauth2 callback handling, unified tls/security.

### dependency injection via serverresources
all components initialized once at startup, shared via `Arc<T>`. eliminates resource creation anti-patterns.

### protocol abstraction
business logic in `protocols::universal` works for both mcp and a2a. write once, use everywhere.

### pluggable architecture
- database: sqlite (dev) or postgresql (prod)
- cache: in-memory lru (current), redis (future)
- tools: compile-time plugin system via `linkme`

## file structure

```
src/
├── bin/
│   ├── pierre-mcp-server.rs  # main binary
│   └── admin-setup.rs         # admin cli tool
├── protocols/
│   └── universal/             # shared business logic
├── mcp/                       # mcp protocol
├── oauth2/                    # oauth2 authorization server
├── a2a/                       # a2a protocol
├── providers/                 # fitness integrations
├── intelligence/              # activity analysis
├── database_plugins/          # database backends
├── auth.rs                    # authentication
├── oauth/                     # provider oauth
├── tenant/                    # multi-tenancy
├── tools/                     # tool execution engine
├── cache/                     # caching layer
└── lib.rs                     # public api
```

## security layers

1. **transport**: https/tls
2. **authentication**: jwt tokens, api keys
3. **authorization**: tenant-based rbac
4. **encryption**: two-tier key management
   - master key: encrypts tenant keys
   - tenant keys: encrypt user tokens
5. **rate limiting**: token bucket per tenant

## scalability

### horizontal scaling
stateless server design. scale by adding instances behind load balancer. shared postgresql + redis for state.

### database sharding
- tenant-based sharding
- time-based partitioning for historical data
- provider-specific tables

### caching strategy
- health checks: 30s ttl
- mcp sessions: lru cache (10k entries)
- weather data: configurable ttl
- future: redis for distributed cache

## monitoring

health endpoint: `GET /health`
- database connectivity
- provider availability
- system uptime
- cache statistics

logs: structured json via tracing + opentelemetry
metrics: request latency, error rates, provider api usage
