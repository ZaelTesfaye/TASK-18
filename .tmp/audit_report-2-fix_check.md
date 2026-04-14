# Fix Check Report

**Source of Truth**: `.tmp/audit_report-2.md` (previously verified findings)

## Source Note

- Source file for this fix-check: `.tmp/audit_report-2.md`

## Overall Result

- **Fixed:** 4
- **Partially Fixed:** 1
- **Not Fixed:** 0

## Issue-by-Issue Verification

### [H-01] Admin Audit Date Filter Contract Mismatch

- **Original Finding**: [.tmp/audit_report-2.md](audit_report-2.md) - Date input format mismatches between frontend/backend audit filters
- **Fix Verification**:
  - **Code Locations**:
    - [repo/backend/src/models/audit.rs](../repo/backend/src/models/audit.rs#L22,L50,L66) (lines 22, 50, 66)
    - [repo/backend/src/services/audit_service.rs](../repo/backend/src/services/audit_service.rs#L83,L86,L90) (lines 83, 86, 90)
    - [repo/frontend/src/api/admin.rs](../repo/frontend/src/api/admin.rs#L52,L59) (lines 52, 59)
  - **What Changed**: Backend now accepts flexible date inputs (RFC3339 or YYYY-MM-DD) and parses both safely; frontend normalizes bare dates to RFC3339 before sending
- **Decision**: Fixed ✓ - Date filter contract is now compatible

### [H-02] Frontend �E2E/Component� Tests Mostly Non-Executable Assertions

- **Original Finding**: [.tmp/audit_report-2.md](audit_report-2.md) - Frontend tests labeled as e2e but lack real browser execution and critical user journey coverage
- **Fix Verification**:
  - **What Changed**:
    - **Improvement**: Browser-test suite added under [repo/frontend/tests/wasm/](../repo/frontend/tests/wasm/test_browser.rs#L23,L32) (lines 23, 32) using `wasm_bindgen_test`
    - **Remaining Gap**: Most [repo/frontend/tests/e2e/](../repo/frontend/tests/e2e/test_components.rs#L3) (line 3) and [repo/frontend/tests/e2e/test_routes.rs](../repo/frontend/tests/e2e/test_routes.rs#L1) (line 1) tests remain type/value assertions rather than full app-flow interaction tests
  - **User Journeys Still Missing**: login → cart → checkout → reviewer submission → admin reporting (all e2e)
- **Decision**: Partially Fixed ⚠️ - Browser tests now exist but critical user journeys still need real e2e coverage

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

The critical backend contract/logic issues from [.tmp/audit_report-2.md](audit_report-2.md) are now addressed. The remaining gap is test realism depth on frontend: browser tests now exist, but critical user journeys are still not fully covered by execution-based end-to-end tests.

---

## Source of Truth and Verification Traceability

| Finding | Source Report Line                     | Current Code Location                                 | Status             |
| ------- | -------------------------------------- | ----------------------------------------------------- | ------------------ |
| H-01    | [audit_report-2.md](audit_report-2.md) | backend/src/models/audit.rs:22,50,66                  | Fixed ✓            |
| H-02    | [audit_report-2.md](audit_report-2.md) | frontend/tests/wasm/test_browser.rs:23,32             | Partially Fixed ⚠️ |
| M-01    | [audit_report-2.md](audit_report-2.md) | backend/src/routes/reports.rs:23,25,98,99             | Fixed ✓            |
| M-02    | [audit_report-2.md](audit_report-2.md) | backend/src/models/user.rs:25,26                      | Fixed ✓            |
| M-03    | [audit_report-2.md](audit_report-2.md) | backend/src/services/order_service.rs:575,587,589,598 | Fixed ✓            |
