# Static Audit Report - SilverScreen Commerce & Review Platform

## 1. Verdict
- **Overall conclusion:** **Partial Pass**

## 2. Scope and Static Verification Boundary
- **Reviewed:** repository documentation, backend Actix route wiring, middleware/auth/RBAC/risk/rate-limit, business services (orders/ratings/reviews/custom fields/backup/retention/audit), DB schema, frontend Yew routes/pages/API client/types, and test code.
- **Excluded from evidence:** `./.tmp/**` (not used as source evidence).
- **Intentionally not executed:** app startup, tests, Docker, network/external services, browser runtime.
- **Cannot be statically confirmed:** runtime correctness (timers, cron-like backup behavior, DB side effects, browser rendering fidelity, end-to-end UX timing, operational restore safety).
- **Manual verification required for:** real backup/restore runbook safety, UI rendering/accessibility quality, date-filter behavior in browser, and production-like load/race behavior.

## 3. Repository / Requirement Mapping Summary
- **Prompt core goal mapped:** offline-first commerce + review platform with Shopper/Reviewer/Admin workflows, strict order lifecycle automation, ratings + leaderboards tie-break, review rounds/versioning/attachments, security controls, auditability, retention, and local backup/restore.
- **Mapped implementation areas:**
  - Backend API surface and role boundaries: `backend/src/routes/mod.rs:21`, `backend/src/routes/auth.rs:14`, `backend/src/routes/orders.rs:21`, `backend/src/routes/reviews.rs:18`, `backend/src/routes/admin.rs:17`
  - Core persistence model: `backend/migrations/001_initial.sql:13`, `backend/migrations/001_initial.sql:190`, `backend/migrations/001_initial.sql:300`, `backend/migrations/001_initial.sql:360`, `backend/migrations/001_initial.sql:423`
  - Frontend workflow pages and API integration: `frontend/src/app.rs:25`, `frontend/src/pages/home.rs:24`, `frontend/src/pages/checkout.rs:13`, `frontend/src/pages/reviewer/submit.rs:64`, `frontend/src/pages/admin/audit_log.rs:12`

## 4. Section-by-section Review

### 4.1 Hard Gates
#### 4.1.1 Documentation and static verifiability
- **Conclusion:** Pass
- **Rationale:** Startup/test/env instructions exist and are statically consistent with repository structure and scripts.
- **Evidence:** `README.md:10`, `README.md:36`, `README.md:88`, `README.md:93`, `repo/.env.example:1`, `run_tests.sh:1`

#### 4.1.2 Material deviation from prompt
- **Conclusion:** Partial Pass
- **Rationale:** Most core flows are implemented; two admin-flow mismatches (audit date filter contract and reports form/date contract) reduce prompt-fit for operational auditing/reporting usability.
- **Evidence:** `backend/src/models/audit.rs:27`, `frontend/src/pages/admin/audit_log.rs:56`, `frontend/src/pages/admin/audit_log.rs:126`, `backend/src/routes/reports.rs:22`, `frontend/src/pages/admin/reports.rs:41`

### 4.2 Delivery Completeness
#### 4.2.1 Core explicit requirements coverage
- **Conclusion:** Partial Pass
- **Rationale:** Core commerce/review/admin/security capabilities are present, but admin audit/report date-handling has static contract gaps.
- **Evidence:** `backend/src/services/order_service.rs:94`, `backend/src/services/order_service.rs:480`, `backend/src/services/rating_service.rs:159`, `backend/src/routes/reviews.rs:256`, `backend/src/routes/custom_fields.rs:330`, `backend/src/services/backup_service.rs:18`

#### 4.2.2 End-to-end deliverable shape
- **Conclusion:** Pass
- **Rationale:** Coherent full-stack project with backend/frontend modules, migrations, CI script, and test directories.
- **Evidence:** `README.md:52`, `backend/Cargo.toml:1`, `frontend/Cargo.toml:1`, `backend/migrations/001_initial.sql:1`

### 4.3 Engineering and Architecture Quality
#### 4.3.1 Structure and module decomposition
- **Conclusion:** Pass
- **Rationale:** Reasonable route/service/model/middleware separation; frontend has API/pages/components separation.
- **Evidence:** `backend/src/routes/mod.rs:16`, `backend/src/services/mod.rs:1`, `backend/src/middleware/mod.rs:1`, `frontend/src/app.rs:6`, `frontend/src/api/mod.rs:1`

