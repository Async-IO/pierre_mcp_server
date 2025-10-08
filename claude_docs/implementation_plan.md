# Implementation TODO Plan - GPT-5 Recommendations

**Branch**: `sonnet_analysis`  
**Status**: READY FOR APPROVAL  
**Estimated Total Effort**: 5-15 days (depending on phase approval)

---

## Overview

This document provides an actionable implementation plan for the GPT-5 routing recommendations. The plan is broken into three phases, each independently valuable and progressively more ambitious.

---

## PHASE 1: Warp Improvements (RECOMMENDED - WEEK 1)

**Status**: ✅ **READY TO START**  
**Effort**: 40 hours (1 week)  
**Risk**: LOW  
**Value**: HIGH (Security + Code Quality)

### Prerequisites
- [ ] Approval from ChefFamille
- [ ] Create implementation branch from `sonnet_analysis`
- [ ] Review current test coverage for affected modules

### Task Breakdown

#### 🔴 Priority 1: Security & Correctness (Days 1-2)

**Task 1.1: CORS Origin Allowlist**
- **File**: `src/middleware/cors.rs`
- **Effort**: 2 hours
- **Steps**:
  1. [ ] Add `CorsConfig` struct to `src/config/mod.rs`
     ```rust
     pub struct CorsConfig {
         pub allowed_origins: Vec<String>,
         pub allow_localhost_any_port: bool,
     }
     ```
  2. [ ] Update `is_valid_origin()` to use config
  3. [ ] Add environment variables:
     - `CORS_ALLOWED_ORIGINS` (comma-separated)
     - `CORS_ALLOW_LOCALHOST_ANY_PORT` (boolean)
  4. [ ] Update tests in `tests/cors_tests.rs`
  5. [ ] Update documentation: `docs/developer-guide/17-security-guide.md`
- **Success Criteria**:
  - [ ] CORS blocks unknown origins
  - [ ] Localhost still works in development
  - [ ] Tests pass for allowed/blocked scenarios

**Task 1.2: MCP JSON-RPC Error Envelopes**
- **File**: `src/mcp/multitenant.rs` (function `handle_mcp_http_request`)
- **Effort**: 1.5 hours
- **Steps**:
  1. [ ] Update invalid JSON handling:
     ```rust
     match serde_json::from_value::<McpRequest>(body.clone()) {
         Ok(req) => { /* existing logic */ }
         Err(_) => {
             let err = McpResponse::error(
                 Some(default_request_id()),
                 -32600,
                 "Invalid request".into()
             );
             return Ok(Box::new(warp::reply::with_status(
                 warp::reply::json(&err),
                 StatusCode::OK
             )));
         }
     }
     ```
  2. [ ] Update parse error handling (similar pattern)
  3. [ ] Add tests for error envelope format
  4. [ ] Update MCP protocol docs: `docs/developer-guide/04-mcp-protocol.md`
- **Success Criteria**:
  - [ ] Invalid JSON returns HTTP 200 with JSON-RPC error
  - [ ] Auth failures still return HTTP 401
  - [ ] MCP SDK tests pass

**Task 1.3: Session Cache Bounding (LRU)**
- **File**: `src/mcp/multitenant.rs` (struct `MultiTenantMcpServer`)
- **Effort**: 3 hours
- **Steps**:
  1. [ ] Add dependency: `lru = "0.12"` to `Cargo.toml`
  2. [ ] Replace `HashMap` with `LruCache`:
     ```rust
     sessions: Arc<tokio::sync::Mutex<LruCache<String, SessionData>>>,
     ```
  3. [ ] Update `new()` to initialize with capacity:
     ```rust
     let cache_size = NonZeroUsize::new(10_000).unwrap();
     LruCache::new(cache_size)
     ```
  4. [ ] Add environment variable: `MCP_SESSION_CACHE_SIZE` (default: 10000)
  5. [ ] Add metrics/logging for cache evictions
  6. [ ] Test with high session churn
- **Success Criteria**:
  - [ ] Memory usage bounded
  - [ ] Active sessions not evicted under normal load
  - [ ] Metrics show cache behavior

