# Pierre MCP Server Dashboard

A web dashboard for managing and monitoring Pierre MCP Server. Built with React, TypeScript, and Vite.

## Features

- **Dashboard Overview**: API key usage statistics and system metrics
- **User Management**: User approval, registration, and tenant management
- **Connections**: A2A clients and API Keys management
- **MCP Tokens**: Token generation and management for MCP protocol
- **Rate Limiting**: Monitor and configure API rate limits
- **A2A Monitoring**: Agent-to-Agent communication tracking
- **Real-time Updates**: WebSocket-based live data updates
- **Usage Analytics**: Request patterns and tool usage breakdown
- **Role-based Access**: Admin impersonation and permission controls

## User Roles

The dashboard supports two user roles with distinct permissions:

### Regular User
| Feature | Access |
|---------|--------|
| Dashboard Overview | View own statistics |
| API Keys | Create, view, deactivate own keys |
| Connected Apps (A2A) | Register, manage own A2A clients |
| MCP Tokens | Generate tokens for own use |
| Analytics | View own usage data |
| Request Monitor | View own request logs |
| Settings | Manage own profile |

### Admin User
| Feature | Access |
|---------|--------|
| All Regular User features | Full access |
| User Management | Approve/reject registrations, suspend users |
| API Keys | Create system-wide API keys |
| User Impersonation | View dashboard as any user |
| All Users Analytics | View platform-wide statistics |
| Tenant Management | Manage multi-tenant configuration |

## User Onboarding Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐     ┌───────────┐
│  Register   │────▶│   Pending    │────▶│ Admin Approves  │────▶│  Active   │
│  (Sign Up)  │     │  Approval    │     │   Registration  │     │   User    │
└─────────────┘     └──────────────┘     └─────────────────┘     └───────────┘
                           │                                            │
                           │ Admin Rejects                              │
                           ▼                                            ▼
                    ┌──────────────┐                            ┌───────────────┐
                    │   Rejected   │                            │   Dashboard   │
                    │    (End)     │                            │    Access     │
                    └──────────────┘                            └───────────────┘
```

### 1. Registration
- User visits `/register` and submits registration form
- Required fields: name, email, password
- Account is created with `pending_approval` status

### 2. Pending Approval
- User sees "Pending Approval" screen after login
- Cannot access dashboard features until approved
- Admin receives notification of pending registration

### 3. Admin Approval
- Admin views pending users in User Management tab
- Can approve or reject each registration
- Approved users gain full dashboard access

### 4. Active User
- Full access to dashboard based on role (user/admin)
- Can create API keys, register A2A clients, generate MCP tokens
- Access to analytics and monitoring features

### Creating the First Admin User

The first admin user must be created via CLI before the dashboard can be used:

```bash
# From repository root
cargo run --bin admin-setup -- create-admin-user \
  --email admin@example.com \
  --password SecurePassword123 \
  --name "Admin User" \
  --super-admin
```

This creates a super-admin who can then approve other user registrations and grant admin privileges.

## Dashboard Tabs

| Tab | Description | User Access | Admin Access |
|-----|-------------|-------------|--------------|
| **Home** | Overview statistics, quick actions | Own data | Platform-wide |
| **Connections** | A2A clients | Own resources | + API Keys tab |
| **MCP Tokens** | Generate MCP protocol tokens | Own tokens | Own tokens |
| **Analytics** | Usage charts, trends | Own analytics | All users |
| **Monitor** | Request logs, tool usage | Own requests | All requests |
| **Tools** | Available MCP tools | View only | View only |
| **User Management** | User approval, status | Hidden | Full access |
| **Settings** | Profile, preferences | Own profile | + System settings |

## Tech Stack

### Core Framework
| Technology | Version | Purpose |
|------------|---------|---------|
| React | 19.1.0 | UI framework with functional components and hooks |
| TypeScript | 5.8.3 | Type safety and developer experience |
| Vite | 6.4.1 | Development server and build tooling |

### Styling & UI
| Technology | Version | Purpose |
|------------|---------|---------|
| TailwindCSS | 3.4.17 | Utility-first CSS framework |
| @tailwindcss/forms | 0.5.10 | Form element styling |
| clsx | 2.1.1 | Conditional className utility |

### Data & State Management
| Technology | Version | Purpose |
|------------|---------|---------|
| @tanstack/react-query | 5.80.7 | Server state management and caching |
| Axios | 1.12.0 | HTTP client for API requests |

### Visualization
| Technology | Version | Purpose |
|------------|---------|---------|
| Chart.js | 4.4.9 | Analytics charts and graphs |
| react-chartjs-2 | 5.3.0 | React wrapper for Chart.js |
| date-fns | 4.1.0 | Date formatting and manipulation |

### Testing
| Technology | Version | Purpose |
|------------|---------|---------|
| Vitest | 3.2.3 | Unit and integration testing |
| @testing-library/react | 16.3.0 | React component testing utilities |
| Playwright | 1.57.0 | End-to-end browser testing |

### Code Quality
| Technology | Version | Purpose |
|------------|---------|---------|
| ESLint | 9.25.0 | Code linting |
| typescript-eslint | 8.30.1 | TypeScript-specific linting rules |

## Development

### Prerequisites

- Node.js 18+
- Pierre MCP Server running on localhost:8081

### Setup

```bash
cd frontend
npm install
npm run dev
```

The development server runs at http://localhost:5173

### Build

```bash
npm run build
```

Production build outputs to `dist/` directory.

## Testing

### Unit Tests (Vitest)

```bash
# Run tests in watch mode
npm test