#### 4.3.2 Maintainability and extensibility
- **Conclusion:** Partial Pass
- **Rationale:** Generally maintainable, but password-reset flow overloading `users.password_hash` increases coupling and operational risk; split/merge + invoice flow may allow duplicate financial artifacts on cancelled parent orders.
- **Evidence:** `backend/src/routes/admin.rs:253`, `backend/src/routes/auth.rs:350`, `backend/src/services/order_service.rs:223`, `backend/src/services/order_service.rs:313`, `backend/src/services/order_service.rs:262`, `backend/src/routes/orders.rs:760`, `backend/src/routes/orders.rs:798`, `backend/src/services/order_service.rs:576`

### 4.4 Engineering Details and Professionalism
#### 4.4.1 Error handling/logging/validation/API quality
- **Conclusion:** Partial Pass
- **Rationale:** Good validation and error mapping overall; structured log redaction exists. However, critical admin date-filter API contracts are inconsistent between frontend and backend.
- **Evidence:** `backend/src/logging.rs:64`, `backend/src/logging.rs:91`, `backend/src/routes/orders.rs:264`, `backend/src/routes/auth.rs:120`, `backend/src/models/audit.rs:27`, `frontend/src/api/admin.rs:49`

#### 4.4.2 Product/service realism
- **Conclusion:** Pass
- **Rationale:** Not a toy sample; includes role-guarded APIs, schema immutability for audit log, retention and backup workflows.
- **Evidence:** `backend/migrations/001_initial.sql:377`, `backend/migrations/001_initial.sql:386`, `backend/src/services/retention_service.rs:26`, `backend/src/routes/backup.rs:18`

### 4.5 Prompt Understanding and Requirement Fit
#### 4.5.1 Business understanding and constraints fit
- **Conclusion:** Partial Pass
- **Rationale:** Strong implementation of offline/local payment simulator, state machine, review versioning, conflict-gated field publication, RBAC/object authz. Remaining gaps are mainly admin audit/report date-handling and test realism on frontend.
- **Evidence:** `backend/src/services/payment_simulator.rs:1`, `backend/src/services/order_state_machine.rs:171`, `backend/src/services/review_service.rs:86`, `backend/src/routes/custom_fields.rs:333`, `backend/src/middleware/rbac.rs:68`, `frontend/tests/e2e/test_routes.rs:18`

### 4.6 Aesthetics (frontend/full-stack)
#### 4.6.1 Visual/interaction quality (static assessment)
- **Conclusion:** Cannot Confirm Statistically
- **Rationale:** Static code indicates loading/empty/error/submitting states and interaction handlers, but visual fidelity and responsive rendering need manual browser verification.
- **Evidence:** `frontend/src/pages/home.rs:366`, `frontend/src/pages/checkout.rs:285`, `frontend/src/pages/reviewer/submit.rs:297`, `frontend/styles/main.css:1`

## 5. Issues / Suggestions (Severity-Rated)

### [H-01] Admin Audit Date Filter Contract Mismatch
- **Severity:** High
- **Conclusion:** Fail
- **Evidence:** `backend/src/models/audit.rs:27`, `backend/src/models/audit.rs:28`, `frontend/src/types.rs:570`, `frontend/src/pages/admin/audit_log.rs:56`, `frontend/src/pages/admin/audit_log.rs:126`, `frontend/src/api/admin.rs:49`
- **Impact:** Admin date-filtered audit queries are likely rejected (or inconsistent), weakening a core audit workflow.
- **Minimum actionable fix:** Align contracts by using RFC3339 datetime inputs in frontend or changing backend `from_date/to_date` to `NaiveDate` (plus explicit inclusive day bounds).

### [H-02] Frontend “E2E/Component” Tests Are Mostly Non-Executable Assertions
- **Severity:** High
- **Conclusion:** Fail
- **Evidence:** `frontend/tests/e2e/test_components.rs:3`, `frontend/tests/e2e/test_routes.rs:18`, `frontend/tests/e2e/test_components.rs:119`, `frontend/tests/e2e/test_api_contracts.rs:188`
- **Impact:** Severe UI/router/state regressions can pass test suite because many tests only validate literals/string formatting, not actual rendering or interaction behavior.
- **Minimum actionable fix:** Add real browser-level tests (wasm-bindgen/webdriver or Playwright against built frontend) for critical flows: login, cart->checkout->payment simulation, reviewer submission with attachment, admin audit/report filtering.

