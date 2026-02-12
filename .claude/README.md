# Claude Code Configuration for Pierre Fitness Platform

This directory contains Claude Code agents, skills, and commands optimized for the Pierre MCP Server codebase.

## 📁 Directory Structure

```
.claude/
├── README.md          # This file
├── agents/           # Complex multi-step automation
│   ├── security-auditor.md
│   ├── protocol-compliance.md
│   ├── algorithm-validator.md
│   ├── test-orchestrator.md
│   ├── error-handling-guardian.md      # NEW: Guards unified error typing
│   └── repository-pattern-guardian.md  # NEW: Guards database refactoring
├── skills/           # Specific single-purpose tasks
│   ├── test-mcp-compliance.md
│   ├── strict-clippy-check.md
│   ├── test-multitenant-isolation.md
│   ├── generate-sdk-types.md
│   ├── validate-architecture.md
│   ├── check-no-secrets.md
│   ├── run-full-test-suite.md
│   ├── test-intelligence-algorithms.md
│   ├── check-error-handling.md          # NEW: Quick anyhow regression check
│   └── check-repository-pattern.md      # NEW: Quick repository pattern check
└── commands/         # Custom slash commands (future)
```

## 🤖 What Are Agents?

Agents are autonomous multi-step workflows that can:
- Execute complex validation pipelines
- Generate comprehensive reports
- Coordinate multiple tools and scripts
- Make decisions based on codebase state

### Available Agents

#### 1. Security & Multi-Tenancy Auditor (`security-auditor.md`)
**Purpose:** Comprehensive security audit for tenant isolation, cryptography, and OWASP compliance

**Use When:**
- Before production deployments
- After authentication changes
- Weekly security audits
- Before security reviews

**Key Tasks:**
- Tenant isolation validation
- Cryptography audit (JWT, AES-GCM, RS256)
- Input validation checks
- Secret management audit
- OWASP Top 10 compliance
- Multi-tenant database scoping

**Example Usage:**
```
Claude, run the security auditor agent
```

---

#### 2. Protocol Compliance Guardian (`protocol-compliance.md`)
**Purpose:** Validates MCP and A2A protocol compliance

**Use When:**
- Before protocol handler changes
- After tool modifications
- Before SDK releases
- Weekly regression testing

**Key Tasks:**
- MCP JSON-RPC 2.0 validation
- A2A protocol compliance
- OAuth 2.0 server compliance (RFC 6749, RFC 7591)
- OAuth 2.0 client compliance
- Tool schema validation
- Transport layer testing (HTTP, stdio, WebSocket, SSE)
- Official MCP compliance suite execution

**Example Usage:**
```
Claude, run the protocol compliance guardian agent
```

---

#### 3. Intelligence Algorithm Validator (`algorithm-validator.md`)
**Purpose:** Validates sports science algorithms for mathematical correctness

**Use When:**
- After algorithm modifications
- Before releases
- Investigating calculation bugs
- Validating against research

**Key Tasks:**
- VDOT (Daniels) race predictions
- TSS/CTL/ATL/TSB training load
- TRIMP (Bannister) calculations
- FTP (Functional Threshold Power)
- VO2max estimation
- Recovery & sleep analysis
- Nutrition (BMR/TDEE) calculations
- Physiological bounds validation
- Algorithm configuration testing

**Example Usage:**
```
Claude, run the algorithm validator agent to check all sports science calculations
```

---

#### 4. Cross-Platform Test Orchestrator (`test-orchestrator.md`)
**Purpose:** Orchestrates comprehensive testing across environments

**Use When:**
- Before commits
- Before pull requests
- Before releases
- Weekly regression testing

**Key Tasks:**
- SQLite and PostgreSQL database tests
- Rust backend testing (unit + integration + doc)
- TypeScript SDK testing
- Frontend React testing
- Multi-transport testing (HTTP, stdio, WebSocket, SSE)
- Multi-tenant isolation validation
- Performance and load testing
- Code quality and linting
- Test coverage analysis

**Example Usage:**
```
Claude, run the test orchestrator agent for a full validation before my PR
```

---

#### 5. Error Handling Guardian (`error-handling-guardian.md`) 🆕
**Purpose:** Guards the unified error handling refactoring (commit b592b5e) - 111 files migrated from anyhow to AppResult

