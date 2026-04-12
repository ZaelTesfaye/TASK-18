# Audit Fix Check Report (10 -> fixcheck-1)

## Scope
- Static re-check of prior findings from `.tmp/audit_report-10.md`
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
- **Previous status:** Fail
- **Current status:** **Fixed**
- **Evidence:**
  - `repo/backend/src/routes/reviews.rs:169` (`async fn get_round`)
  - `repo/backend/src/routes/reviews.rs:174` now includes `require_any_role(&user, &["Reviewer", "Admin"])?;`
  - Added guard tests:
    - `repo/backend/tests/api/test_review_api.rs:525` (`test_get_round_detail_forbidden_for_shopper`)
    - `repo/backend/tests/api/test_review_api.rs:590` (`test_all_review_round_endpoints_reject_shopper`)
- **Conclusion:** Role-boundary gap is closed for round detail endpoint.

### F-02 (High) Frontend/backend API contract drift (score + order item title)
- **Previous status:** Fail
- **Current status:** **Fixed**
- **Evidence:**
  - Product score alignment:
    - Backend emits `average_score`: `repo/backend/src/models/product.rs:52`
    - Frontend now accepts backend field via alias and maps to UI field:
      - `repo/frontend/src/types.rs:119-120` (`alias = "average_score"` -> `aggregate_score`)
      - `repo/frontend/src/components/product_card.rs:56`
  - Order item title alignment:
    - Backend response now includes `product_title`:
      - `repo/backend/src/models/order.rs:85`
      - `repo/backend/src/routes/orders.rs:71` (joins `p.title AS product_title`)
      - `repo/backend/src/routes/orders.rs:89` (maps `product_title`)
    - Frontend consumes `product_title`:
      - `repo/frontend/src/types.rs:272`
      - `repo/frontend/src/pages/order_detail.rs:288`
- **Conclusion:** Reported contract mismatches are resolved.

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
