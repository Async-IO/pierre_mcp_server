# Check for Secrets Skill

## Purpose
Scans codebase for accidentally committed secrets, credentials, API keys, and sensitive data. Prevents catastrophic security breaches.

## CLAUDE.md Compliance
- ✅ Enforces no hardcoded secrets
- ✅ Validates environment variable usage
- ✅ Checks git history for leaked credentials
- ✅ Security-critical validation

## Usage
Run this skill:
- Before every commit
- Before pull requests
- After adding new integrations
- Weekly security scans
- Before production deployments

## Prerequisites
- ripgrep (`rg`)
- git

## Commands

### Quick Secret Scan
```bash
# Run automated secret detection
./scripts/validate-no-secrets.sh
```

### Comprehensive Secret Detection
```bash
# 1. Check for API keys
echo "🔑 Checking for API keys..."
rg -i "api[_-]?key.*=.*['\"][a-zA-Z0-9]{20,}" src/ --type rust -n

# 2. Check for passwords
echo "🔒 Checking for hardcoded passwords..."
rg -i "password.*=.*['\"][^'\"]{8,}" src/ --type rust -n | grep -v "example"

# 3. Check for tokens
echo "🎫 Checking for access tokens..."
rg -i "token.*=.*['\"][a-zA-Z0-9]{40,}" src/ --type rust -n

# 4. Check for database URLs
echo "🗄️ Checking for database URLs..."
rg "postgres://|mysql://|mongodb://" src/ --type rust -n

# 5. Check for OAuth secrets
echo "🔐 Checking for OAuth client secrets..."
rg "client_secret.*=.*['\"]" src/ --type rust -n | grep -v "env\|config"

# 6. Check for encryption keys
echo "🔓 Checking for hardcoded encryption keys..."
rg "const.*KEY.*=.*['\"][A-Za-z0-9+/=]{32,}" src/ --type rust -n

# 7. Check for AWS credentials
echo "☁️ Checking for AWS credentials..."
rg "AKIA[0-9A-Z]{16}" . -n

# 8. Check for private keys
echo "🗝️ Checking for private keys..."
rg "BEGIN.*PRIVATE.*KEY|BEGIN RSA PRIVATE KEY" . -n
```

### Environment File Checks
```bash
# Check .env is not tracked
echo "📋 Checking .env files..."
git ls-files | rg "\.env$" && \
  echo "❌ .env file tracked in git!" || \
  echo "✓ No .env in git"

# Verify .env in .gitignore
grep -q "^\.env$" .gitignore && \
  echo "✓ .env in .gitignore" || \
  echo "⚠️  Add .env to .gitignore"

# Check for committed .env files
find . -name ".env" -type f | while read env_file; do
    if git ls-files --error-unmatch "$env_file" 2>/dev/null; then
        echo "❌ ALERT: $env_file is tracked in git!"
    fi
done
```

### Git History Scan
```bash
# Scan commit history for secrets (requires gitleaks)
if command -v gitleaks &> /dev/null; then
    echo "🔍 Scanning git history for secrets..."
    gitleaks detect --source . --verbose
else
    echo "⚠️  Install gitleaks for history scanning: https://github.com/gitleaks/gitleaks"
fi
```

### Configuration Validation
```bash
# Verify secrets come from environment
echo "⚙️ Checking configuration sources..."

# Should use env vars
rg "env::var|std::env::var|dotenvy::" src/config/ --type rust -n | wc -l

# Should NOT have hardcoded values
rg "client_secret.*=.*\"[a-zA-Z0-9]" src/config/ --type rust -n && \
  echo "⚠️  Hardcoded client_secret" || \
  echo "✓ Secrets from environment"
```

## Common Secret Patterns

### API Keys
```rust
// ❌ FORBIDDEN
const API_KEY: &str = "sk_live_51H9xK2...";
let api_key = "pk_test_abc123...";

// ✅ CORRECT
let api_key = env::var("API_KEY")
    .map_err(|_| ConfigError::MissingApiKey)?;
```

### OAuth Client Secrets
```rust
// ❌ FORBIDDEN
let client_secret = "your-client-secret-here";

// ✅ CORRECT
let client_secret = env::var("STRAVA_CLIENT_SECRET")
    .map_err(|_| ConfigError::MissingStravaSecret)?;
```

### Database URLs
```rust
// ❌ FORBIDDEN
const DATABASE_URL: &str = "postgres://user:password@localhost/db";

// ✅ CORRECT
let database_url = env::var("DATABASE_URL")
    .map_err(|_| ConfigError::MissingDatabaseUrl)?;
```

