<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 24: Design System - Frontend Dashboard, Templates & UX

This chapter covers Pierre's design system including the React admin dashboard, OAuth templates, brand identity, and user experience patterns for fitness data visualization.

## What You'll Learn

- Admin dashboard architecture (React/TypeScript)
- Component organization and lazy loading
- State management (React Query + WebSocket)
- Brand identity and color system
- OAuth success/error templates
- Real-time data visualization

## Frontend Admin Dashboard

Pierre includes a full-featured React admin dashboard for server management.

**Technology stack**:
- React 19 with TypeScript
- TailwindCSS for styling
- React Query for data fetching/caching
- Chart.js for analytics visualization
- WebSocket for real-time updates
- Vite for development/building

```
frontend/
├── src/
│   ├── App.tsx              # Main application
│   ├── services/api.ts      # Axios API client
│   ├── contexts/            # React contexts
│   │   ├── AuthContext.tsx  # Auth state
│   │   └── WebSocketProvider.tsx  # Real-time updates
│   ├── hooks/               # Custom hooks
│   │   ├── useAuth.ts       # Auth hook
│   │   └── useWebSocket.ts  # WebSocket hook
│   └── components/          # UI components (20+)
│       ├── Dashboard.tsx    # Main dashboard
│       ├── UserManagement.tsx
│       ├── A2AManagement.tsx
│       └── ...
├── tailwind.config.js       # Brand colors
└── BRAND.md                 # Design system docs
```

## Dashboard Architecture

The dashboard uses lazy loading for performance optimization:

**Source**: frontend/src/components/Dashboard.tsx:12-18
```typescript
// Lazy load heavy components to reduce initial bundle size
const OverviewTab = lazy(() => import('./OverviewTab'));
const UsageAnalytics = lazy(() => import('./UsageAnalytics'));
const RequestMonitor = lazy(() => import('./RequestMonitor'));
const ToolUsageBreakdown = lazy(() => import('./ToolUsageBreakdown'));
const UnifiedConnections = lazy(() => import('./UnifiedConnections'));
const UserManagement = lazy(() => import('./UserManagement'));
```

**Dashboard tabs**:
- **Overview**: System metrics, API key usage, health status
- **Analytics**: Request patterns, tool usage breakdown, trends
- **Connections**: Provider OAuth status, user connections
- **Users**: User approval, tenant management (admin only)
- **A2A**: Agent-to-Agent monitoring, client registration

## API Service Layer

The API service handles CSRF protection and auth token management:

**Source**: frontend/src/services/api.ts:7-33
```typescript
class ApiService {
  private csrfToken: string | null = null;

  constructor() {
    axios.defaults.baseURL = API_BASE_URL;
    axios.defaults.withCredentials = true;
    this.setupInterceptors();
  }

  private setupInterceptors() {
    // Add CSRF token for state-changing operations
    axios.interceptors.request.use((config) => {
      if (this.csrfToken && ['POST', 'PUT', 'DELETE'].includes(config.method?.toUpperCase() || '')) {
        config.headers['X-CSRF-Token'] = this.csrfToken;
      }
      return config;
    });

    // Handle 401 errors (trigger logout)
    axios.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.status === 401) {
          this.handleAuthFailure();
        }
        return Promise.reject(error);
      }
    );
  }
}
```

## Real-Time Updates (WebSocket)

The dashboard receives live updates via WebSocket:

**Source**: frontend/src/components/Dashboard.tsx:76-80
```typescript
// Refresh data when WebSocket updates are received
useEffect(() => {
  if (lastMessage) {
    if (lastMessage.type === 'usage_update' || lastMessage.type === 'system_stats') {
      refetchOverview();
    }
  }
}, [lastMessage, refetchOverview]);
```

**WebSocket message types**:
- `usage_update`: API usage metrics changed
- `system_stats`: System health metrics updated
- `connection_status`: Provider connection changed
- `user_approved`: User approval status changed

## Brand Identity

Pierre uses a "Three Pillars" design system representing holistic fitness:

