# Static Audit Report - SilverScreen Commerce & Review Platform

## 1. Verdict
- **Overall conclusion:** **Partial Pass**

## 2. Scope and Static Verification Boundary
- **Reviewed:** repository docs/config/scripts, backend route wiring/middleware/services/models/migrations, frontend routes/pages/API client/types, backend and frontend tests.
- **Excluded from evidence:** `./.tmp/**`.
- **Intentionally not executed:** app startup, tests, Docker/Compose, browser runtime, external services.
- **Cannot be statically confirmed:** runtime scheduler correctness (nightly backup timing), restore safety in production data, final browser rendering fidelity, end-to-end UX behavior under real interactions and network failures.
- **Manual verification required for:** backup/restore operational runbook, file-download watermark behavior in real clients, full UI interaction quality, and real browser E2E flows.

## 3. Repository / Requirement Mapping Summary
- **Prompt core goals mapped:** offline-first ordering + local payment simulation, structured ratings/leaderboards tie-break, reviewer multi-round submission with history/attachments, admin moderation/audit/reporting, security controls (auth/RBAC/object checks/rate-limit/risk), retention and encrypted backups.
- **Mapped implementation areas:**
  - Backend API and role boundaries: `backend/src/routes/mod.rs:18`, `backend/src/routes/orders.rs:140`, `backend/src/routes/reviews.rs:130`, `backend/src/routes/admin.rs:139`, `backend/src/routes/backup.rs:32`
  - Core business/state logic: `backend/src/services/order_service.rs:94`, `backend/src/services/order_state_machine.rs:143`, `backend/src/services/rating_service.rs:158`, `backend/src/services/review_service.rs:20`, `backend/src/services/field_migration_service.rs:262`
  - Persistence/security baseline: `backend/migrations/001_initial.sql:14`, `backend/migrations/001_initial.sql:362`, `backend/migrations/001_initial.sql:387`, `backend/src/middleware/auth.rs:26`, `backend/src/logging.rs:64`
  - Frontend flow wiring: `frontend/src/app.rs:25`, `frontend/src/pages/home.rs:63`, `frontend/src/pages/checkout.rs:202`, `frontend/src/pages/reviewer/submit.rs:196`, `frontend/src/pages/admin/dashboard.rs:10`

## 4. Section-by-section Review

### 4.1 Hard Gates
#### 4.1.1 Documentation and static verifiability
- **Conclusion:** Pass
- **Rationale:** Startup/config/test instructions and project structure are present and statically consistent with files/scripts.
- **Evidence:** `README.md:5`, `README.md:36`, `README.md:83`, `README.md:114`, `.env.example:3`, `run_tests.sh:101`

#### 4.1.2 Material deviation from prompt
- **Conclusion:** Partial Pass
- **Rationale:** Core flows are implemented, but two notable fit gaps remain: evidence watermark is header-level (not embedded in file content), and moderation UX is ID-driven without a pending-queue listing endpoint.
- **Evidence:** `backend/src/routes/reviews.rs:559`, `backend/src/routes/reviews.rs:561`, `frontend/src/pages/admin/moderation.rs:22`, `frontend/src/pages/admin/moderation.rs:100`

### 4.2 Delivery Completeness
#### 4.2.1 Core explicit requirements coverage
- **Conclusion:** Partial Pass
- **Rationale:** Most explicit requirements are present: 30-min reservation + auto-cancel, return/refund reason/window checks, local payment simulation, ratings/leaderboards tie-break, custom-field migration/publish gating, retention, backup. Remaining partials are moderation UX and watermark implementation style.
- **Evidence:** `backend/src/services/order_service.rs:94`, `backend/src/main.rs:68`, `backend/src/services/order_state_machine.rs:182`, `backend/src/services/payment_simulator.rs:23`, `backend/src/services/rating_service.rs:188`, `backend/src/routes/custom_fields.rs:330`, `backend/src/services/retention_service.rs:30`, `backend/src/main.rs:81`

#### 4.2.2 End-to-end deliverable shape
- **Conclusion:** Pass
- **Rationale:** Coherent full-stack repository with backend/frontend modules, migrations, docs, and test suites.
- **Evidence:** `README.md:47`, `backend/Cargo.toml:1`, `frontend/Cargo.toml:1`, `backend/migrations/001_initial.sql:1`