### Encryption Keys
```rust
// ❌ FORBIDDEN
const MASTER_KEY: &str = "aGVsbG93b3JsZGhlbGxvd29ybGQ=";

// ✅ CORRECT
let master_key = env::var("MASTER_ENCRYPTION_KEY")
    .map_err(|_| ConfigError::MissingMasterKey)?;
```

### JWT Signing Keys
```rust
// ❌ FORBIDDEN
const JWT_SECRET: &str = "super-secret-key";

// ✅ CORRECT (RSA keypair from files)
let private_key = fs::read_to_string(
    env::var("JWT_PRIVATE_KEY_PATH")?
)?;
```

## Allowed Patterns

### Test/Example Values
```rust
// ✅ ALLOWED (clearly marked as example)
const EXAMPLE_API_KEY: &str = "example_key_for_testing";  // Not a real key

// ✅ ALLOWED (test fixtures)
#[cfg(test)]
mod tests {
    const TEST_CLIENT_SECRET: &str = "test_secret";  // Test only
}
```

### Public Constants
```rust
// ✅ ALLOWED (public, non-sensitive)
const STRAVA_AUTHORIZE_URL: &str = "https://www.strava.com/oauth/authorize";
const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";
```

## False Positive Handling

### Known Safe Patterns
```bash
# Exclude test files
rg "api_key" src/ --type rust -n | grep -v "^tests/" | grep -v "_test.rs"

# Exclude example values
rg "secret" src/ --type rust -n | grep -v "example" | grep -v "placeholder"

# Exclude documentation
rg "password" . -n | grep -v "README" | grep -v ".md:"
```

## CI/CD Integration

### Pre-Commit Hook
```bash
# .git/hooks/pre-commit
./scripts/validate-no-secrets.sh || exit 1
```

### GitHub Actions
```yaml
# .github/workflows/security.yml
- name: Check for secrets
  run: |
    ./scripts/validate-no-secrets.sh
    if [ $? -ne 0 ]; then
      echo "❌ Secrets detected in code!"
      exit 1
    fi
```

## Remediation Steps

### If Secret Found in Code
```bash
# 1. Remove the secret immediately
# 2. Replace with environment variable
# 3. Update .env.example (without real values)
# 4. Commit the fix

# Example:
echo "STRAVA_CLIENT_SECRET=your-secret-here" >> .env.example
echo "STRAVA_CLIENT_SECRET" >> .env.example  # Template only

# 5. Rotate the compromised secret
# (Generate new API key, client secret, etc.)
```

### If Secret in Git History
```bash
# WARNING: Rewriting history affects all collaborators

# Option 1: Use git-filter-repo (recommended)
git filter-repo --invert-paths --path .env

# Option 2: Use BFG Repo-Cleaner
bfg --delete-files .env
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# Force push (coordinate with team!)
git push origin --force --all
```

## Environment Variable Management

### Local Development
```bash
# Create .env from template
cp .env.example .env

# Add real secrets (never commit!)
echo "STRAVA_CLIENT_SECRET=actual-secret" >> .env

# Use direnv for automatic loading
direnv allow
```

### Production Deployment
```bash
# Use secret management service
# - AWS Secrets Manager
# - HashiCorp Vault
# - Kubernetes Secrets
# - GitHub Secrets (CI/CD)

# Example: Docker with secrets
docker run -e STRAVA_CLIENT_SECRET="$STRAVA_CLIENT_SECRET" pierre-mcp-server
```

## Success Criteria
- ✅ No API keys in source code
- ✅ No passwords in source code
- ✅ No OAuth secrets in source code
- ✅ No database URLs with credentials
- ✅ No encryption keys hardcoded
- ✅ .env files not tracked in git
- ✅ .env in .gitignore
- ✅ All secrets from environment variables
- ✅ Git history clean (no historical leaks)

## Tools Integration

### Install gitleaks (optional)
```bash
# macOS
brew install gitleaks

# Linux
wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.0/gitleaks_8.18.0_linux_x64.tar.gz
tar -xzf gitleaks_8.18.0_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
```

### Install truffleHog (optional)
```bash
pip install truffleHog

# Scan repository
trufflehog filesystem --directory .
```

## Related Files
- `scripts/validate-no-secrets.sh` - Secret detection script
- `.gitignore` - Excludes .env and sensitive files
- `.env.example` - Template for environment variables
- `docs/configuration.md` - Configuration documentation

## Related Skills
- `validate-architecture.md` - Architectural validation
- `strict-clippy-check.md` - Code quality
- `security-auditor.md` (agent) - Comprehensive security audit