**Source**: frontend/BRAND.md:15-20
```
| Pillar     | Color   | Hex       | Usage                        |
|------------|---------|-----------|------------------------------|
| Activity   | Emerald | #10B981   | Movement, fitness, energy    |
| Nutrition  | Amber   | #F59E0B   | Food, fuel, nourishment      |
| Recovery   | Indigo  | #6366F1   | Rest, sleep, restoration     |
```

**Primary brand colors**:
- Pierre Violet (`#7C3AED`): Intelligence, AI, sophistication
- Pierre Cyan (`#06B6D4`): Data flow, connectivity, freshness

**TailwindCSS classes**:
```html
<!-- Primary colors -->
<div class="bg-pierre-violet">Intelligence</div>
<div class="bg-pierre-cyan">Data Flow</div>

<!-- Three Pillars -->
<Badge class="bg-pierre-activity">Running</Badge>
<Badge class="bg-pierre-nutrition">Calories</Badge>
<Badge class="bg-pierre-recovery">Sleep</Badge>
```

## OAuth Templates

Pierre uses HTML templates for OAuth callback pages.

**OAuth success template** (templates/oauth_success.html):
```html
<!DOCTYPE html>
<html>
<head>
    <title>OAuth Success - Pierre Fitness</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        .container {
            background: white;
            padding: 40px;
            border-radius: 12px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
            text-align: center;
        }
        h1 { color: #667eea; }
        .success-icon { font-size: 64px; color: #10b981; }
    </style>
</head>
<body>
    <div class="container">
        <div class="success-icon">✓</div>
        <h1>Successfully Connected to {{PROVIDER}}</h1>
        <p>You can now close this window and return to the app.</p>
        <p>User ID: {{USER_ID}}</p>
    </div>
</body>
</html>
```

**Template rendering**:

**Source**: src/oauth2_client/flow_manager.rs:11-26
```rust
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    pub fn render_success_template(
        provider: &str,
        callback_response: &OAuthCallbackResponse,
    ) -> Result<String, Box<dyn std::error::Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{USER_ID}}", &callback_response.user_id);

        Ok(rendered)
    }
}
```

## Dashboard Components

### Overview Tab
Displays system health, API key statistics, and quick metrics.

### Usage Analytics
Chart.js visualizations for request patterns over time.

### Request Monitor
Real-time feed of API requests with filtering and search.

### Tool Usage Breakdown
Pie charts and tables showing which MCP tools are most used.

### User Management (Admin)
- Approve/reject pending user registrations
- View user activity and connections
- Manage tenant assignments

### A2A Management
- Register new A2A clients
- Monitor agent-to-agent communications
- View capability discovery logs

## Development Setup

```bash
# Install dependencies
cd frontend
npm install

# Start development server (with Vite proxy to backend)
npm run dev

# Build for production
npm run build

# Run tests
npm test

# Type checking
npm run type-check
```

**Vite proxy configuration** (vite.config.ts):
```typescript
export default defineConfig({
  server: {
    proxy: {
      '/api': 'http://localhost:8081',
      '/ws': {
        target: 'ws://localhost:8081',
        ws: true,
      },
    },
  },
});
```

## Key Takeaways

1. **React admin dashboard**: Full-featured dashboard with 20+ components for server management.
2. **Lazy loading**: Heavy components loaded on-demand for fast initial page load.
3. **React Query**: Server state management with automatic caching and refetching.
4. **WebSocket**: Real-time updates for live metrics and status changes.
5. **Three Pillars**: Activity (emerald), Nutrition (amber), Recovery (indigo) color system.
6. **OAuth templates**: HTML templates with `{{PLACEHOLDER}}` substitution for success/error pages.
7. **CSRF protection**: API service automatically adds CSRF tokens to state-changing requests.
8. **TailwindCSS**: Brand colors available as `pierre-*` utility classes.

---

**Next Chapter**: [Chapter 25: Production Deployment, Clippy & Performance](./chapter-25-deployment.md) - Learn about production deployment strategies, Clippy lint configuration, performance optimization, and monitoring.