### 4.3 Engineering and Architecture Quality
#### 4.3.1 Structure and module decomposition
- **Conclusion:** Pass
- **Rationale:** Clear separation of routes/services/models/middleware in backend and pages/components/api/types in frontend.
- **Evidence:** `backend/src/routes/mod.rs:18`, `backend/src/services/mod.rs:1`, `backend/src/middleware/mod.rs:1`, `frontend/src/app.rs:5`, `frontend/src/api/mod.rs:1`

#### 4.3.2 Maintainability and extensibility
- **Conclusion:** Partial Pass
- **Rationale:** Core architecture is maintainable, but moderation workflow is operationally constrained (manual rating-ID moderation) and watermark implementation relies on transport headers only.
- **Evidence:** `frontend/src/pages/admin/moderation.rs:22`, `frontend/src/pages/admin/moderation.rs:100`, `backend/src/routes/reviews.rs:559`, `backend/src/routes/reviews.rs:561`

### 4.4 Engineering Details and Professionalism
#### 4.4.1 Error handling/logging/validation/API quality
- **Conclusion:** Partial Pass
- **Rationale:** Strong input validation/error mapping/log redaction are present; however, parts of test strategy remain weak for behavior confidence (many non-execution assertions, conditional skips).
- **Evidence:** `backend/src/routes/auth.rs:120`, `backend/src/services/review_service.rs:199`, `backend/src/logging.rs:64`, `frontend/tests/e2e/mod.rs:3`, `backend/tests/api/test_review_api.rs:650`

#### 4.4.2 Product/service realism
- **Conclusion:** Pass
- **Rationale:** Implementation is product-shaped (RBAC/object checks, immutable audit log trigger, retention/legal hold, encrypted local backup/restore).
- **Evidence:** `backend/src/routes/orders.rs:245`, `backend/migrations/001_initial.sql:387`, `backend/src/services/retention_service.rs:48`, `backend/src/services/backup_service.rs:24`

### 4.5 Prompt Understanding and Requirement Fit
#### 4.5.1 Business understanding and constraints fit
- **Conclusion:** Partial Pass
- **Rationale:** Strong alignment on offline/local payments, strict lifecycle automation, rating tie-break logic, review version trail, and admin auditing; residual fit gaps are moderation usability and watermark style.
- **Evidence:** `backend/src/services/payment_simulator.rs:1`, `backend/src/services/order_state_machine.rs:158`, `backend/src/services/rating_service.rs:159`, `backend/src/services/review_service.rs:100`, `backend/src/routes/reviews.rs:559`, `frontend/src/pages/admin/moderation.rs:22`

### 4.6 Aesthetics (frontend/full-stack)
#### 4.6.1 Visual/interaction quality (static assessment)
- **Conclusion:** Cannot Confirm Statistically
- **Rationale:** Static code shows loading/error/empty/submitting states and organized page composition, but visual quality/responsiveness/interaction polish needs browser verification.
- **Evidence:** `frontend/src/pages/home.rs:358`, `frontend/src/pages/checkout.rs:217`, `frontend/src/pages/reviewer/submit.rs:324`, `frontend/styles/main.css:1`

## 5. Issues / Suggestions (Severity-Rated)

### [H-01] Test Confidence Gap for Critical Flows (UI + review attachment workflow)
- **Severity:** High
- **Conclusion:** Fail
- **Evidence:** `frontend/tests/e2e/mod.rs:3`, `frontend/tests/e2e/test_routes.rs:1`, `frontend/tests/wasm/test_browser.rs:24`, `backend/tests/api/test_review_api.rs:650`, `backend/tests/api/test_order_api.rs:603`
- **Impact:** Critical regressions can pass static test suites because many tests are type/contract checks, non-seeded ID checks, or conditional-skipped integration paths.
- **Minimum actionable fix:** Add deterministic seeded integration/browser tests for core business journeys: cart->checkout->payment success/failure, reviewer submission + attachment approval + download watermark assertion, admin moderation/report/audit filtering with real seeded data.

### [M-01] Evidence Watermark Is Header-Only, Not Embedded in Downloaded File Content
- **Severity:** Medium
- **Conclusion:** Partial Fail
- **Evidence:** `backend/src/routes/reviews.rs:559`, `backend/src/routes/reviews.rs:561`, `backend/src/routes/reviews.rs:570`
- **Impact:** Watermark may be lost when the file is saved/shared, reducing evidentiary traceability versus embedded watermarking.
- **Minimum actionable fix:** Add content-level watermarking for supported file types (at least PDF/image), keep `X-Watermark` as supplemental metadata.

