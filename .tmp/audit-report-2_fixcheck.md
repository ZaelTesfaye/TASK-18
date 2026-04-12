# Fix Check Report: audit_report-12

## Source Note

- Requested source file `.tmp/audit-report-12.md` was not found.
- Verified against `.tmp/audit_report-12.md` (existing file in this workspace).

## Overall Result

- **Fixed:** 4
- **Partially Fixed:** 1
- **Not Fixed:** 0

## Issue-by-Issue Verification

### [H-01] Admin Audit Date Filter Contract Mismatch

- **Status:** **Fixed**
- **Why:** Backend now accepts flexible date inputs (`RFC3339` or `YYYY-MM-DD`) and parses both safely; frontend also normalizes bare dates to RFC3339 before sending.
- **Evidence:**
  - `backend/src/models/audit.rs:22`, `backend/src/models/audit.rs:50`, `backend/src/models/audit.rs:66`
  - `backend/src/services/audit_service.rs:83`, `backend/src/services/audit_service.rs:86`, `backend/src/services/audit_service.rs:90`
  - `frontend/src/api/admin.rs:52`, `frontend/src/api/admin.rs:59`

### [H-02] Frontend �E2E/Component� Tests Mostly Non-Executable Assertions

- **Status:** **Partially Fixed**
- **Why:** A browser-test suite was added under `frontend/tests/wasm/` using `wasm_bindgen_test`, but most `frontend/tests/e2e/*` tests are still type/value assertions rather than full app-flow interaction tests (login/cart/checkout/reviewer/admin filtering end-to-end).
- **Evidence:**
  - `frontend/tests/wasm/test_browser.rs:23`, `frontend/tests/wasm/test_browser.rs:32`
  - `frontend/tests/e2e/test_components.rs:3`
  - `frontend/tests/e2e/test_routes.rs:1`

### [M-01] Admin Reports UI Allows Empty Dates While Backend Requires Date Range

- **Status:** **Fixed**
- **Why:** Backend no longer requires both dates; it now accepts optional `start_date`/`end_date` and applies defaults (`today-30d` to `today`) when omitted.
- **Evidence:**
  - `backend/src/routes/reports.rs:23`, `backend/src/routes/reports.rs:25`
  - `backend/src/routes/reports.rs:98`, `backend/src/routes/reports.rs:99`

### [M-02] Password Reset Flow Overwrites Main Password Hash

- **Status:** **Fixed**
- **Why:** Admin reset now stores token hash/expiry in dedicated fields (`reset_token_hash`, `reset_token_expires_at`) without mutating `password_hash`; `password_hash` changes only when user redeems token.
- **Evidence:**
  - `backend/src/models/user.rs:25`, `backend/src/models/user.rs:26`
  - `backend/src/routes/admin.rs:244`, `backend/src/routes/admin.rs:255`
  - `backend/src/routes/auth.rs:325`, `backend/src/routes/auth.rs:362`

### [M-03] Split/Merge + Invoice Path Risks Duplicate Financial Artifacts

- **Status:** **Fixed**
- **Why:** Invoice generation now blocks cancelled orders that have lineage children, preventing invoicing superseded parent orders.
- **Evidence:**
  - `backend/src/services/order_service.rs:575`
  - `backend/src/services/order_service.rs:587`, `backend/src/services/order_service.rs:589`, `backend/src/services/order_service.rs:598`
  - `backend/src/routes/orders.rs:798`

## Final Assessment

The critical backend contract/logic issues from `audit_report-12` are now addressed. The remaining gap is test realism depth on frontend: browser tests now exist, but critical user journeys are still not fully covered by execution-based end-to-end tests.