**Use When:**
- Before committing error handling changes
- Weekly regression checks
- After dependency updates
- Before releases
- When reviewing PRs with error handling

**Key Tasks:**
- Anyhow regression detection (no anyhow! macro, no anyhow imports)
- AppResult<T> usage validation
- ErrorCode enum usage verification
- Error context preservation checks
- HTTP status code mapping validation
- External crate error conversion validation
- Error handling test coverage
- Migration completeness verification

**Refactoring Context:**
- **Commit:** b592b5e (Nov 19, 2025)
- **Scope:** 111 files converted
- **Pattern:** anyhow → structured AppResult types with ErrorCode enum

**Example Usage:**
```
Claude, run the error handling guardian to check for anyhow regressions
```

---

#### 6. Repository Pattern Guardian (`repository-pattern-guardian.md`) 🆕
**Purpose:** Guards the repository pattern refactoring (commit 6f3efef) - eliminated 135-method god-trait into 13 focused repositories

**Use When:**
- Before committing database changes
- Weekly architecture reviews
- After adding new repositories
- Before releases
- When reviewing database-related PRs

**Key Tasks:**
- God-trait regression detection (no DatabaseProvider)
- Repository trait structure validation (13 repositories exist)
- Single Responsibility Principle validation
- Repository usage pattern verification
- Dependency injection validation
- Interface segregation validation
- Repository implementation completeness
- Transaction support validation
- Naming consistency checks
- Performance and query optimization

**Refactoring Context:**
- **Commit:** 6f3efef (Nov 19, 2025)
- **Scope:** 90+ files, complete database layer restructure
- **Pattern:** God-trait (135 methods) → 13 focused repositories (SOLID principles)

**The 13 Repositories:**
1. UserRepository - User account management
2. TenantRepository - Multi-tenant management
3. ApiKeyRepository - API key management
4. OAuthTokenRepository - OAuth token storage
5. OAuth2ServerRepository - OAuth 2.0 server
6. AdminRepository - Admin token management
7. A2ARepository - Agent-to-Agent operations
8. NotificationRepository - OAuth notifications
9. UsageRepository - Usage tracking/analytics
10. FitnessConfigRepository - Fitness configuration
11. ProfileRepository - User profiles
12. SecurityRepository - Security and key rotation
13. InsightRepository - AI-generated insights

**Example Usage:**
```
Claude, run the repository pattern guardian to ensure SOLID principles
```

## 🛠️ What Are Skills?

Skills are focused, single-purpose tasks with clear commands. They're faster than agents and ideal for specific validation steps.

### Available Skills

#### Testing & Validation

| Skill | Purpose | Quick Command |
|-------|---------|---------------|
| `test-mcp-compliance.md` | MCP protocol validation | `./scripts/ci/ensure-mcp-compliance.sh` |
| `test-multitenant-isolation.md` | Tenant isolation security | `cargo test --test mcp_multitenant_complete_test` |
| `run-full-test-suite.md` | All tests (unit+integration+E2E) | `cargo test --all-features` |
| `test-intelligence-algorithms.md` | Sports science algorithm validation | `cargo test intelligence -- --nocapture` |

#### Code Quality & Security

| Skill | Purpose | Quick Command |
|-------|---------|---------------|
| `strict-clippy-check.md` | Zero-tolerance linting | `cargo clippy --all-targets --all-features -- -D warnings` |
| `validate-architecture.md` | Pattern and architecture validation | `./scripts/ci/architectural-validation.sh` |
| `check-no-secrets.md` | Secret detection | `./scripts/ci/validate-no-secrets.sh` |
| `check-error-handling.md` 🆕 | Anyhow regression detection | `rg "use anyhow" src/ --type rust` |
| `check-repository-pattern.md` 🆕 | Repository pattern validation | `rg "trait DatabaseProvider" src/ --type rust` |

#### Development Workflows

| Skill | Purpose | Quick Command |
|-------|---------|---------------|
| `generate-sdk-types.md` | TypeScript type generation | `cd sdk && bun run generate-types` |

## 🎯 Usage Patterns

