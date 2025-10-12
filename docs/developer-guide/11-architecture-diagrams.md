# Architecture Diagrams

This document contains comprehensive architecture diagrams for Pierre MCP Server, showing system structure, component relationships, and data flow patterns.

## Table of Contents

1. [System Overview Architecture](#system-overview-architecture)
2. [Component Architecture](#component-architecture)
3. [Protocol Layer Architecture](#protocol-layer-architecture)
4. [Multi-Tenant Architecture](#multi-tenant-architecture)
5. [Database Architecture](#database-architecture)
6. [Security Architecture](#security-architecture)
7. [Deployment Architecture](#deployment-architecture)
8. [Network Flow Architecture](#network-flow-architecture)

## System Overview Architecture

```mermaid
graph TB
    subgraph "External Clients"
        MCP[MCP Clients<br/>Claude Desktop, Continue, etc.]
        A2A[A2A Agents<br/>Discord Bots, Analytics, etc.]
        WEB[Web Dashboard<br/>Admin & User Interface]
        API[API Clients<br/>Mobile Apps, Scripts]
    end

    subgraph "Load Balancer / Reverse Proxy"
        LB[Nginx / Traefik<br/>SSL Termination<br/>Rate Limiting]
    end

    subgraph "Pierre MCP Server Cluster"
        subgraph "Server Instance 1"
            WS1[WebSocket Handler]
            HTTP1[HTTP Router]
            MCP1[MCP Protocol]
            A2A1[A2A Protocol]
        end
        
        subgraph "Server Instance 2"
            WS2[WebSocket Handler]
            HTTP2[HTTP Router]
            MCP2[MCP Protocol]
            A2A2[A2A Protocol]
        end
    end

    subgraph "Shared Services"
        REDIS[(Redis Cache<br/>Rate Limiting<br/>Session Storage)]
        DB[(Database<br/>PostgreSQL/SQLite<br/>Multi-Tenant)]
    end

    subgraph "External APIs"
        STRAVA[Strava API<br/>OAuth Provider]
        FITBIT[Fitbit API<br/>OAuth Provider]
        WEATHER[Weather APIs<br/>OpenWeatherMap]
    end

    MCP --> LB
    A2A --> LB
    WEB --> LB
    API --> LB

    LB --> WS1
    LB --> HTTP1
    LB --> WS2
    LB --> HTTP2

    WS1 --> MCP1
    HTTP1 --> A2A1
    WS2 --> MCP2
    HTTP2 --> A2A2

    MCP1 --> REDIS
    MCP1 --> DB
    MCP1 --> STRAVA
    MCP1 --> FITBIT

    A2A1 --> REDIS
    A2A1 --> DB

    MCP2 --> REDIS
    MCP2 --> DB
    MCP2 --> STRAVA
    MCP2 --> FITBIT

    A2A2 --> REDIS
    A2A2 --> DB

    style MCP fill:#e1f5fe
    style A2A fill:#f3e5f5
    style WEB fill:#e8f5e8
    style REDIS fill:#ffebee
    style DB fill:#fff3e0
```

## Component Architecture

```mermaid
graph TB
    subgraph "Application Layer"
        SERVER[Pierre MCP Server<br/>main.rs]
        CONFIG[Configuration Manager<br/>Environment & TOML]
    end

    subgraph "Protocol Handlers"
        MCP_HANDLER[MCP Protocol Handler<br/>JSON-RPC 2.0]
        A2A_HANDLER[A2A Protocol Handler<br/>REST API]
        WS_HANDLER[WebSocket Handler<br/>Real-time Communication]
        HTTP_HANDLER[HTTP Router<br/>Axum/Warp]
    end

    subgraph "Core Services"
        AUTH[Authentication Manager<br/>JWT + API Keys]
        TENANT[Tenant Manager<br/>Multi-tenancy]
        RATE[Rate Limiter<br/>Token Bucket]
        PROVIDER[Provider Manager<br/>OAuth Integration]
    end

    subgraph "Data Layer"
        DB_FACTORY[Database Factory<br/>Plugin Architecture]
        SQLITE[SQLite Plugin<br/>Development]
        POSTGRES[PostgreSQL Plugin<br/>Production]
        ENCRYPTION[Encryption Service<br/>AES-256-GCM]
    end

    subgraph "External Integration"
        OAUTH[OAuth Providers<br/>Strava, Garmin, Fitbit]
        CACHE[Redis Cache<br/>Rate Limiting]
        METRICS[Telemetry<br/>OpenTelemetry]
    end

    SERVER --> MCP_HANDLER
    SERVER --> A2A_HANDLER
    SERVER --> WS_HANDLER
    SERVER --> HTTP_HANDLER
    SERVER --> CONFIG

    MCP_HANDLER --> AUTH
    A2A_HANDLER --> AUTH
    WS_HANDLER --> AUTH
    HTTP_HANDLER --> AUTH

    AUTH --> TENANT
    AUTH --> RATE
    AUTH --> PROVIDER

    TENANT --> DB_FACTORY
    RATE --> CACHE
    PROVIDER --> OAUTH

    DB_FACTORY --> SQLITE
    DB_FACTORY --> POSTGRES
    DB_FACTORY --> ENCRYPTION

    PROVIDER --> ENCRYPTION

    style SERVER fill:#ff9800
    style AUTH fill:#4caf50
    style TENANT fill:#2196f3
    style DB_FACTORY fill:#9c27b0
    style ENCRYPTION fill:#f44336
```

## Protocol Layer Architecture

```mermaid
graph LR
    subgraph "Client Layer"
        CLIENT1[MCP Client<br/>Claude Desktop]
        CLIENT2[A2A Agent<br/>Discord Bot]
        CLIENT3[Web Client<br/>Dashboard]
    end

    subgraph "Transport Layer"
        WS[WebSocket<br/>MCP Protocol]
        HTTP[HTTP/REST<br/>A2A & Web API]
    end

    subgraph "Protocol Processing"
        subgraph "MCP Stack"
            MCP_PARSER[JSON-RPC Parser<br/>v2025-06-18]
            MCP_ROUTER[Method Router<br/>tools/*, notifications/*]
            MCP_HANDLER[Tool Handlers<br/>get_activities, etc.]
        end

        subgraph "A2A Stack"
            REST_PARSER[HTTP Parser<br/>JSON Payloads]
            REST_ROUTER[Route Router<br/>/a2a/*, /oauth/*]
            REST_HANDLER[REST Handlers<br/>Registration, Discovery]
        end
    end

    subgraph "Business Logic"
        AUTH_LAYER[Authentication<br/>JWT & API Key Validation]
        BUSINESS_LAYER[Business Logic<br/>Data Processing]
        INTEGRATION_LAYER[External Integration<br/>Strava, Garmin, Fitbit APIs]
    end

    CLIENT1 --> WS
    CLIENT2 --> HTTP
    CLIENT3 --> HTTP

    WS --> MCP_PARSER
    HTTP --> REST_PARSER

    MCP_PARSER --> MCP_ROUTER
    REST_PARSER --> REST_ROUTER

    MCP_ROUTER --> MCP_HANDLER
    REST_ROUTER --> REST_HANDLER

    MCP_HANDLER --> AUTH_LAYER
    REST_HANDLER --> AUTH_LAYER

    AUTH_LAYER --> BUSINESS_LAYER
    BUSINESS_LAYER --> INTEGRATION_LAYER

    style CLIENT1 fill:#e3f2fd
    style CLIENT2 fill:#f3e5f5
    style CLIENT3 fill:#e8f5e8
    style MCP_PARSER fill:#fff3e0
    style REST_PARSER fill:#fff3e0
    style AUTH_LAYER fill:#ffebee
    style INTEGRATION_LAYER fill:#f1f8e9
```

## Multi-Tenant Architecture

```mermaid
graph TB
    subgraph "Tenant A - SaaS Customer"
        TENANT_A[Tenant A Configuration]
        USERS_A[Users A1, A2, A3]
        OAUTH_A[OAuth Config A<br/>Custom Strava App]
        KEYS_A[API Keys A<br/>Rate Limits A]
    end

    subgraph "Tenant B - Enterprise Customer"
        TENANT_B[Tenant B Configuration]
        USERS_B[Users B1, B2, B3]
        OAUTH_B[OAuth Config B<br/>Custom Fitbit App]
        KEYS_B[API Keys B<br/>Rate Limits B]
    end

    subgraph "Shared Infrastructure"
        subgraph "Application Layer"
            SERVER[Pierre MCP Server<br/>Single Instance]
            TENANT_MANAGER[Tenant Manager<br/>Isolation Layer]
        end

        subgraph "Data Layer"
            subgraph "Database Tables"
                T_TENANTS[(tenants)]
                T_USERS[(users)]
                T_TOKENS[(user_tokens)]
                T_KEYS[(api_keys)]
                T_OAUTH[(oauth_configs)]
            end
        end

        subgraph "Cache Layer"
            CACHE_A[Tenant A Cache<br/>Namespace: tenant_a:*]
            CACHE_B[Tenant B Cache<br/>Namespace: tenant_b:*]
        end
    end

    TENANT_A --> SERVER
    TENANT_B --> SERVER

    SERVER --> TENANT_MANAGER

    TENANT_MANAGER --> T_TENANTS
    TENANT_MANAGER --> T_USERS
    TENANT_MANAGER --> T_TOKENS
    TENANT_MANAGER --> T_KEYS
    TENANT_MANAGER --> T_OAUTH

    USERS_A --> CACHE_A
    USERS_B --> CACHE_B

    OAUTH_A -.-> T_OAUTH
    OAUTH_B -.-> T_OAUTH

    KEYS_A -.-> T_KEYS
    KEYS_B -.-> T_KEYS

    style TENANT_A fill:#e1f5fe
    style TENANT_B fill:#f3e5f5
    style TENANT_MANAGER fill:#fff3e0
    style T_TENANTS fill:#ffebee
```

## Database Architecture

```mermaid
erDiagram
    TENANTS {
        uuid id PK
        string name
        string plan_type
        jsonb settings
        timestamp created_at
        timestamp updated_at
    }

    USERS {
        uuid id PK
        uuid tenant_id FK
        string email
        string provider_id
        enum provider
        boolean is_active
        timestamp created_at
        timestamp last_login
    }

    API_KEYS {
        string id PK
        uuid user_id FK
        uuid tenant_id FK
        string name
        text description
        enum tier
        string key_prefix
        bytes key_hash
        boolean is_active
        timestamp expires_at
        timestamp last_used_at
        timestamp created_at
    }

    USER_TOKENS {
        uuid id PK
        uuid user_id FK
        uuid tenant_id FK
        enum provider
        bytes encrypted_access_token
        bytes encrypted_refresh_token
        timestamp expires_at
        timestamp created_at
        timestamp updated_at
    }

    OAUTH_CONFIGS {
        uuid id PK
        uuid tenant_id FK
        enum provider
        bytes encrypted_client_id
        bytes encrypted_client_secret
        string redirect_uri
        jsonb scopes
        boolean is_active
        timestamp created_at
    }

    RATE_LIMIT_RECORDS {
        string id PK
        uuid tenant_id FK
        string resource_type
        string resource_id
        integer tokens_remaining
        timestamp last_refill
        timestamp created_at
    }

    A2A_AGENTS {
        string agent_id PK
        uuid user_id FK
        uuid tenant_id FK
        string name
        jsonb capabilities
        jsonb metadata
        boolean is_active
        timestamp registered_at
        timestamp last_seen
    }

    AUDIT_LOGS {
        uuid id PK
        uuid tenant_id FK
        uuid user_id FK
        string action
        jsonb details
        string ip_address
        string user_agent
        timestamp created_at
    }

    TENANTS ||--o{ USERS : contains
    TENANTS ||--o{ API_KEYS : owns
    TENANTS ||--o{ OAUTH_CONFIGS : configures
    TENANTS ||--o{ RATE_LIMIT_RECORDS : manages
    TENANTS ||--o{ A2A_AGENTS : hosts
    TENANTS ||--o{ AUDIT_LOGS : tracks

    USERS ||--o{ API_KEYS : creates
    USERS ||--o{ USER_TOKENS : owns
    USERS ||--o{ A2A_AGENTS : registers
    USERS ||--o{ AUDIT_LOGS : performs
```

## Security Architecture

```mermaid
graph TB
    subgraph "External Threats"
        ATTACKER[Malicious Actors]
        BOT[Automated Bots]
        MITM[Man-in-the-Middle]
    end

    subgraph "Security Perimeter"
        subgraph "Network Security"
            FIREWALL[Firewall<br/>Port Restrictions]
            DDoS[DDoS Protection<br/>Rate Limiting]
            TLS[TLS 1.3<br/>SSL Termination]
        end

        subgraph "Application Security"
            AUTH[Multi-Factor Authentication<br/>JWT + API Keys]
            RBAC[Role-Based Access Control<br/>Tenant Isolation]
            VALIDATION[Input Validation<br/>Schema Enforcement]
        end

        subgraph "Data Security"
            ENCRYPTION_REST[Encryption at Rest<br/>AES-256-GCM]
            ENCRYPTION_TRANSIT[Encryption in Transit<br/>TLS 1.3]
            KEY_MANAGEMENT[Key Management<br/>Secure Key Rotation]
        end
    end

    subgraph "Internal Systems"
        APP[Application Layer]
        DB[Database Layer]
        CACHE[Cache Layer]
        LOGS[Audit Logs]
    end

    subgraph "Monitoring & Response"
        SIEM[Security Monitoring<br/>OpenTelemetry]
        ALERTS[Alert System<br/>Anomaly Detection]
        INCIDENT[Incident Response<br/>Automated Actions]
    end

    ATTACKER -.-> FIREWALL
    BOT -.-> DDoS
    MITM -.-> TLS

    FIREWALL --> AUTH
    DDoS --> AUTH
    TLS --> AUTH

    AUTH --> RBAC
    RBAC --> VALIDATION
    VALIDATION --> APP

    APP --> ENCRYPTION_REST
    APP --> DB
    DB --> CACHE

    APP --> LOGS
    LOGS --> SIEM
    SIEM --> ALERTS
    ALERTS --> INCIDENT

    ENCRYPTION_REST --> KEY_MANAGEMENT
    ENCRYPTION_TRANSIT --> KEY_MANAGEMENT

    style ATTACKER fill:#f44336
    style BOT fill:#f44336
    style MITM fill:#f44336
    style FIREWALL fill:#4caf50
    style DDoS fill:#4caf50
    style TLS fill:#4caf50
    style ENCRYPTION_REST fill:#2196f3
    style ENCRYPTION_TRANSIT fill:#2196f3
    style SIEM fill:#ff9800
```

## Deployment Architecture

```mermaid
graph TB
    subgraph "Production Environment"
        subgraph "Kubernetes Cluster"
            subgraph "Ingress Layer"
                INGRESS[Nginx Ingress<br/>SSL Termination<br/>Load Balancing]
            end

            subgraph "Application Pods"
                POD1[Pierre MCP Server Pod 1<br/>CPU: 1000m, RAM: 2Gi]
                POD2[Pierre MCP Server Pod 2<br/>CPU: 1000m, RAM: 2Gi]
                POD3[Pierre MCP Server Pod 3<br/>CPU: 1000m, RAM: 2Gi]
            end

            subgraph "Service Layer"
                SVC[Service<br/>ClusterIP<br/>Port 3000]
                HPA[Horizontal Pod Autoscaler<br/>CPU: 70%, Replicas: 3-10]
            end

            subgraph "Configuration"
                CM[ConfigMap<br/>App Configuration]
                SECRET[Secret<br/>Encryption Keys, DB Credentials]
            end
        end

        subgraph "External Services"
            RDS[Amazon RDS<br/>PostgreSQL 15<br/>Multi-AZ]
            ELASTICACHE[Amazon ElastiCache<br/>Redis Cluster<br/>Failover Enabled]
        end

        subgraph "Monitoring Stack"
            PROMETHEUS[Prometheus<br/>Metrics Collection]
            GRAFANA[Grafana<br/>Dashboards]
            JAEGER[Jaeger<br/>Distributed Tracing]
        end
    end

    subgraph "Development Environment"
        DEV_POD[Single Pod<br/>SQLite Database<br/>Local Redis]
        DEV_CONFIG[Dev ConfigMap<br/>Debug Settings]
    end

    subgraph "CI/CD Pipeline"
        GITHUB[GitHub Actions<br/>Build & Test]
        REGISTRY[Container Registry<br/>Docker Images]
        ARGOCD[ArgoCD<br/>GitOps Deployment]
    end

    INGRESS --> SVC
    SVC --> POD1
    SVC --> POD2
    SVC --> POD3

    HPA --> POD1
    HPA --> POD2
    HPA --> POD3

    POD1 --> RDS
    POD1 --> ELASTICACHE
    POD2 --> RDS
    POD2 --> ELASTICACHE
    POD3 --> RDS
    POD3 --> ELASTICACHE

    CM --> POD1
    CM --> POD2
    CM --> POD3

    SECRET --> POD1
    SECRET --> POD2
    SECRET --> POD3

    POD1 --> PROMETHEUS
    POD2 --> PROMETHEUS
    POD3 --> PROMETHEUS

    PROMETHEUS --> GRAFANA
    POD1 --> JAEGER
    POD2 --> JAEGER
    POD3 --> JAEGER

    GITHUB --> REGISTRY
    REGISTRY --> ARGOCD
    ARGOCD --> POD1
    ARGOCD --> POD2
    ARGOCD --> POD3

    style POD1 fill:#4caf50
    style POD2 fill:#4caf50
    style POD3 fill:#4caf50
    style RDS fill:#2196f3
    style ELASTICACHE fill:#f44336
    style PROMETHEUS fill:#ff9800
```

## Network Flow Architecture

```mermaid
graph LR
    subgraph "Client Network"
        CLIENT[MCP Client<br/>Claude Desktop]
        AGENT[A2A Agent<br/>Discord Bot]
    end

    subgraph "Internet"
        INTERNET[Public Internet<br/>HTTPS/WSS Traffic]
    end

    subgraph "CDN/Edge"
        CDN[CloudFlare<br/>DDoS Protection<br/>SSL Termination]
    end

    subgraph "Load Balancer"
        ALB[Application Load Balancer<br/>AWS ALB / Nginx<br/>Health Checks]
    end

    subgraph "Pierre Server Cluster"
        subgraph "Server 1"
            WS1[WebSocket Handler<br/>:3000/ws]
            HTTP1[HTTP Handler<br/>:3000/api]
        end

        subgraph "Server 2"
            WS2[WebSocket Handler<br/>:3000/ws]
            HTTP2[HTTP Handler<br/>:3000/api]
        end

        subgraph "Shared State"
            REDIS[Redis Cluster<br/>Session Storage<br/>Rate Limiting]
        end
    end

    subgraph "External APIs"
        STRAVA_API[Strava API<br/>api.strava.com:443]
        FITBIT_API[Fitbit API<br/>api.fitbit.com:443]
    end

    subgraph "Database"
        DB[PostgreSQL<br/>Primary/Replica<br/>Encrypted Connection]
    end

    CLIENT -->|WSS| INTERNET
    AGENT -->|HTTPS| INTERNET
    INTERNET --> CDN
    CDN --> ALB

    ALB -->|Round Robin| WS1
    ALB -->|Round Robin| HTTP1
    ALB -->|Round Robin| WS2
    ALB -->|Round Robin| HTTP2

    WS1 --> REDIS
    HTTP1 --> REDIS
    WS2 --> REDIS
    HTTP2 --> REDIS

    WS1 --> DB
    HTTP1 --> DB
    WS2 --> DB
    HTTP2 --> DB

    HTTP1 -->|OAuth Flow| STRAVA_API
    HTTP1 -->|OAuth Flow| FITBIT_API
    HTTP2 -->|OAuth Flow| STRAVA_API
    HTTP2 -->|OAuth Flow| FITBIT_API

    style CLIENT fill:#e3f2fd
    style AGENT fill:#f3e5f5
    style CDN fill:#fff3e0
    style ALB fill:#e8f5e8
    style REDIS fill:#ffebee
    style DB fill:#f1f8e9
    style STRAVA_API fill:#fc4c02
    style FITBIT_API fill:#00b0b9
```

## Architecture Decision Records (ADR)

### ADR-001: Multi-Protocol Support
**Decision**: Support both MCP and A2A protocols in a single server instance.
**Rationale**: Reduces operational complexity while allowing different client types.
**Consequences**: Shared authentication and tenant isolation across protocols.

### ADR-002: Plugin-Based Database Architecture
**Decision**: Use a plugin architecture for database backends.
**Rationale**: Supports different deployment scenarios (SQLite for dev, PostgreSQL for prod).
**Consequences**: Abstracts database operations behind a common interface.

### ADR-003: JWT + API Key Dual Authentication
**Decision**: Support both JWT tokens and API keys for authentication.
**Rationale**: JWTs for web sessions, API keys for programmatic access.
**Consequences**: Dual validation paths in authentication middleware.

### ADR-004: Redis for Rate Limiting
**Decision**: Use Redis for distributed rate limiting state.
**Rationale**: Required for multi-instance deployments with consistent rate limits.
**Consequences**: External dependency but enables horizontal scaling.

### ADR-005: Tenant-Scoped OAuth Configurations
**Decision**: Allow per-tenant OAuth application configurations.
**Rationale**: Enterprise customers need their own OAuth apps for branding/compliance.
**Consequences**: Complex OAuth flow handling but supports B2B requirements.

## Performance Characteristics

### Throughput Metrics
- **WebSocket Connections**: 10,000+ concurrent connections per instance
- **HTTP Requests**: 1,000+ requests/second per instance
- **Database Operations**: 5,000+ queries/second with connection pooling
- **Rate Limiting**: Sub-millisecond token bucket operations via Redis

### Latency Metrics
- **MCP Tool Calls**: <100ms average (excluding external API calls)
- **Authentication**: <10ms for JWT validation, <50ms for API key lookup
- **Database Queries**: <5ms average with proper indexing
- **OAuth Token Refresh**: <500ms including external API round-trip

### Scalability Limits
- **Horizontal Scaling**: Stateless application design supports unlimited instances
- **Database Scaling**: Read replicas for analytics, primary for writes
- **Cache Scaling**: Redis cluster for distributed rate limiting state
- **External API Limits**: Bounded by provider rate limits (Strava/Garmin/Fitbit: 600 requests/15min)

These architecture diagrams provide a comprehensive view of how Pierre MCP Server is structured to handle multi-protocol communication, maintain tenant isolation, and scale horizontally while preserving security and performance characteristics.