<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Introduction

Pierre Fitness Platform connects AI assistants to fitness data from Strava, Garmin, Fitbit, WHOOP, COROS, and Terra (150+ wearables). It implements Model Context Protocol (MCP), A2A protocol, OAuth 2.0, and REST APIs for Claude, ChatGPT, and other AI assistants.

## Core Features

- **MCP Protocol**: JSON-RPC 2.0 for AI assistant integration
- **A2A Protocol**: Agent-to-agent communication
- **OAuth 2.0 Server**: RFC 7591 dynamic client registration
- **53 MCP Tools**: Activities, goals, analysis, sleep, recovery, nutrition, recipes, mobility
- **TypeScript SDK**: `pierre-mcp-client` npm package
- **Pluggable Providers**: Compile-time provider selection
- **TOON Format**: Token-Oriented Object Notation for ~40% LLM token reduction

## Intelligence System

Sports science-based fitness analysis including:
- Training load management (ATL, CTL, TSB)
- Race predictions (VDOT-based)
- Sleep and recovery scoring
- Nutrition planning
- Pattern detection

## Provider Support

| Provider | Capabilities |
|----------|--------------|
| Strava | Activities, Stats, Routes |
| Garmin | Activities, Sleep, Health |
| WHOOP | Sleep, Recovery, Strain |
| Fitbit | Activities, Sleep, Health |
| COROS | Activities, Sleep, Recovery |
| Terra | 150+ wearables, Activities, Sleep, Health |
| Synthetic | Development/Testing |

## Documentation Structure

This documentation is organized into sections:

- **Getting Started**: Installation and initial setup
- **Core Concepts**: Architecture, protocols, and authentication
- **Intelligence**: Sports science methodologies
- **API Reference**: Tools, prompts, OAuth, and LLM providers
- **Development**: Contributing and building Pierre

## Quick Links

- [Getting Started](getting-started.md) - Set up your development environment
- [Architecture](architecture.md) - System design overview
- [Tools Reference](tools-reference.md) - All 53 MCP tools
- [Testing](testing.md) - Testing strategy and practices