**Subtotal**: 6.5 hours

---

#### 🟡 Priority 2: Architecture & Consistency (Days 3-4)

**Task 2.1: Implement Unified Auth Filters**
- **File**: `src/auth/filters.rs` (NEW FILE)
- **Effort**: 3 hours
- **Steps**:
  1. [ ] Create `src/auth/filters.rs`
  2. [ ] Implement `with_auth()` filter (see Part 4.1 in analysis)
  3. [ ] Implement `maybe_auth()` filter
  4. [ ] Implement `parse_and_validate()` helper (JWT + API key support)
  5. [ ] Add `AuthError` rejection type
  6. [ ] Create comprehensive tests: `tests/auth_filter_tests.rs`
  7. [ ] Update `src/auth/mod.rs` to export filters
- **Success Criteria**:
  - [ ] Filters extract `AuthResult` correctly
  - [ ] Both JWT and API key validation work
  - [ ] Error messages are consistent
  - [ ] Tests cover all auth scenarios

**Task 2.2: Migrate API Key Routes**
- **File**: `src/api_key_routes.rs`
- **Effort**: 2 hours
- **Steps**:
  1. [ ] Update `create_api_key_simple()` signature:
     ```rust
     pub async fn create_api_key_simple(
         &self,
         auth: AuthResult,  // Changed from Option<&str>
         request: CreateApiKeyRequestSimple,
     ) -> Result<ApiKeyCreateResponse>
     ```
  2. [ ] Remove `authenticate_user()` method (now redundant)
  3. [ ] Update route creation in `multitenant.rs`:
     ```rust
     let create_api_key = warp::path("api")
         .and(warp::path("keys"))
         .and(warp::post())
         .and(with_auth(resources.clone()))
         .and(warp::body::json())
         .and_then(|auth, req| api_key_routes.create_api_key_simple(auth, req));
     ```
  4. [ ] Update all other API key route handlers
  5. [ ] Run tests: `cargo test api_key`
- **Success Criteria**:
  - [ ] All API key routes use `with_auth`
  - [ ] No manual header parsing in handlers
  - [ ] Tests pass

**Task 2.3: Migrate Dashboard Routes**
- **File**: `src/dashboard_routes.rs`
- **Effort**: 1.5 hours
- **Steps**: (Similar pattern to Task 2.2)
  1. [ ] Update handler signatures to accept `AuthResult`
  2. [ ] Update route creation to use `with_auth`
  3. [ ] Remove manual auth logic
  4. [ ] Run tests: `cargo test dashboard`
- **Success Criteria**:
  - [ ] All dashboard routes consistent
  - [ ] Tests pass

**Task 2.4: Migrate Configuration Routes**
- **File**: `src/configuration_routes.rs`
- **Effort**: 1.5 hours
- **Steps**: (Similar pattern)
  1. [ ] Update handlers
  2. [ ] Update routes
  3. [ ] Test
- **Success Criteria**:
  - [ ] Configuration routes use filters
  - [ ] Tests pass

**Task 2.5: Migrate A2A Routes**
- **File**: `src/a2a_routes.rs`
- **Effort**: 2 hours
- **Steps**: (Similar pattern)
  1. [ ] Update handlers
  2. [ ] Update routes
  3. [ ] Test A2A auth flow
- **Success Criteria**:
  - [ ] A2A routes consistent
  - [ ] Tests pass

**Subtotal**: 10 hours

---

#### 🟢 Priority 3: Compile Time Optimization (Day 5)

**Task 3.1: Box Individual Route Groups**
- **Files**: All `create_*_routes()` functions in `src/mcp/multitenant.rs`
- **Effort**: 2 hours
- **Steps**:
  1. [ ] Import `use warp::filters::BoxedFilter;`
  2. [ ] Update each function to return `BoxedFilter<(impl Reply,)>`
  3. [ ] Add `.boxed()` before each return
  4. [ ] Example:
     ```rust
     fn create_auth_routes(
         auth_routes: AuthRoutes,
     ) -> BoxedFilter<(impl Reply,)> {
         // ... existing route logic ...
         register.or(login).or(refresh).boxed()
     }
     ```
  5. [ ] Apply to all route creation functions:
     - `create_auth_routes`
     - `create_oauth_routes`
     - `create_api_key_routes`
     - `create_dashboard_routes`
     - `create_a2a_*_routes` (4 functions)
     - `create_configuration_routes`
     - `create_fitness_configuration_routes`
     - `create_tenant_routes_filter`
  6. [ ] Compile and verify no errors
