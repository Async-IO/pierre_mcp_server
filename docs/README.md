# Pierre MCP Server Documentation

Complete documentation for the Pierre MCP Server platform.

## Documentation Index

### **Getting Started**
- [**Getting Started Guide**](getting-started.md) - Complete setup guide from development to production deployment

### **API References**
- [**API Reference**](developer-guide/14-api-reference.md) - Complete API documentation including MCP tools, endpoints, error handling, and integration examples
- [**A2A Quick Start**](A2A_QUICK_START.md) - Quick 5-minute guide to get started with A2A protocol
- [**A2A Reference**](developer-guide/05-a2a-protocol.md) - Complete A2A protocol reference and implementation guide
- [**OpenAPI Specification**](openapi.yaml) - Complete API reference

### **Deployment & Operations**
- [**Deployment Guide**](DEPLOYMENT_GUIDE.md) - Production deployment, architecture, testing, and provisioning

### **Database & Storage**
- [**Database Guide**](database.md) - Setup, schema, encryption, and management

## Quick Navigation

### For Developers
1. **Quick Start**: Use automated setup script in [Getting Started Guide](getting-started.md)
2. **Production Setup**: Follow production path in [Getting Started Guide](getting-started.md)  
3. **AI Integration**: See [API Reference](developer-guide/14-api-reference.md) for MCP tools and prompt examples
4. **Web Development**: Check [Getting Started Guide](getting-started.md) for authentication and [API Reference](developer-guide/14-api-reference.md) for endpoints

### For Production Use
1. **Integration Planning**: Review [Deployment Guide](DEPLOYMENT_GUIDE.md) for architecture
2. **Production Deploy**: Follow [Deployment Guide](DEPLOYMENT_GUIDE.md)

### For Support
1. **Troubleshooting**: Check [API Reference](developer-guide/14-api-reference.md) for error handling
2. **Configuration Issues**: See [Getting Started Guide](getting-started.md)
3. **API Problems**: Use [OpenAPI Specification](openapi.yaml)

### Architecture Notes
- **OAuth Configuration**: OAuth credentials are stored per-tenant in the database (not environment variables)
- **User Authentication**: Users must be approved by admin before accessing the system
- **MCP Protocol**: Available on port 8080 with JWT authentication
- **A2A Protocol**: Enterprise integration protocol on port 8081

## Reading Guide

### First Time Setup
1. [Getting Started Guide](getting-started.md) - Complete setup from development to production
2. [API Reference](developer-guide/14-api-reference.md) - Understand available tools and endpoints
3. [Database Guide](database.md) - Database configuration and management

### Integration Development
1. [OpenAPI Specification](openapi.yaml) - Complete API reference
2. [A2A Quick Start](A2A_QUICK_START.md) - For agent-to-agent integration
3. [API Reference](developer-guide/14-api-reference.md) - MCP tools and REST endpoints

### Production Deployment
1. [Deployment Guide](DEPLOYMENT_GUIDE.md) - Deploy to production
2. [Database Guide](database.md) - Database configuration
3. [A2A Reference](developer-guide/05-a2a-protocol.md) - For enterprise integrations

## Getting Help

- **Setup Issues**: Check [Getting Started Guide](getting-started.md)
- **API Errors**: See [API Reference](developer-guide/14-api-reference.md) error handling section
- **Database Issues**: Review [Database Guide](database.md) and [Database Cleanup](DATABASE_CLEANUP.md)
- **Integration**: Use [OpenAPI Specification](openapi.yaml) and [A2A Reference](developer-guide/05-a2a-protocol.md)

---

*All documentation is up-to-date as of the latest release. For the most current API reference, always check the [OpenAPI specification](openapi.yaml).*