### [M-01] Admin Reports UI Allows Empty Dates While Backend Requires Date Range
- **Severity:** Medium
- **Conclusion:** Partial Fail
- **Evidence:** `backend/src/routes/reports.rs:22`, `backend/src/routes/reports.rs:23`, `frontend/src/pages/admin/reports.rs:41`, `frontend/src/types.rs:603`, `frontend/src/types.rs:605`
- **Impact:** Report generation can fail due to missing required params, degrading admin operations.
- **Minimum actionable fix:** Make from/to required in frontend form (and validate before submit), or make backend accept defaults when absent.

### [M-02] Password Reset Flow Overwrites Main Password Hash
- **Severity:** Medium
- **Conclusion:** Partial Fail
- **Evidence:** `backend/src/routes/admin.rs:253`, `backend/src/routes/auth.rs:350`
- **Impact:** Issuing reset tokens immediately replaces login password hash, potentially locking users until reset completion and coupling two credentials into one field.
- **Minimum actionable fix:** Store reset token hash in dedicated columns/table (`reset_token_hash`, `reset_expires_at`) without mutating `password_hash`.

### [M-03] Split/Merge + Invoice Path Risks Duplicate Financial Artifacts
- **Severity:** Medium
- **Conclusion:** Suspected Risk
- **Evidence:** `backend/src/services/order_service.rs:223`, `backend/src/services/order_service.rs:313`, `backend/src/services/order_service.rs:262`, `backend/src/routes/orders.rs:760`, `backend/src/routes/orders.rs:798`, `backend/src/services/order_service.rs:576`
- **Impact:** Because child/merged orders duplicate line items while parent/children are only cancelled (not logically excluded in invoice generation), invoices can be generated for cancelled lineage nodes.
- **Minimum actionable fix:** Gate invoice generation by status/lineage rules (e.g., disallow cancelled parent/child invoice when superseded), and enforce one canonical invoice target per lineage group.

## 6. Security Review Summary
- **Authentication entry points:** **Pass**  
  Evidence: JWT bearer extraction/validation/revocation checks in `backend/src/middleware/auth.rs:26`, `backend/src/middleware/auth.rs:35`, `backend/src/middleware/auth.rs:49`.
- **Route-level authorization:** **Pass**  
  Evidence: role/owner guards in orders/admin/reviews/backup/audit (`backend/src/routes/orders.rs:372`, `backend/src/routes/admin.rs:139`, `backend/src/routes/reviews.rs:132`, `backend/src/routes/backup.rs:32`, `backend/src/routes/audit.rs:26`).
- **Object-level authorization:** **Pass**  
  Evidence: owner/admin checks in orders/reviews (`backend/src/routes/orders.rs:245`, `backend/src/routes/reviews.rs:325`, `backend/src/routes/reviews.rs:520`) and seeded cross-user 403 tests (`backend/tests/api/test_object_authz.rs:333`, `backend/tests/api/test_order_api.rs:122`).
- **Function-level authorization:** **Pass**  
  Evidence: centralized RBAC helpers (`backend/src/middleware/rbac.rs:18`, `backend/src/middleware/rbac.rs:68`).
- **Tenant/user data isolation:** **Partial Pass**  
  Evidence: user-scoped queries/guards exist (`backend/src/routes/orders.rs:140`, `backend/src/routes/cart.rs:113`), but some tests still rely on non-existent IDs/status-code assertions rather than fully seeded multi-tenant data (`backend/tests/api/test_order_api.rs:33`, `backend/tests/api/test_order_api.rs:239`).
- **Admin/internal/debug endpoint protection:** **Pass**  
  Evidence: admin-only protection across admin/backup/audit (`backend/src/routes/admin.rs:139`, `backend/src/routes/backup.rs:32`, `backend/src/routes/audit.rs:26`). No exposed debug routes found statically.

## 7. Tests and Logging Review
- **Unit tests:** **Pass** (backend), **Partial Pass** (frontend)
  - Backend unit coverage exists for auth/state-machine/encryption/rbac/rating aggregation (`backend/tests/unit/test_auth.rs:8`, `backend/tests/unit/test_order_state_machine.rs:10`, `backend/tests/unit/test_encryption.rs:18`).
  - Frontend unit tests are mostly DTO/logic serialization checks, limited behavioral confidence (`frontend/tests/unit/test_store.rs:5`, `frontend/tests/unit/test_types.rs:10`).
- **API/integration tests:** **Partial Pass**
  - Extensive backend API tests with DB-backed harness (`backend/tests/api/common.rs:14`, `backend/tests/api/common.rs:18`).
  - Many cases still test fake IDs and “not 500” rather than strict business assertions (`backend/tests/api/test_order_api.rs:33`, `backend/tests/api/test_order_api.rs:468`).