- **Success Criteria**:
  - [ ] All route functions return `BoxedFilter`
  - [ ] Code compiles without warnings
  - [ ] Type errors are more readable

**Task 3.2: Box Top-Level Composition**
- **File**: `src/mcp/multitenant.rs` (function `run_http_server_with_resources`)
- **Effort**: 30 minutes
- **Steps**:
  1. [ ] Add `.boxed()` to final routes composition:
     ```rust
     let routes = auth_route_filter
         .or(oauth_route_filter)
         // ... all other routes ...
         .with(cors)
         .with(security_headers)
         .recover(handle_rejection)
         .boxed();
     ```
  2. [ ] Verify compilation
- **Success Criteria**:
  - [ ] Top-level routes boxed
  - [ ] Code compiles

**Task 3.3: Measure Compile Time**
- **Effort**: 30 minutes
- **Steps**:
  1. [ ] Clean build: `cargo clean`
  2. [ ] Measure before: `time cargo build --release`
  3. [ ] Record baseline time
  4. [ ] Implement boxing changes
  5. [ ] Clean again: `cargo clean`
  6. [ ] Measure after: `time cargo build --release`
  7. [ ] Calculate improvement percentage
  8. [ ] Document in `claude_docs/compile_time_metrics.md`
- **Success Criteria**:
  - [ ] Documented baseline vs improved compile times
  - [ ] Expected: 10-20% improvement

**Subtotal**: 3 hours

---

#### 🔵 Priority 4: Quality of Life (Day 5 afternoon)

**Task 4.1: SSE Keepalive**
- **File**: `src/sse/routes.rs`
- **Effort**: 1 hour
- **Steps**:
  1. [ ] Update SSE reply creation:
     ```rust
     let keep = warp::sse::keep_alive()
         .interval(Duration::from_secs(15))
         .text(": keepalive\n\n");
     let reply = warp::sse::reply(keep.stream(stream));
     ```
  2. [ ] Add environment variable: `SSE_KEEPALIVE_INTERVAL` (default: 15s)
  3. [ ] Test with long-lived connection
  4. [ ] Verify proxies don't timeout
- **Success Criteria**:
  - [ ] SSE connections stay alive
  - [ ] Keepalive comments visible in client

**Task 4.2: Documentation Updates**
- **Effort**: 2 hours
- **Files to update**:
  1. [ ] `docs/developer-guide/04-mcp-protocol.md` (JSON-RPC errors, SSE keepalive)
  2. [ ] `docs/developer-guide/17-security-guide.md` (CORS config)
  3. [ ] `docs/developer-guide/09-api-routes.md` (auth filter usage)
  4. [ ] `README.md` (new environment variables)
  5. [ ] Create migration guide: `docs/MIGRATION_PHASE1.md`
- **Success Criteria**:
  - [ ] All new features documented
  - [ ] Migration guide complete
  - [ ] Examples updated

**Subtotal**: 3 hours

---

### Phase 1 Testing & Validation (End of Week)

**Task 5: Comprehensive Testing**
- **Effort**: 2-3 hours
- **Steps**:
  1. [ ] Run full test suite: `./scripts/lint-and-test.sh`
  2. [ ] Run clippy with strict settings:
     ```bash
     cargo clippy -- -W clippy::all -W clippy::pedantic -D warnings
     ```
  3. [ ] Check for prohibited patterns:
     ```bash
     rg "unwrap\(\)|expect\(|panic!\(" src/ --count-matches
     rg "#\[allow\(clippy::" src/ --count-matches
     ```
  4. [ ] Integration testing:
     - [ ] Auth flow (JWT + API key)
     - [ ] MCP protocol (valid + invalid requests)
     - [ ] SSE long-lived connection
     - [ ] CORS from various origins
  5. [ ] Performance testing:
     - [ ] Measure request latency (should be unchanged)
     - [ ] Memory usage monitoring (session cache bounded)
  6. [ ] Generate coverage report (if available)

