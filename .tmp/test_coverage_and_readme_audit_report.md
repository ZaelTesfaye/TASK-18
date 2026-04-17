# Test Coverage Audit

## Project Type Detection
- Declared type: `fullstack` ([repo/README.md:1](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:1)).
- Inference check: backend (`repo/backend`) + frontend (`repo/frontend`) both present.

## Backend Endpoint Inventory
- Source routing root: `/api` scope in [repo/backend/src/routes/mod.rs:21](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/src/routes/mod.rs:21).
- Health route: `GET /health` in [repo/backend/src/main.rs:152](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/src/main.rs:152).
- Total endpoints identified: **73**.

## API Test Mapping Table
| Endpoint | Covered | Test Type | Test Files | Evidence |
|---|---|---|---|---|
| DELETE /api/cart | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs | repo/backend/tests/api/test_cart_api.rs:56 |
| DELETE /api/cart/items/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs | repo/backend/tests/api/test_cart_api.rs:192 |
| DELETE /api/products/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_product_api.rs | repo/backend/tests/api/test_product_api.rs:133 |
| DELETE /api/ratings/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:349 |
| DELETE /api/taxonomy/tags/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_taxonomy_api.rs:115 |
| DELETE /api/taxonomy/topics/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_taxonomy_api.rs:172 |
| GET /api/admin/risk-events | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/test_admin_api.rs:127 |
| GET /api/admin/users | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_admin_api.rs:12 |
| GET /api/audit | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_admin_api.rs:161 |
| GET /api/backup | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs | repo/backend/tests/api/test_admin_api.rs:316 |
| GET /api/cart | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_cart_api.rs:10 |
| GET /api/custom-fields | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:394 |
| GET /api/custom-fields/:id/conflicts | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:88 |
| GET /api/leaderboards | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:90 |
| GET /api/orders | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_object_authz.rs:95 |
| GET /api/orders/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_object_authz.rs:326 |
| GET /api/orders/:id/invoice | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:894 |
| GET /api/products | yes | true no-mock HTTP | repo/backend/tests/api/common.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_product_api.rs | repo/backend/tests/api/common.rs:169 |
| GET /api/products/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_product_api.rs | repo/backend/tests/api/test_product_api.rs:98 |
| GET /api/ratings/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:266 |
| GET /api/ratings/product/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:77 |
| GET /api/reports | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_admin_api.rs:196 |
| GET /api/reviews/attachments/:id/download | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_rbac_api.rs, repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_object_authz.rs:159 |
| GET /api/reviews/rounds | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs, repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_rbac_api.rs:432 |
| GET /api/reviews/rounds/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_review_api.rs:218 |
| GET /api/reviews/submissions/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_rbac_api.rs, repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_object_authz.rs:133 |
| GET /api/reviews/submissions/:id/history | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_object_authz.rs:451 |
| GET /api/taxonomy/tags | yes | true no-mock HTTP | repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_taxonomy_api.rs:28 |
| GET /api/taxonomy/topics | yes | true no-mock HTTP | repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_taxonomy_api.rs:64 |
| GET /api/users/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_user_api.rs | repo/backend/tests/api/test_user_api.rs:19 |
| GET /api/users/me | yes | true no-mock HTTP | repo/backend/tests/api/common.rs, repo/backend/tests/api/test_auth_api.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/common.rs:144 |
| GET /health | yes | true no-mock HTTP | repo/backend/tests/api/test_health_api.rs | repo/backend/tests/api/test_health_api.rs:43 |
| POST /api/admin/moderation/ratings/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_rbac_api.rs:125 |
| POST /api/admin/retention/legal-hold/:order_id | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs | repo/backend/tests/api/test_admin_api.rs:252 |
| POST /api/admin/retention/run | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs | repo/backend/tests/api/test_admin_api.rs:230 |
| POST /api/admin/users/:id/reset-password | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_auth_api.rs | repo/backend/tests/api/test_admin_api.rs:92 |
| POST /api/admin/users/:id/unlock | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs | repo/backend/tests/api/test_admin_api.rs:110 |
| POST /api/auth/login | yes | true no-mock HTTP | repo/backend/tests/api/common.rs, repo/backend/tests/api/test_auth_api.rs, repo/backend/tests/api/test_cart_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/common.rs:127 |
| POST /api/auth/logout | yes | true no-mock HTTP | repo/backend/tests/api/test_auth_api.rs | repo/backend/tests/api/test_auth_api.rs:252 |
| POST /api/auth/refresh | yes | true no-mock HTTP | repo/backend/tests/api/test_auth_api.rs | repo/backend/tests/api/test_auth_api.rs:204 |
| POST /api/auth/register | yes | true no-mock HTTP | repo/backend/tests/api/common.rs, repo/backend/tests/api/test_auth_api.rs, repo/backend/tests/api/test_cart_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/common.rs:113 |
| POST /api/auth/reset-password | yes | true no-mock HTTP | repo/backend/tests/api/test_auth_api.rs | repo/backend/tests/api/test_auth_api.rs:507 |
| POST /api/backup | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_admin_api.rs:213 |
| POST /api/backup/:id/restore | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_rbac_api.rs:274 |
| POST /api/backup/:id/verify | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_rbac_api.rs:255 |
| POST /api/cart/items | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs, repo/backend/tests/api/test_object_authz.rs | repo/backend/tests/api/test_cart_api.rs:39 |
| POST /api/custom-fields | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:16 |
| POST /api/custom-fields/:id/publish | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:55 |
| POST /api/orders | yes | true no-mock HTTP | repo/backend/tests/api/common.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rating_api.rs, repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/common.rs:180 |
| POST /api/orders/:id/exchange | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:409 |
| POST /api/orders/:id/merge | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:258 |
| POST /api/orders/:id/refund | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:484 |
| POST /api/orders/:id/return | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:160 |
| POST /api/orders/:id/split | yes | true no-mock HTTP | repo/backend/tests/api/test_order_api.rs | repo/backend/tests/api/test_order_api.rs:241 |
| POST /api/payment/simulate | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rbac_api.rs | repo/backend/tests/api/test_object_authz.rs:506 |
| POST /api/products | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs, repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_product_api.rs, repo/backend/tests/api/test_rating_api.rs, repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/test_cart_api.rs:72 |
| POST /api/ratings | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:11 |
| POST /api/reviews/attachments/:id/approve | yes | true no-mock HTTP | repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_review_api.rs:272 |
| POST /api/reviews/rounds/:id/submit | yes | true no-mock HTTP | repo/backend/tests/api/test_review_api.rs | repo/backend/tests/api/test_review_api.rs:116 |
| POST /api/reviews/submissions/:id/attachments | no | unit-only / indirect | - | repo/backend/src/routes/reviews.rs:25 |
| POST /api/taxonomy/tags | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs, repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_rbac_api.rs:58 |
| POST /api/taxonomy/topics | yes | true no-mock HTTP | repo/backend/tests/api/test_rbac_api.rs, repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_rbac_api.rs:17 |
| POST /api/users/me/unmask | yes | true no-mock HTTP | repo/backend/tests/api/test_user_api.rs | repo/backend/tests/api/test_user_api.rs:83 |
| PUT /api/admin/risk-events/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_risk_event_schema.rs | repo/backend/tests/api/test_risk_event_schema.rs:52 |
| PUT /api/admin/users/:id/role | yes | true no-mock HTTP | repo/backend/tests/api/test_admin_api.rs, repo/backend/tests/api/test_object_authz.rs | repo/backend/tests/api/test_admin_api.rs:48 |
| PUT /api/cart/items/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_cart_api.rs | repo/backend/tests/api/test_cart_api.rs:264 |
| PUT /api/custom-fields/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:148 |
| PUT /api/custom-fields/:id/conflicts/:product_id | yes | true no-mock HTTP | repo/backend/tests/api/test_field_conflict_workflow.rs | repo/backend/tests/api/test_field_conflict_workflow.rs:433 |
| PUT /api/orders/:id/status | yes | true no-mock HTTP | repo/backend/tests/api/test_object_authz.rs, repo/backend/tests/api/test_order_api.rs, repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_object_authz.rs:391 |
| PUT /api/products/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_product_api.rs | repo/backend/tests/api/test_product_api.rs:176 |
| PUT /api/ratings/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_rating_api.rs | repo/backend/tests/api/test_rating_api.rs:437 |
| PUT /api/taxonomy/topics/:id | yes | true no-mock HTTP | repo/backend/tests/api/test_taxonomy_api.rs | repo/backend/tests/api/test_taxonomy_api.rs:235 |
| PUT /api/users/me | yes | true no-mock HTTP | repo/backend/tests/api/test_user_api.rs | repo/backend/tests/api/test_user_api.rs:43 |

