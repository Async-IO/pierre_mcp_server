# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. Please report security issues responsibly.

### How to Report

1. **Email**: Send details to the maintainers via GitHub security advisories
2. **GitHub Security Advisories**: Use the "Security" tab to report privately

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: Within 24-48 hours
  - High: Within 7 days
  - Medium: Within 30 days
  - Low: Next release cycle

### What to Expect

- Acknowledgment of your report
- Regular updates on progress
- Credit in release notes (unless you prefer anonymity)
- Notification when the fix is released

## Security Best Practices

When deploying Pierre Fitness Platform:

1. **Use HTTPS** in production
2. **Rotate JWT tokens** regularly
3. **Set strong encryption keys** (`PIERRE_MASTER_ENCRYPTION_KEY`)
4. **Limit OAuth scopes** to minimum required
5. **Enable rate limiting** for all endpoints
6. **Keep dependencies updated** (`cargo update`, `npm update`)
7. **Review audit logs** regularly

## Security Features

Pierre includes:

- RS256 asymmetric JWT signing (4096-bit keys)
- AES-256-GCM encryption for stored tokens
- PKCE for all OAuth flows
- Rate limiting per tenant
- CSRF protection for web applications
- Atomic token operations (TOCTOU prevention)