**Task 6: Code Review & Merge**
- **Steps**:
  1. [ ] Self-review all changes
  2. [ ] Create PR from `sonnet_analysis` to `main`
  3. [ ] PR description with:
     - Summary of changes
     - Testing performed
     - Breaking changes (none expected)
     - Migration notes
  4. [ ] Request review from team
  5. [ ] Address review feedback
  6. [ ] Squash merge to main

---

### Phase 1 Success Criteria Summary

- [ ] All security improvements implemented (CORS, session bounds)
- [ ] Auth pattern unified across all routes
- [ ] Compile time reduced by ≥10%
- [ ] All tests pass (`./scripts/lint-and-test.sh`)
- [ ] No clippy warnings
- [ ] Binary size unchanged or smaller
- [ ] Documentation updated
- [ ] Zero production incidents post-deployment

**Phase 1 Total Effort**: ~25-30 hours (1 week with buffer)

---

## PHASE 2: Axum Feasibility Spike (OPTIONAL - WEEK 2-3)

**Status**: ⏸️ **AWAITING PHASE 1 COMPLETION + APPROVAL**  
**Effort**: 35-40 hours (5-7 days)  
**Risk**: MEDIUM  
**Value**: HIGH (Risk mitigation for future)

### Prerequisites
- [ ] Phase 1 complete and merged
- [ ] Approval from ChefFamille for exploration
- [ ] Create spike branch: `spike/axum-evaluation`

### Spike Goals

1. **Technical Validation**: Prove Axum can handle all our requirements
2. **Performance Comparison**: Benchmark compile times and runtime
3. **Ergonomics Assessment**: Team feedback on developer experience
4. **Migration Cost Estimation**: Detailed breakdown for full port

### Task Breakdown

**Day 1: Setup & Infrastructure**
- [ ] Task: Create basic Axum server
  - [ ] Add `axum = "0.8"` to Cargo.toml (feature-gated)
  - [ ] Setup shared state pattern (equivalent to `ServerResources`)
  - [ ] Implement basic error handling (`IntoResponse` for `AppError`)
  - [ ] Create health check endpoint
  - [ ] Verify server starts and responds

**Day 2: Auth System**
- [ ] Task: Implement auth extractors
  - [ ] Create `AuthExtractor` (equivalent to `with_auth`)
  - [ ] Create `MaybeAuthExtractor` (equivalent to `maybe_auth`)
  - [ ] Support JWT validation
  - [ ] Support API key validation
  - [ ] Test auth flows

**Day 3: Port Auth Routes**
- [ ] Task: Register, Login, Refresh endpoints
  - [ ] Port registration handler
  - [ ] Port login handler
  - [ ] Port token refresh handler
  - [ ] Test complete auth flow
  - [ ] Compare code complexity with Warp version

**Day 4: Port Complex Route Group**
- [ ] Task: Choose between Dashboard or MCP endpoint
  - [ ] Dashboard option: Port all dashboard routes
  - [ ] MCP option: Port HTTP JSON-RPC endpoint
  - [ ] Measure implementation time
  - [ ] Note any blockers or challenges

**Day 5: SSE Implementation**
- [ ] Task: Server-Sent Events in Axum
  - [ ] Use `axum::response::sse::Sse`
  - [ ] Implement keepalive
  - [ ] Test long-lived connection
  - [ ] Compare with Warp implementation

**Day 6: Benchmarking**
- [ ] Task: Comprehensive comparison
  - **Compile Times**:
    - [ ] Clean build time (Warp vs Axum)
    - [ ] Incremental build time
    - [ ] IDE responsiveness
  - **Runtime Performance**:
    - [ ] Request latency (p50, p95, p99)
    - [ ] Memory usage
    - [ ] Connection handling
  - **Code Metrics**:
    - [ ] Lines of code comparison
    - [ ] Cyclomatic complexity
    - [ ] Type error readability