## API Test Classification
- True No-Mock HTTP: 14 API test files (`repo/backend/tests/api/test_*.rs`), app bootstrapped via `test::init_service(...configure(configure_routes))` in [repo/backend/tests/api/common.rs:30](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/api/common.rs:30).
- HTTP with Mocking: **none found** (no `jest.mock`, `vi.mock`, `sinon.stub`, `mockall`, `mockito` in backend API tests).
- Non-HTTP (unit/integration without HTTP): none in `backend/tests/api`; these exist in `backend/tests/unit` by design.

## Mock Detection
- Static grep for common mocking/stubbing patterns across backend/frontend tests returned no actual mocking framework usage (only variable names like `fake_id`). Evidence search executed over `repo/backend/tests` and `repo/frontend/tests`.
- API tests execute real route handlers with real middleware chain (`configure_routes`) and real DB pool initialization in [repo/backend/tests/api/common.rs:17](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/api/common.rs:17).

## Coverage Summary
- Total endpoints: **73**
- Endpoints with HTTP tests: **72**
- Endpoints with TRUE no-mock tests: **72**
- HTTP coverage: **98.63%** (72/73)
- True API coverage: **98.63%** (72/73)
- Uncovered endpoint: `POST /api/reviews/submissions/:id/attachments` ([repo/backend/src/routes/reviews.rs:25](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/src/routes/reviews.rs:25)).