### [M-02] Admin Moderation Workflow Is Manual-by-ID Without Pending Queue Listing
- **Severity:** Medium
- **Conclusion:** Partial Fail
- **Evidence:** `frontend/src/pages/admin/moderation.rs:22`, `frontend/src/pages/admin/moderation.rs:100`
- **Impact:** Operational moderation is less usable/scalable; admins need prior knowledge of IDs instead of reviewing a pending queue.
- **Minimum actionable fix:** Implement/list a pending moderation endpoint and render a queue with inline approve/reject actions.

## 6. Security Review Summary
- **Authentication entry points:** **Pass**  
  Evidence: JWT extract/validate/revocation in `backend/src/middleware/auth.rs:26`, login/refresh/logout token-type checks in `backend/src/routes/auth.rs:120`, `backend/src/routes/auth.rs:204`, `backend/src/routes/auth.rs:258`.
- **Route-level authorization:** **Pass**  
  Evidence: role guards across admin/audit/backup/reports/reviews/orders: `backend/src/routes/admin.rs:139`, `backend/src/routes/audit.rs:26`, `backend/src/routes/backup.rs:32`, `backend/src/routes/reports.rs:84`, `backend/src/routes/reviews.rs:132`, `backend/src/routes/orders.rs:372`.
- **Object-level authorization:** **Pass**  
  Evidence: owner/admin checks on orders/users/payment/review attachments: `backend/src/routes/orders.rs:245`, `backend/src/routes/users.rs:213`, `backend/src/routes/payment.rs:52`, `backend/src/routes/reviews.rs:520`.
- **Function-level authorization:** **Pass**  
  Evidence: centralized RBAC helpers used broadly: `backend/src/middleware/rbac.rs:18`, `backend/src/middleware/rbac.rs:42`, `backend/src/middleware/rbac.rs:69`.
- **Tenant/user data isolation:** **Partial Pass**  
  Evidence: user-scoped order/cart/review queries exist (`backend/src/routes/orders.rs:157`, `backend/src/routes/cart.rs:49`, `backend/src/routes/reviews.rs:213`), but part of test coverage still uses non-seeded/nonexistent-resource checks (`backend/tests/api/test_review_api.rs:35`, `backend/tests/api/test_order_api.rs:33`).
- **Admin/internal/debug endpoint protection:** **Pass**  
  Evidence: admin-only constraints for backup/audit/reports/retention and no exposed debug route found: `backend/src/routes/backup.rs:32`, `backend/src/routes/audit.rs:26`, `backend/src/routes/reports.rs:84`, `backend/src/routes/admin.rs:558`.

## 7. Tests and Logging Review
- **Unit tests:** **Pass** (backend), **Partial Pass** (frontend)
  - Backend unit tests cover auth/state machine/rbac/rating/encryption (`backend/tests/unit/test_auth.rs:8`, `backend/tests/unit/test_order_state_machine.rs:10`, `backend/tests/unit/test_rbac.rs:1`, `backend/tests/unit/test_leaderboard_tiebreak.rs:1`).
  - Frontend unit/e2e-labeled tests are largely non-rendering logic assertions (`frontend/tests/e2e/test_components.rs:8`, `frontend/tests/e2e/test_routes.rs:8`).
- **API/integration tests:** **Partial Pass**
  - Extensive backend API test files exist (`backend/tests/api/common.rs:11`, `backend/tests/api/test_order_api.rs:1`, `backend/tests/api/test_object_authz.rs:1`).
  - Some high-value flows rely on skip-paths or non-seeded fake IDs (`backend/tests/api/test_review_api.rs:650`, `backend/tests/api/test_order_api.rs:603`).
- **Logging categories/observability:** **Pass**
  - Structured logging + redaction implemented (`backend/src/logging.rs:8`, `backend/src/logging.rs:64`, `backend/src/logging.rs:82`).