**Day 7: Analysis & Recommendation**
- [ ] Task: Create decision document
  - [ ] Compile all metrics
  - [ ] Team feedback survey
  - [ ] Pros/Cons analysis
  - [ ] Effort estimate for full migration
  - [ ] Go/No-Go recommendation
  - [ ] Present to team

### Spike Deliverables

1. [ ] Working Axum implementation (20% of routes)
2. [ ] Benchmark comparison document
3. [ ] Code complexity analysis
4. [ ] Team feedback summary
5. [ ] Full migration effort estimate
6. [ ] Formal recommendation (Go/No-Go)

### Spike Success Criteria

**GO Decision Factors**:
- [ ] Compile time improved by ≥25%
- [ ] Runtime performance equal or better
- [ ] Code is more maintainable (subjective, team vote)
- [ ] No major blockers identified
- [ ] Team is confident in migration

**NO-GO Decision Factors**:
- [ ] Any requirement can't be met (SSE, WebSocket, etc.)
- [ ] Performance regression
- [ ] Migration effort >3 weeks
- [ ] Team lacks confidence

---

## PHASE 3: Full Axum Migration (CONTINGENT - WEEKS 4-6)

**Status**: ⏸️ **CONTINGENT ON PHASE 2 GO DECISION**  
**Effort**: 80-120 hours (2-3 weeks)  
**Risk**: MEDIUM-HIGH  
**Value**: VERY HIGH (Long-term)

### Prerequisites
- [ ] Phase 2 spike complete with GO recommendation
- [ ] Approval from ChefFamille for full migration
- [ ] Create migration branch: `feat/axum-migration`
- [ ] Feature flag strategy defined

### Week 4: Core Migration

**Day 1-2: Auth & OAuth**
- [ ] Port all auth routes (register, login, refresh, etc.)
- [ ] Port OAuth flow (authorize, callback, token)
- [ ] Port OAuth2 server routes (RFC 7591)
- [ ] Integration tests

**Day 3-4: REST APIs**
- [ ] Port API key management routes
- [ ] Port dashboard routes
- [ ] Port A2A protocol routes (4 groups)
- [ ] Integration tests

**Day 5: Configuration**
- [ ] Port configuration routes
- [ ] Port fitness configuration routes
- [ ] Port health check
- [ ] Integration tests

### Week 5: Advanced Features

**Day 1-2: MCP Protocol**
- [ ] Port MCP HTTP endpoint
- [ ] Port session management
- [ ] Port conditional auth logic
- [ ] Test with MCP SDK

**Day 3: SSE**
- [ ] Port SSE routes
- [ ] Implement keepalive
- [ ] Test notification flow
- [ ] Test MCP SSE transport

**Day 4-5: Admin & Tenant**
- [ ] Port admin routes
- [ ] Port tenant management routes
- [ ] Port tenant OAuth configuration
- [ ] Integration tests

### Week 6: Stabilization

**Day 1-2: Integration Testing**
- [ ] End-to-end auth flows
- [ ] MCP client integration
- [ ] A2A protocol compliance
- [ ] OAuth2 client registration
- [ ] Multi-tenant isolation

**Day 3: Performance Validation**
- [ ] Load testing (compare to Warp baseline)
- [ ] Memory leak testing
- [ ] Connection pool stress testing
- [ ] Benchmark report

**Day 4: Documentation**
- [ ] Update developer guide
- [ ] Update API reference
- [ ] Create Axum migration notes
- [ ] Update deployment guide

**Day 5: Review & Merge**
- [ ] Final code review
- [ ] Remove Warp feature flag (after 1 release cycle)
- [ ] Update CI/CD
- [ ] Deploy to staging
- [ ] Merge to main

### Phase 3 Success Criteria