## Unit Test Summary
### Backend Unit Tests
- Unit test files found: **19** under `repo/backend/tests/unit`.
- Modules covered (evidence examples):
  - Services: auth, order_state_machine, order_service, review_service, backup, retention, taxonomy, encryption ([repo/backend/tests/unit/test_auth.rs:1](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/unit/test_auth.rs:1), [repo/backend/tests/unit/test_order_state_machine.rs:2](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/unit/test_order_state_machine.rs:2)).
  - Middleware/guards: rbac, rate_limit, request logger, risk ([repo/backend/tests/unit/test_rbac.rs:2](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/unit/test_rbac.rs:2), [repo/backend/tests/unit/test_rate_limit.rs:1](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/unit/test_rate_limit.rs:1)).
  - Models/contracts: broad serialization/shape checks in [repo/backend/tests/unit/test_contract.rs:14](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/unit/test_contract.rs:14).
- Important backend modules NOT unit-tested directly:
  - Route handler modules in `repo/backend/src/routes/*.rs` (covered via API tests, not unit tests).
  - DB bootstrap/error modules (`src/db.rs`, `src/errors.rs`) do not have dedicated unit tests.

### Frontend Unit Tests (STRICT REQUIREMENT)
- Frontend test files detected: `repo/frontend/tests/unit/*.test.rs`, `repo/frontend/tests/component/*.test.rs`, plus browser wasm tests in `repo/frontend/tests/wasm/*.test.rs`.
- Frameworks/tools detected:
  - Rust built-in test harness (`#[test]`) across unit/component tests ([repo/frontend/tests/unit/types.test.rs:10](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/unit/types.test.rs:10)).
  - `wasm-bindgen-test` for browser-level frontend tests ([repo/frontend/Cargo.toml](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/Cargo.toml), [repo/frontend/tests/wasm/browser.test.rs:42](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/wasm/browser.test.rs:42)).
- Components/modules covered (imports/render-path evidence):
  - Components: navbar, product_card, pagination, toast, loading, rating_stars ([repo/frontend/tests/component/components.test.rs:9](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/component/components.test.rs:9)).
  - Pages/modules: home/admin/reviewer routing and page logic ([repo/frontend/tests/unit/pages.test.rs:9](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/unit/pages.test.rs:9), [repo/frontend/tests/component/routes.test.rs:11](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/component/routes.test.rs:11)).
  - Store/types/api-client logic ([repo/frontend/tests/unit/store.test.rs:10](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/unit/store.test.rs:10), [repo/frontend/tests/unit/api_client.test.rs:20](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/unit/api_client.test.rs:20)).
- Important frontend components/modules NOT tested deeply (or only indirectly):
  - Concrete page modules like `pages/login.rs`, `pages/register.rs`, `pages/checkout.rs`, `pages/order_detail.rs` lack direct behavioral tests by file name; coverage is mostly route/type/logic-level.
  - API domain modules (`src/api/auth.rs`, `cart.rs`, `orders.rs`, `reviews.rs`, `admin.rs`) are not individually exercised with per-module unit tests; testing is mostly through shared client/type logic and wasm flows.

**Frontend unit tests: PRESENT**

### Cross-Layer Observation
- Testing is reasonably balanced: heavy backend API coverage + substantial frontend unit/component + browser wasm tests. Not backend-only.