- **Sensitive-data leakage risk (logs/responses):** **Partial Pass**
  - Positive: log redaction and masked PII response patterns (`backend/src/logging.rs:64`, `backend/src/services/encryption_service.rs:102`, `backend/src/routes/users.rs:63`).
  - Residual risk: admin reset endpoint returns raw reset token by design (`backend/src/routes/admin.rs:278`, `backend/src/routes/admin.rs:282`), requiring strict operational handling.

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview
- Unit tests exist for backend and frontend (`backend/tests/unit/mod.rs:1`, `frontend/tests/unit/mod.rs:1`).
- Backend API/integration tests exist with DB-backed test app (`backend/tests/api/common.rs:11`, `backend/tests/api/common.rs:19`).
- Frontend has e2e-labeled type/contract tests and separate wasm browser tests (`frontend/tests/e2e/mod.rs:3`, `frontend/tests/wasm/mod.rs:1`).
- Test entry points documented in README and script (`README.md:83`, `README.md:117`, `run_tests.sh:101`, `run_tests.sh:119`).
- Browser wasm tests are optional and skipped when `wasm-pack` is missing (`run_tests.sh:118`, `run_tests.sh:135`).

### 8.2 Coverage Mapping Table
| Requirement / Risk Point | Mapped Test Case(s) | Key Assertion / Fixture / Mock | Coverage Assessment | Gap | Minimum Test Addition |
|---|---|---|---|---|---|
| Auth + JWT + refresh/logout revocation | `backend/tests/api/test_auth_api.rs:7`, `backend/tests/unit/test_auth.rs:8` | token issuance/validation + endpoint responses | basically covered | deeper token abuse/replay scenarios limited | add token replay/revocation race tests |
| Route RBAC (Admin/Reviewer/Shopper) | `backend/tests/api/test_rbac_api.rs:13`, `backend/tests/api/test_review_api.rs:503` | 401/403 checks across protected routes | sufficient | some checks remain non-seeded | add seeded endpoint-behavior assertions for key admin flows |
| Object-level authz (orders/payment/reviews) | `backend/tests/api/test_object_authz.rs:333`, `backend/tests/api/test_order_api.rs:51` | seeded cross-user 403 for orders/payment | partially covered | review-submission cross-user tests mostly 404 non-existent resource pattern | add seeded round/submission/attachment ownership tests with real resources |
| Order lifecycle/return-window/reason codes | `backend/tests/unit/test_order_state_machine.rs:10`, `backend/tests/api/test_order_api.rs:531` | transition validity + reason-code path assertions | basically covered | many API checks use fake order IDs | add seeded delivered-order >30-day scenario and rollback assertions |
| Leaderboard tie-break (score/count/recency) | `backend/tests/unit/test_leaderboard_tiebreak.rs:27`, `backend/tests/api/test_rating_api.rs:188` | deterministic sorting logic | basically covered | mostly unit-level for tie-break; limited seeded API determinism | add seeded API ranking dataset test |
| Custom field migration conflict gating | `backend/tests/api/test_field_conflict_workflow.rs:330` | publish blocked until conflicts resolved | basically covered | conversion matrix edges can expand | add broad conversion matrix tests for Text/Enum/Date/Number |
| Frontend critical journeys (catalog->checkout->payment, reviewer submit/attachments, admin moderation/audit/report) | `frontend/tests/e2e/mod.rs:3`, `frontend/tests/e2e/test_components.rs:8`, `frontend/tests/wasm/test_browser.rs:24` | mostly type/serialization/DOM smoke | insufficient | no deterministic browser journey coverage for core business workflows | add Playwright/wasm-browser scenario tests for the critical workflows |

### 8.3 Security Coverage Audit
- **Authentication tests:** basically covered; token paths are tested, but abuse/race edges are thinner.
- **Route authorization tests:** covered; broad 401/403 checks across admin/reviewer/shopper endpoints.
- **Object-level authorization tests:** partially covered; strong seeded order/payment cases exist, but review object-ownership scenarios still often rely on non-existent IDs.
- **Tenant/data isolation tests:** partially covered; scoped queries and some seeded cross-user checks exist, but not all list/filter paths are deeply seeded.
- **Admin/internal protection tests:** covered for major admin surfaces (backup/audit/admin endpoints).

### 8.4 Final Coverage Judgment
- **Conclusion:** **Partial Pass**
- **Reason:** Core backend security/business logic has meaningful coverage, but severe regressions could still slip through because many frontend tests are non-execution contract checks and several backend high-risk review/attachment paths depend on skip logic or non-seeded resource patterns.

## 9. Final Notes
- This delivery is close to production-shaped and mostly aligned with the prompt.
- Highest priority is strengthening deterministic execution-based tests for critical end-to-end flows.
- Static analysis cannot prove runtime scheduling accuracy, browser UX quality, or operational restore safety.