- [ ] 100% feature parity with Warp implementation
- [ ] All tests pass (unit, integration, E2E)
- [ ] Performance equal or better than baseline
- [ ] Documentation complete
- [ ] Zero production incidents for 1 week post-deployment
- [ ] Team confident in new codebase

---

## Risk Management

### Phase 1 Risks

| Risk | Mitigation |
|------|------------|
| Breaking changes | Comprehensive testing, feature flags if needed |
| Performance regression | Benchmark before/after, revert if issues |
| Session eviction too aggressive | Tune LRU size, add metrics |

### Phase 2 Risks

| Risk | Mitigation |
|------|------------|
| Spike takes too long | Time-box to 7 days, make Go/No-Go decision |
| Missing features in Axum | Identify early, research workarounds |
| Team resistance | Gather feedback early, address concerns |

### Phase 3 Risks

| Risk | Mitigation |
|------|------------|
| Timeline overrun | Break into smaller PRs, parallel work |
| Production issues | Feature flag, gradual rollout, quick rollback |
| Migration bugs | Extensive testing, staging environment |

---

## Rollback Plans

### Phase 1
- **Trigger**: Test failures, performance regression, production issues
- **Action**: `git revert <commit>`, redeploy previous version
- **Time**: <1 hour

### Phase 2
- **Trigger**: None needed (spike branch, not deployed)
- **Action**: Archive branch, continue with Warp

### Phase 3
- **Trigger**: Production issues, performance problems
- **Action**: 
  1. Toggle feature flag to Warp (if still available)
  2. Or revert merge commit
  3. Redeploy
  4. Root cause analysis
- **Time**: <2 hours

---

## Success Metrics

### Phase 1
- Security: CORS blocks unauthorized origins
- Performance: Request latency unchanged (±5%)
- Quality: Clippy passes, no warnings
- Compile time: Reduced by ≥10%

### Phase 2
- Compile time: Reduced by ≥25% vs baseline
- Code complexity: Reduced (LoC, cyclomatic)
- Team confidence: ≥80% positive feedback

### Phase 3
- Uptime: 99.9% maintained
- Performance: Equal or better
- Maintainability: Reduced onboarding time for new devs
- Velocity: Faster feature development (measured after 1 month)

---

## Approvals Required

### Phase 1
- [ ] **ChefFamille** (Technical Lead/Boss) - APPROVE/REJECT
- [ ] **Budget**: 1 week (40 hours)

### Phase 2
- [ ] **ChefFamille** - APPROVE/REJECT after Phase 1 complete
- [ ] **Budget**: 1 week (40 hours)

### Phase 3
- [ ] **ChefFamille** - APPROVE/REJECT after Phase 2 GO recommendation
- [ ] **Budget**: 2-3 weeks (80-120 hours)
- [ ] **Stakeholders**: Review migration plan

---

## Timeline Summary

```
Week 1: Phase 1 (Warp Improvements)
├─ Day 1-2: Security & Correctness
├─ Day 3-4: Architecture & Consistency
└─ Day 5: Compile Optimization + QoL

Week 2-3: Phase 2 (Axum Spike) [OPTIONAL]
├─ Day 1-2: Setup + Auth
├─ Day 3-5: Route porting
├─ Day 6: Benchmarking
└─ Day 7: Analysis

Week 4-6: Phase 3 (Full Migration) [CONTINGENT]
├─ Week 4: Core routes
├─ Week 5: Advanced features
└─ Week 6: Stabilization
```

---

## Next Steps

**Immediate Action Items**:
1. [ ] ChefFamille reviews this plan
2. [ ] Decide on Phase 1 approval
3. [ ] If approved, create implementation branch
4. [ ] Assign developer(s) to Phase 1
5. [ ] Schedule daily standups for progress tracking

**Communication Plan**:
- Daily: Slack updates on progress
- End of Day 2: Security improvements demo
- End of Week: Phase 1 review meeting
- Phase 2 Decision: Team meeting to discuss Axum spike
- Phase 3 Decision: Stakeholder review of migration plan

---

*Document maintained by: Claude (Senior Rust Developer)*  
*Last updated: 2025*  
*Status: READY FOR REVIEW*
