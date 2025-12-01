# Pierre MCP Server Dashboard

A web dashboard for managing and monitoring Pierre MCP Server. Built with React, TypeScript, and Vite.

## Features

- **Dashboard Overview**: API key usage statistics and system metrics
- **User Management**: User approval and tenant management
- **Rate Limiting**: Monitor and configure API rate limits
- **A2A Monitoring**: Agent-to-Agent communication tracking
- **Real-time Updates**: WebSocket-based live data updates
- **Usage Analytics**: Request patterns and tool usage breakdown

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

### Build

```bash
npm run build
```

### Testing

```bash
# Run tests
npm test

# Run tests with UI
npm test:ui

# Run tests with coverage
npm test:coverage
```

### Linting

```bash
npm run lint
npm run type-check
```

### Demo Data

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
- **20 API keys** - Fitness-focused (Strava Sync, Garmin Connect, Apple Health, Training Plan Bot, etc.)
- **10 A2A clients** - AI assistants and integration bots (Claude Desktop, GPT-4 Fitness Coach, Slack Bot, etc.)
- **10 admin tokens** - Service tokens (CI/CD Pipeline, API Gateway, Monitoring Service, etc.)
- **50,000+ API usage records** - 30 days of usage data with realistic patterns
- **4,500+ request logs** - 14 days of endpoint access logs

## Architecture

- **React 19** with functional components and hooks
- **TypeScript** for type safety
- **TailwindCSS** for styling
- **React Query** for data fetching and caching
- **Chart.js** for analytics visualizations
- **WebSocket** integration for real-time updates
- **Vite** for development and building