## API Observability Check
- Strong overall: most tests specify explicit method/path, request payload, and assert response JSON fields (example: [repo/backend/tests/api/test_product_api.rs:129](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/api/test_product_api.rs:129)).
- Weak spots: some tests assert only status code without response-body assertions (example: [repo/backend/tests/api/test_order_api.rs:954](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/api/test_order_api.rs:954), [repo/backend/tests/api/test_review_api.rs:355](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/backend/tests/api/test_review_api.rs:355)).

## Test Quality & Sufficiency
- Success/failure/authz coverage is broad (auth, RBAC, object-level checks, conflict workflows, risk events, retention/legal-hold).
- Edge-case depth exists in orders/ratings/reviews/custom-fields; however one functional upload endpoint is not covered.
- `run_tests.sh` assessment: Docker-first and comprehensive, but includes explicit local escape hatch `ALLOW_LOCAL_RUN=true`, which violates strict Docker-only expectation ([repo/run_tests.sh:22](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/run_tests.sh:22)). **FLAGGED**.

## End-to-End Expectations (Fullstack)
- Frontend-to-backend path is present via wasm/browser e2e flow tests ([repo/frontend/tests/wasm/e2e_flow.test.rs:86](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/frontend/tests/wasm/e2e_flow.test.rs:86)).
- Compensation not needed; fullstack e2e intent is visible in tests.

## Tests Check
- Static-only audit performed; no execution performed.
- API bootstrapping without mocks confirmed in test harness.
- One uncovered backend endpoint found.
- Frontend unit tests explicitly present and evidenced.

## Test Coverage Score (0-100)
- **91/100**

## Score Rationale
- + High true no-mock API coverage (72/73).
- + Broad backend unit and frontend unit/component/wasm coverage.
- - Missing test for `POST /api/reviews/submissions/:id/attachments` (material endpoint gap).
- - Some API tests assert status without deep payload assertions.
- - `run_tests.sh` allows local non-Docker mode (policy inconsistency).

## Key Gaps
- Missing HTTP tests for review attachment upload endpoint.
- Inconsistent strict Docker-only policy due to `ALLOW_LOCAL_RUN` path in test runner.
- Select API tests are shallow (status-only checks).

## Confidence & Assumptions
- Confidence: **high** for endpoint inventory and API mapping (computed from route and test files).
- Assumption: only routes mounted through `configure_routes` and `/health` in `main.rs` are in audit scope.
- Assumption: static evidence of no mocking is limited to visible source patterns; runtime monkey-patching is not inferable statically.

## Test Coverage Verdict
- **PARTIAL PASS** (single uncovered endpoint + policy inconsistency in test runner).

---

# README Audit

## Hard Gate Evaluation
- README location: present at [repo/README.md](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md).
- Project type declaration at top: `Project Type: fullstack` ([repo/README.md:1](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:1)) - PASS.
- Startup instructions (`docker-compose up` for fullstack/backend): present as `docker-compose up --build` ([repo/README.md:38](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:38)) - PASS.
- Access method (URL + port): frontend/backend URLs and DB port listed ([repo/README.md:43](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:43)) - PASS.
- Verification method: explicit curl health/login checks and UI role flows ([repo/README.md:67](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:67), [repo/README.md:77](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:77)) - PASS.
- Environment rules (no runtime package-manager installs/manual DB setup): README explicitly states Docker-contained dependencies and no local toolchain required ([repo/README.md:49](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:49)) - PASS.
- Demo credentials for auth roles: Admin/Reviewer/Shopper credentials provided ([repo/README.md:85](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:85)) - PASS.

## Engineering Quality
- Tech stack and architecture are clearly documented with directory-level structure and role model.
- Testing instructions are present (`./run_tests.sh`) and aligned with Docker-first workflow.
- Security and role guidance is explicit and substantial.
- Presentation issue: file contains visible encoding artifacts (`—`, box-drawing corruption) reducing readability in plain UTF-8 rendering (examples at [repo/README.md:20](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:20), [repo/README.md:100](D:/Documents/Dev/Projects/Work/Eaglepoint/w2t18/repo/README.md:100)).

## High Priority Issues
- None.

## Medium Priority Issues
- Character encoding corruption in multiple README lines impacts readability/professional presentation.

## Low Priority Issues
- Minor wording inconsistency: environment section says variables are defined in `docker-compose.yml`, while secrets are also described as sourced from `.env`; clarify source-of-truth phrasing to avoid ambiguity.

## Hard Gate Failures
- None.

## README Verdict
- **PASS**

## Final Combined Verdicts
- Test Coverage Audit: **PARTIAL PASS**
- README Audit: **PASS**