### Daily Development Workflow
```bash
# Before committing
Claude, run strict-clippy-check skill
Claude, run validate-architecture skill
Claude, run check-no-secrets skill
Claude, run check-error-handling skill       # NEW: Detect anyhow regression
Claude, run check-repository-pattern skill  # NEW: Validate repository pattern

# Or use agent for comprehensive check
Claude, run the test orchestrator agent with quick validation
```

### Before Pull Requests
```bash
# Comprehensive validation
Claude, run the security auditor agent
Claude, run the protocol compliance guardian agent
Claude, run error handling guardian agent    # NEW: Full error validation
Claude, run repository pattern guardian agent # NEW: Full database pattern validation
Claude, run strict-clippy-check skill
Claude, run run-full-test-suite skill
```

### Guarding Major Refactorings 🆕
```bash
# After error handling changes
Claude, run check-error-handling skill      # Quick check
Claude, run error handling guardian agent   # Comprehensive audit

# After database changes
Claude, run check-repository-pattern skill   # Quick check
Claude, run repository pattern guardian agent # Comprehensive audit
```

### Before Releases
```bash
# Full validation pipeline
Claude, run all agents in parallel:
1. Security auditor
2. Protocol compliance guardian
3. Algorithm validator
4. Test orchestrator
```

### After Algorithm Changes
```bash
Claude, run test-intelligence-algorithms skill
Claude, run the algorithm validator agent for comprehensive validation
```

### After Protocol Changes
```bash
Claude, run test-mcp-compliance skill
Claude, generate SDK types (generate-sdk-types skill)
Claude, run the protocol compliance guardian agent
```

### After Database Changes
```bash
Claude, run test-multitenant-isolation skill
Claude, run the security auditor agent focusing on database scoping
```

## 📋 CLAUDE.md Compliance

All agents and skills enforce Pierre's coding standards from `.claude/CLAUDE.md`:

### Zero Tolerance Policies
- ❌ NO `unwrap()`, `expect()`, `panic!()` in production code (src/)
- ❌ NO `anyhow::anyhow!()` - use structured error types only
- ❌ NO placeholder implementations or TODO comments
- ❌ NO hardcoded secrets or credentials
- ❌ NO magic strings for protocol methods
- ❌ NO hardcoded algorithm formulas outside `src/intelligence/algorithms/`

### Required Patterns
- ✅ ALL errors use `thiserror` with named fields
- ✅ ALL database queries parameterized (SQL injection prevention)
- ✅ ALL tests deterministic (seeded RNG, synthetic data)
- ✅ ALL algorithms use enum-based dependency injection
- ✅ ALL resources use dependency injection (no global state)
- ✅ ALL sensitive data uses `zeroize` for cleanup

### Testing Requirements
- ✅ Synthetic athlete data (no external OAuth in tests)
- ✅ Multi-tenant isolation tests (cross-tenant attacks)
- ✅ Edge case testing (zero, negative, extreme values)
- ✅ Both success and error path coverage

## 🚀 Quick Reference

### Most Common Commands

```bash
# Pre-commit validation
cargo clippy --all-targets --all-features -- -D warnings
./scripts/ci/architectural-validation.sh
./scripts/ci/validate-no-secrets.sh

# Full test suite
cargo test --all-features

# Security validation
cargo test --test mcp_multitenant_complete_test
./scripts/ci/architectural-validation.sh

# Protocol validation
./scripts/ci/ensure-mcp-compliance.sh

# Algorithm validation
cargo test intelligence -- --nocapture

# Generate SDK types (after tool changes)
cd sdk && bun run generate-types && cd ..
```

### CI/CD Workflow Mapping

| GitHub Workflow | Corresponding Skill/Agent |
|-----------------|---------------------------|
| `rust.yml` | `strict-clippy-check.md` + `run-full-test-suite.md` |
| `backend-ci.yml` | `test-orchestrator.md` (agent) |
| `sdk-tests.yml` | `generate-sdk-types.md` |
| `mcp-compliance.yml` | `test-mcp-compliance.md` |
| `security.yml` | `check-no-secrets.md` + `security-auditor.md` (agent) |

## 📊 Agent vs Skill Decision Matrix

