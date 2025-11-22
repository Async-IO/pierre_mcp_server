# Pierre Fitness Platform - Comprehensive Rust Tutorial

> **Target Audience**: Junior Rust developers with 6-12 months experience
>
> **Prerequisites**: Basic knowledge of ownership, async/await, traits, and error handling
>
> **Estimated Duration**: 60-80 hours of learning content

---

## Part I: Foundation & Project Structure

1. [Chapter 1: Project Architecture & Module Organization](tutorial/chapter-01-project-architecture.md)
2. [Chapter 2: Error Handling & Type-Safe Errors](tutorial/chapter-02-error-handling.md)
3. [Chapter 3: Configuration Management & Environment Variables](tutorial/chapter-03-configuration.md)
4. [Chapter 3.5: Database Architecture & Abstraction Layer](tutorial/chapter-03.5-database-architecture.md)
5. [Chapter 4: Dependency Injection with Context Pattern](tutorial/chapter-04-dependency-injection.md)

## Part II: Authentication & Security

6. [Chapter 5: Cryptographic Key Management](tutorial/chapter-05-cryptographic-keys.md)
7. [Chapter 6: JWT Authentication with RS256](tutorial/chapter-06-jwt-authentication.md)
8. [Chapter 7: Multi-Tenant Database Isolation](tutorial/chapter-07-multi-tenant-isolation.md)
9. [Chapter 8: Middleware & Request Context](tutorial/chapter-08-middleware-context.md)

## Part III: MCP Protocol Implementation

10. [Chapter 9: JSON-RPC 2.0 Foundation](tutorial/chapter-09-jsonrpc-foundation.md)
11. [Chapter 10: MCP Protocol Deep Dive - Request Flow](tutorial/chapter-10-mcp-request-flow.md)
12. [Chapter 11: MCP Transport Layers (HTTP, stdio, WebSocket, SSE)](tutorial/chapter-11-mcp-transport-layers.md)
13. [Chapter 12: MCP Tool Registry & Type-Safe Routing](tutorial/chapter-12-mcp-tool-registry.md)

## Part IV: SDK & Type System

14. [Chapter 13: SDK Bridge Architecture & stdio Transport](tutorial/chapter-13-sdk-bridge-architecture.md)
15. [Chapter 14: Type Generation & Tools-to-Types System](tutorial/chapter-14-type-generation.md)

## Part V: OAuth 2.0, A2A & Provider Integration

16. [Chapter 15: OAuth 2.0 Server Implementation (RFC 7591)](tutorial/chapter-15-oauth-server.md)
17. [Chapter 16: OAuth 2.0 Client for Fitness Providers](tutorial/chapter-16-oauth-client.md)
18. [Chapter 17: Provider Data Models & Rate Limiting](tutorial/chapter-17-provider-models.md)
19. **[Chapter 17.5: Pluggable Provider Architecture (1 to x Providers)](tutorial/chapter-17.5-pluggable-providers.md)**
20. [Chapter 18: A2A Protocol - Agent-to-Agent Communication](tutorial/chapter-18-a2a-protocol.md)

## Part VI: Tools & Intelligence System

21. [Chapter 19: Comprehensive Tools Guide - All 35+ MCP Tools](tutorial/chapter-19-tools-guide.md)
22. [Chapter 20: Sports Science Algorithms & Intelligence](tutorial/chapter-20-sports-science.md)
23. [Chapter 21: Training Load, Recovery & Sleep Analysis](tutorial/chapter-21-recovery-sleep.md)
24. [Chapter 22: Nutrition System & USDA Integration](tutorial/chapter-22-nutrition.md)

## Part VII: Testing, Design & Deployment

25. [Chapter 23: Testing Framework - Synthetic Data & E2E Tests](tutorial/chapter-23-testing.md)
26. [Chapter 24: Design System - Templates, Frontend & User Experience](tutorial/chapter-24-design-system.md)
27. [Chapter 25: Production Deployment, Clippy & Performance](tutorial/chapter-25-deployment.md)

---

## Appendices

- [Appendix A: Rust Idioms Reference](tutorial/appendix-a-rust-idioms.md)
- [Appendix B: CLAUDE.md Compliance Checklist](tutorial/appendix-b-claude-md.md)
- [Appendix C: Pierre Codebase Map](tutorial/appendix-c-codebase-map.md)
- [Appendix D: Natural Language to Tool Mapping](tutorial/appendix-d-tool-mapping.md)

---

## Learning Paths

### Quick Start Path (Core Concepts)
For developers who need to get productive quickly:
1. Chapter 1 (Architecture)
2. Chapter 2 (Error Handling)
3. Chapter 9 (JSON-RPC)
4. Chapter 10 (MCP Protocol)
5. Chapter 19 (Tools Guide)

### Security-Focused Path
For developers working on authentication and security:
1. Chapter 5 (Cryptographic Keys)
2. Chapter 6 (JWT Authentication)
3. Chapter 7 (Multi-Tenant Isolation)
4. Chapter 8 (Middleware)
5. Chapter 15 (OAuth 2.0 Server)

### Full Stack Path
Complete tutorial from start to finish - recommended for thorough understanding.

---

## Summary

**Total**: 25 Chapters + 1 Database Architecture Chapter + 4 Appendices
**Focus**: Rust idioms, real code examples, progressive complexity
**Code Citations**: All examples reference actual Pierre codebase with file:line numbers
