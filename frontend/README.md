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

## Architecture

- **React 19** with functional components and hooks
- **TypeScript** for type safety
- **TailwindCSS** for styling
- **React Query** for data fetching and caching
- **Chart.js** for analytics visualizations
- **WebSocket** integration for real-time updates
- **Vite** for development and building