| Scenario | Use Agent | Use Skill |
|----------|-----------|-----------|
| Quick single validation | | ✅ |
| Comprehensive audit | ✅ | |
| Pre-commit check | | ✅ |
| Pre-release validation | ✅ | |
| Debug specific issue | | ✅ |
| Generate report | ✅ | |
| Run specific script | | ✅ |
| Multi-step pipeline | ✅ | |

## 🔍 When to Use What

### Use **Skills** for:
- Running a specific test suite
- Linting code
- Generating types
- Quick validation
- Checking for secrets
- Running a single script

### Use **Agents** for:
- Comprehensive security audits
- Full protocol compliance validation
- Multi-step algorithm verification
- Cross-platform orchestration
- Generating detailed reports
- Coordinating multiple tools

## 📝 Creating Custom Skills/Agents

### Skill Template
Create `.claude/skills/my-skill.md`:

```markdown
# My Skill Name

## Purpose
[Clear one-sentence description]

## CLAUDE.md Compliance
- ✅ [Relevant coding standard]
- ✅ [Relevant testing standard]

## Usage
Run this skill:
- [When to use scenario 1]
- [When to use scenario 2]

## Commands
\`\`\`bash
# Main command
./scripts/my-script.sh
\`\`\`

## Success Criteria
- ✅ [Expected outcome 1]
- ✅ [Expected outcome 2]
```

### Agent Template
Create `.claude/agents/my-agent.md`:

```markdown
# My Agent Name

## Overview
[Comprehensive description of agent purpose]

## Coding Directives (CLAUDE.md)
[Relevant Pierre coding standards]

## Tasks

### 1. Task Name
**Objective:** [What this task accomplishes]

**Actions:**
\`\`\`bash
# Commands to execute
\`\`\`

**Validation:**
\`\`\`bash
# How to verify success
\`\`\`

[Repeat for each task...]

## Report Generation
[Template for agent output]

## Success Criteria
[List of success conditions]
```

## 🤝 Contributing

When adding new agents or skills:

1. **Follow CLAUDE.md standards** - All code must comply with Pierre's zero-tolerance policies
2. **Document thoroughly** - Include purpose, usage, commands, and success criteria
3. **Test comprehensively** - Validate the skill/agent works as documented
4. **Update this README** - Add your skill/agent to the tables above
5. **Use descriptive names** - Clear, action-oriented names (e.g., `test-oauth-flows.md`)

## 📚 Related Documentation

- `CONTRIBUTING.md` - Contribution guidelines
- `.claude/CLAUDE.md` - CLAUDE.md compliance checklist
- `scripts/ci/validation-patterns.toml` - Architectural validation patterns
- `book/src/ci-cd.md` - CI/CD workflow documentation

## 💡 Tips for Claude Code Users

1. **Be specific** - "Run the security auditor agent" is better than "check security"
2. **Reference by name** - Agents and skills have clear filenames
3. **Chain commands** - "Run clippy check, then full test suite"
4. **Ask for reports** - Agents can generate detailed markdown reports
5. **Parallel execution** - Request multiple agents to run in parallel for speed

## 🎓 Learning Path

For new contributors to Pierre:

1. **Start with skills** - Run `strict-clippy-check.md` and `run-full-test-suite.md`
2. **Learn patterns** - Review `validate-architecture.md` output
3. **Understand security** - Run `security-auditor.md` agent
4. **Master testing** - Use `test-orchestrator.md` agent
5. **Deep dive** - Explore algorithm validation with `algorithm-validator.md`

## 🔗 Integration with IDE

### VS Code
Add to `.vscode/tasks.json`:
```json
{
  "label": "Claude: Run Clippy",
  "type": "shell",
  "command": "cargo clippy --all-targets --all-features -- -D warnings"
}
```

### IntelliJ IDEA
Create run configuration for common skills as External Tools.

---

## 📞 Support

For issues or questions about agents and skills:
- Check agent/skill documentation first
- Review Pierre's contributing guidelines
- Open GitHub issue with `[claude-code]` prefix
- Reference specific agent/skill in issue description

---

**Last Updated:** 2025-11-17
**Pierre Version:** 0.2.0
**Claude Code Version:** Compatible with official Claude Code CLI
