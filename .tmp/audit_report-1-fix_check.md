# Audit Fix Check Report

**Source of Truth**: `.tmp/audit_report-1.md` (previously verified findings)

## Scope

- Static re-check of prior findings from `.tmp/audit_report-1.md`
- Code reviewed under `repo/` only
- Not executed: runtime, tests, Docker

## Overall Result

- **Previous High findings:** 2
- **Status now:** **Both High findings fixed**
- **Previous Medium findings:** 2
- **Status now:** 1 fixed, 1 substantially improved/fixed for reported gap

---

## Finding-by-Finding Re-check

### F-01 (High) Missing role enforcement on `GET /api/reviews/rounds/{id}`

- **Original Finding**: [.tmp/audit_report-1.md](audit_report-1.md) - `/api/reviews/rounds/{id}` allowed unauthorized Shopper access
- **Fix Verification**:
  - **Code Location**: [repo/backend/src/routes/reviews.rs](../repo/backend/src/routes/reviews.rs#L169-L174) (lines 169-174)
  - **What Changed**: Added `require_any_role(&user, &["Reviewer", "Admin"])?;` guard to round detail endpoint
  - **Test Verification**: [repo/backend/tests/api/test_review_api.rs](../repo/backend/tests/api/test_review_api.rs#L525,L590) (lines 525, 590) - tests confirm Shopper access now returns 403
- **Decision**: Fixed ✓ - Role-boundary gap is closed for round detail endpoint

### F-02 (High) Frontend/backend API contract drift (score + order item title)

- **Original Finding**: [.tmp/audit_report-1.md](audit_report-1.md) - Frontend/backend field name mismatches on product score and order item title
- **Fix Verification**:
  - **Product Score Alignment**:
    - **Before**: Backend emitted `average_score` but frontend expected `aggregate_score`
    - **After**: [repo/frontend/src/types.rs](../repo/frontend/src/types.rs#L119-L120) (lines 119-120) now includes `#[serde(alias = "average_score")]` mapping
    - **Decision**: Fixed ✓
  - **Order Item Title Alignment**:
    - **Before**: Backend response lacked `product_title` field for order items
    - **After**: [repo/backend/src/routes/orders.rs](../repo/backend/src/routes/orders.rs#L71,L89) (lines 71, 89) now joins and maps `p.title AS product_title`; [repo/frontend/src/types.rs](../repo/frontend/src/types.rs#L272) (line 272) consumes this field
    - **Decision**: Fixed ✓
- **Final Decision**: Contract mismatches resolved ✓

### F-03 (Medium) Custom-field filter JSON text mismatch risk

- **Previous status:** Suspected Risk / Partial Fail
- **Current status:** **Fixed**
- **Evidence:**
  - Query condition changed to JSONB-safe comparison:
    - `repo/backend/src/routes/products.rs:76-82`
    - Uses `cfv.value = to_jsonb(${next_param_idx}::text)` instead of `cfv.value::text = ...`
- **Conclusion:** Prior JSON text-cast mismatch risk has been addressed.

### F-04 (Medium) Test suite missed authz + contract drift detections

- **Previous status:** Partial Fail
- **Current status:** **Fixed for the reported gaps**
- **Evidence:**
  - Added shopper denial tests for round detail endpoint:
    - `repo/backend/tests/api/test_review_api.rs:525-541`
    - `repo/backend/tests/api/test_review_api.rs:590-618`
  - Contract fixtures now align with backend naming (`average_score`):
    - `repo/frontend/tests/contract/test_typed_deserialization.rs:29-48`
    - `repo/frontend/tests/e2e/test_api_contracts.rs:258-267`
- **Note:** Contract fixtures remain hand-maintained (not generated), which is a maintainability consideration, but the specific defect reported in F-04 is no longer present.

---

## Final Judgment

- **Issue set from prior report:** materially resolved.
- **Acceptance for this fix-check:** **Pass (for prior issue set re-check)**

## Boundary Reminder

- This conclusion is static-only and does not claim runtime behavior without execution.
