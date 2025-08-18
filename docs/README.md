# Pierre MCP Server Documentation

Complete documentation for the Pierre MCP Server platform.

## Documentation Index

### **Getting Started**
- [**Getting Started Guide**](GETTING_STARTED.md) - Complete setup, configuration, and authentication guide

### **API References**
- [**API Reference**](API_REFERENCE.md) - Complete API documentation including MCP tools, endpoints, error handling, and integration examples
- [**A2A Quick Start**](A2A_QUICK_START.md) - Quick 5-minute guide to get started with A2A protocol
- [**A2A Reference**](A2A_REFERENCE.md) - Complete A2A protocol reference and implementation guide
- [**OpenAPI Specification**](openapi.yaml) - Complete API reference

### **Deployment & Operations**
- [**Deployment Guide**](DEPLOYMENT_GUIDE.md) - Production deployment, architecture, testing, and provisioning

### **Database & Storage**
- [**Database Guide**](DATABASE_GUIDE.md) - SQLite/PostgreSQL setup and database plugins architecture
- [**Database Cleanup Guide**](DATABASE_CLEANUP.md) - How to clean databases for fresh starts, troubleshooting, and testing

## Quick Navigation

### For Developers
1. **Personal Use**: Start with [Getting Started Guide](GETTING_STARTED.md)
2. **AI Integration**: See [API Reference](API_REFERENCE.md) for MCP tools and prompt examples
3. **Web Development**: Check [Getting Started Guide](GETTING_STARTED.md) for authentication and [API Reference](API_REFERENCE.md) for endpoints

### For Production Use
1. **Integration Planning**: Review [Deployment Guide](DEPLOYMENT_GUIDE.md) for architecture
2. **Production Deploy**: Follow [Deployment Guide](DEPLOYMENT_GUIDE.md)

### For Support
1. **Troubleshooting**: Check [API Reference](API_REFERENCE.md) for error handling
2. **Configuration Issues**: See [Getting Started Guide](GETTING_STARTED.md)
3. **API Problems**: Use [OpenAPI Specification](openapi.yaml)

## What's New

### Latest Updates
- Complete A2A protocol support in OpenAPI specification
- Enhanced authentication guide with JWT details and A2A flows
- Comprehensive error reference with troubleshooting
- Updated Python examples with working A2A authentication
- Consolidated documentation (removed outdated files)

### Architecture Notes
- **OAuth Configuration**: OAuth credentials are stored per-tenant in the database (not environment variables)
- **User Authentication**: Users must be approved by admin before accessing the system
- **MCP Protocol**: Available on port 8080 with JWT authentication
- **A2A Protocol**: Enterprise integration protocol on port 8081

## Reading Guide

### First Time Setup
1. [Getting Started Guide](GETTING_STARTED.md) - Get the server running
2. [API Reference](API_REFERENCE.md) - Understand available tools and endpoints
3. [Database Guide](DATABASE_GUIDE.md) - Set up your database

### Integration Development
1. [OpenAPI Specification](openapi.yaml) - Complete API reference
2. [A2A Quick Start](A2A_QUICK_START.md) - For agent-to-agent integration
3. [API Reference](API_REFERENCE.md) - MCP tools and REST endpoints

### Production Deployment
1. [Deployment Guide](DEPLOYMENT_GUIDE.md) - Deploy to production
2. [Database Guide](DATABASE_GUIDE.md) - Database configuration
3. [A2A Reference](A2A_REFERENCE.md) - For enterprise integrations

## Getting Help

- **Setup Issues**: Check [Getting Started Guide](GETTING_STARTED.md)
- **API Errors**: See [API Reference](API_REFERENCE.md) error handling section
- **Database Issues**: Review [Database Guide](DATABASE_GUIDE.md) and [Database Cleanup](DATABASE_CLEANUP.md)
- **Integration**: Use [OpenAPI Specification](openapi.yaml) and [A2A Reference](A2A_REFERENCE.md)

---

*All documentation is up-to-date as of the latest release. For the most current API reference, always check the [OpenAPI specification](openapi.yaml).*