- **Logging categories/observability:** **Pass**
  - Structured logging + redaction patterns implemented (`backend/src/logging.rs:18`, `backend/src/logging.rs:64`, `backend/src/logging.rs:91`).
- **Sensitive-data leakage risk (logs/responses):** **Partial Pass**
  - Positive: token/password redaction and masked PII output (`backend/src/logging.rs:74`, `backend/src/routes/users.rs:71`).
  - Residual risk: reset token returned in admin response by design (`backend/src/routes/admin.rs:280`); acceptable only under strict admin channel handling.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist: backend and frontend (`backend/tests/unit/...`, `frontend/tests/unit/...`).
- API/integration tests exist: backend Actix tests (`backend/tests/api/...`).
- Frontend “e2e/contract” tests exist but mostly static assertions, not browser execution (`frontend/tests/e2e/test_components.rs:3`, `frontend/tests/e2e/test_routes.rs:1`).
- Frameworks: Rust `#[test]`, `#[actix_web::test]`, sqlx migration-backed test app (`backend/tests/api/common.rs:14`, `backend/tests/api/common.rs:23`).
- Test entry points documented: `README.md:88`, `README.md:93`, `run_tests.sh:1`.

### 8.2 Coverage Mapping Table
| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Auth + JWT + password policy | `backend/tests/unit/test_auth.rs:8`, `backend/tests/api/test_auth_api.rs:7` | token generation/expiry and auth endpoints | sufficient | Limited reset-token abuse scenarios | Add brute-force/abuse tests for reset endpoint |
| Route-level RBAC (Admin/Reviewer/Shopper) | `backend/tests/api/test_rbac_api.rs:12`, `backend/tests/api/test_admin_api.rs:7` | 401/403 checks on protected endpoints | sufficient | Some endpoints still mostly status-only | Add payload-level response assertions for denied actions |
| Object-level authz on orders/reviews/payments | `backend/tests/api/test_object_authz.rs:333`, `backend/tests/api/test_order_api.rs:122` | seeded cross-user 403 expectations | basically covered | review submission cross-user case partly uses nonexistent IDs | Seed real submissions/attachments for reviewer A then assert reviewer B=403 |
| Order state machine + 30-min auto-cancel + 30-day return/refund window | `backend/tests/unit/test_order_state_machine.rs:10`, `backend/tests/unit/test_order_state_machine.rs:343` | explicit transition and reason/window assertions | basically covered | background reconciliation/cron not runtime-verified | Add integration test seeding expired reserved order and asserting cancellation+inventory restore |
| Leaderboard tie-break rules | `backend/tests/unit/test_leaderboard_tiebreak.rs:5`, `backend/tests/api/test_rating_api.rs:188` | ordering by score, count, recency | basically covered | API tests often avoid seeded deterministic dataset | Add DB-seeded deterministic leaderboard ordering test |
| Custom field migration conflict/publish gate | `backend/tests/api/test_field_conflict_workflow.rs:330`, `backend/src/routes/custom_fields.rs:330` | publish blocked when conflicts unresolved | basically covered | limited multi-value conversion edge cases | Add conversion matrix tests for Text/Enum/Date/Number with conflicts |
| Frontend core flow (render + interaction) | `frontend/tests/e2e/test_routes.rs:18`, `frontend/tests/e2e/test_components.rs:119` | literal/string assertions | insufficient | does not execute actual component rendering/router transitions | Add real browser integration tests for critical user journeys |

### 8.3 Security Coverage Audit
- **Authentication tests:** basically covered (token and login paths tested), but reset-token misuse coverage is limited.
- **Route authorization tests:** covered (many 401/403 tests).
- **Object-level authorization tests:** partially covered (good order/payment coverage; review ownership tests still include non-existent-resource patterns).
- **Tenant/data isolation tests:** partially covered (cart/order isolation tested; broader data-scoped queries and list filtering not comprehensively asserted).
- **Admin/internal protection tests:** covered for major admin endpoints.

### 8.4 Final Coverage Judgment
- **Conclusion:** **Partial Pass**
- **Reason:** Backend security/business-rule coverage is substantial; however, frontend test layers do not provide real interaction confidence, and several backend API tests rely on fake IDs/status smoke checks where severe business regressions could still pass.

## 9. Final Notes
- The delivery is close to production-shaped and broadly aligns with the prompt.
- The highest-priority corrective work is contract alignment for admin audit/report date filtering and replacing pseudo-e2e frontend tests with real execution-based coverage.
- Static analysis alone cannot prove runtime behavior for backup scheduling, restore safety, and final UI rendering quality.