# Run tests with UI
npm run test:ui

# Run tests with coverage
npm run test:coverage
```

### E2E Tests (Playwright)

The E2E test suite covers **294 tests** across 14 spec files:

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `login.spec.ts` | 14 | Authentication flows |
| `registration.spec.ts` | 14 | User registration |
| `pending-approval.spec.ts` | 10 | Approval workflow |
| `dashboard.spec.ts` | 21 | Main dashboard |
| `overview.spec.ts` | 45 | Overview tab |
| `connections.spec.ts` | 37 | A2A clients, API Keys |
| `admin-tokens.spec.ts` | 23 | API Key management |
| `admin-config.spec.ts` | 23 | Admin configuration |
| `analytics.spec.ts` | 14 | Usage analytics |
| `monitor.spec.ts` | 25 | Request monitoring |
| `tools.spec.ts` | 27 | MCP tools display |
| `prompts.spec.ts` | 17 | Prompt management |
| `user-management.spec.ts` | 14 | User admin functions |
| `impersonation.spec.ts` | 10 | Admin impersonation |

```bash
# Run all E2E tests
npm run test:e2e

# Run with Playwright UI
npm run test:e2e:ui

# Run in headed mode (visible browser)
npm run test:e2e:headed

# Run specific test file
npx playwright test e2e/connections.spec.ts

# Run tests matching pattern
npx playwright test --grep "API Key"
```

### Test Architecture

E2E tests use **mocked API responses** via Playwright route interception. This approach:
- Provides fast, deterministic tests
- Eliminates backend dependencies during frontend testing
- Allows testing of error states and edge cases

Shared test utilities are in `e2e/test-helpers.ts`:
- `setupDashboardMocks(page, options)` - Common API mocks
- `loginToDashboard(page)` - Authentication helper
- `navigateToTab(page, tabName)` - Navigation helper

### Linting

```bash
npm run lint
npm run type-check
```

## Demo Data

To populate the dashboard with realistic demo data for visualization and testing:

```bash
# From repository root - clear database first
./scripts/fresh-start.sh

# Start server to create tables
# Then in another terminal:
./scripts/seed-demo-data.sh
```

The seed script creates:
- **18 demo users** - Mix of tiers (starter/professional/enterprise) and statuses (active/pending/suspended)
- **20 MCP tokens** - User tokens for AI clients (Claude Desktop, Cursor IDE, etc.)
- **10 A2A clients** - AI assistants and integration bots (Claude Desktop, GPT-4 Fitness Coach, Slack Bot, etc.)
- **10 Admin API Keys** - Service tokens (CI/CD Pipeline, API Gateway, Monitoring Service, etc.)
- **50,000+ API usage records** - 30 days of usage data with realistic patterns
- **4,500+ request logs** - 14 days of endpoint access logs

## Project Structure

```
frontend/
├── src/
│   ├── components/     # React components
│   ├── contexts/       # React context providers
│   ├── services/       # API service layer
│   ├── types/          # TypeScript type definitions
│   └── App.tsx         # Root component
├── e2e/
│   ├── *.spec.ts       # Playwright test files
│   └── test-helpers.ts # Shared test utilities
├── public/             # Static assets
└── dist/               # Production build output
```

## API Integration

The frontend communicates with Pierre MCP Server via REST API:

- **Base URL**: `http://localhost:8081` (development)
- **Authentication**: JWT tokens stored in localStorage
- **API Service**: `src/services/api.ts`

Key endpoints:
- `/api/auth/*` - Authentication
- `/api/keys/*` - API key management
- `/api/admin/*` - Admin operations
- `/api/dashboard/*` - Dashboard data
- `/a2a/*` - A2A client management
- `/mcp/*` - MCP token operations
