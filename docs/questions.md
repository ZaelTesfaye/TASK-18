# questions.md

## 1. Account Provisioning and Role Assignment

**Question:** Can end users self-register, or are all accounts provisioned by administrators, and how are Shopper/Reviewer/Admin roles assigned or changed?
**Assumption:** Accounts are created locally with self-registration for Shoppers, while Reviewer and Admin roles are granted only by an Admin.
**Solution:** Implement account registration for base users, add admin-only role elevation/demotion with audit entries for every role change.

---

## 2. Authentication Session Model

**Question:** Which auth mechanism is required for API access (server session, JWT, rotating token), and what are token/session expiry and revocation rules?
**Assumption:** Short-lived access token plus revocable refresh token is required for all protected endpoints.
**Solution:** Implement token-based auth with explicit expiry, refresh, logout revocation, and forced invalidation on password reset/role change.

---

## 3. Password Policy and Credential Recovery

**Question:** What password complexity rules, reset flow, and lockout recovery rules are required in an offline environment?
**Assumption:** Strong password policy is mandatory; admin-assisted reset is required offline; lockout requires cooldown plus admin override.
**Solution:** Enforce password policy at registration/change, store salted password hashes, implement admin reset flow and cooldown-based lockout recovery.

---

## 4. Login Rate Limiting Scope

**Question:** For the 10 attempts per 15 minutes rule, should enforcement trigger on username, IP, and username+IP independently, and what response is returned when throttled?
**Assumption:** Limits are enforced across all three keys to prevent bypass.
**Solution:** Add separate counters/windows for username, IP, and username+IP, returning `429` with retry metadata and audit logs.

---

## 5. Sensitive Field Encryption Key Management

**Question:** How are encryption keys generated, stored, rotated, and recovered for encrypted phone/address fields?
**Assumption:** Envelope encryption with server-managed master key and per-record data keys is required.
**Solution:** Encrypt sensitive fields using per-record keys encrypted by a rotatable master key, with key version tracking and re-encryption migration.

---

## 6. Masking Rules for Encrypted Contact Data

**Question:** What exact masking format is required for phone/address display across list and detail views, and which roles can view unmasked values?
**Assumption:** Masked display is default for all roles except explicit privileged access with justification.
**Solution:** Apply deterministic masking (e.g., `(415) ***-**21`) in all responses by default and add privileged unmask endpoint with audit requirement.

---

## 7. Catalog Taxonomy Constraints

**Question:** What are allowed hierarchy depth, cycle-prevention rules, and deletion behavior for topics/tags used in many-to-many taxonomy?
**Assumption:** Topic hierarchy supports fixed max depth and must be acyclic; deletes are blocked while references exist.
**Solution:** Enforce depth and acyclic constraints at write time, block destructive changes when linked records exist, and provide reassignment workflow.

---

## 8. Custom Field Versioning and Visibility

**Question:** When a custom field version changes, which catalog/version is visible to shoppers during unresolved migration conflicts?
**Assumption:** New version remains unpublished; latest published stable version remains visible until conflicts are resolved.
**Solution:** Add draft/published states for field schema versions and block publishing while unresolved conflicts exist.

---

## 9. Custom Field Migration Conflict Resolution

**Question:** What conflict states and admin actions are required when value conversion fails during type/enum changes?
**Assumption:** Each conflicting row must be traceable and resolvable individually before publication.
**Solution:** Store per-record conflict entries, provide admin resolution actions (map value, set null/default, manual override), and require completion gate.

---

## 10. Inventory Reservation Semantics

**Question:** At which checkout step is stock reserved, what happens on timeout, and how are concurrent reservations prevented from overselling?
**Assumption:** Reservation occurs at order creation and expires with unpaid cancellation.
**Solution:** Use transactional stock reservation at order placement, release on timeout/cancel, and lock inventory rows to prevent oversell.

---

## 11. Order Timer and Scheduler Recovery

**Question:** How should auto-cancel jobs behave if the app/server is down at the deadline and later restarts offline?
**Assumption:** Expired orders must be reconciled immediately on startup.
**Solution:** Implement startup reconciliation that cancels all unpaid expired orders and records system-triggered audit events.

---

## 12. Order State Machine Contract

**Question:** What are the exact allowed states and legal transitions for reserve, pay, ship, deliver, refund, return, exchange, split, and merge actions?
**Assumption:** Transitions are strict and invalid transitions must be rejected atomically.
**Solution:** Encode a centralized transition matrix, validate every change in a transaction, and return consistent domain errors on invalid transitions.

---

## 13. Split/Merge and Consolidated Invoice Rules

**Question:** What invariants define valid partial fulfillment split/merge operations and how is consolidated invoicing calculated after changes?
**Assumption:** Child orders preserve item-level traceability and totals must reconcile to parent-level accounting.
**Solution:** Implement lineage links between original and derived orders, enforce quantity/amount conservation, and generate invoice snapshots per mutation.

---

## 14. Return/Refund/Exchange Eligibility Window

**Question:** Is the 30-day limit based on exact timestamp (to the second) from delivery confirmation, and which timezone is authoritative?
**Assumption:** UTC timestamp comparison is authoritative and immutable after delivery confirmation.
**Solution:** Persist delivery timestamps in UTC, enforce precise 30-day eligibility checks, and log explicit reason-code validation.

---

## 15. Offline Payment Callback Simulator Behavior

**Question:** Which simulated payment outcomes and retry/idempotency rules are required for local callback testing?
**Assumption:** Simulator must support success, pending, failed, and reversed outcomes with idempotent callback replay.
**Solution:** Build local callback simulator endpoints with signed test payloads, replay protection, and deterministic state transitions.

---

## 16. Rating Eligibility and Possession Verification

**Question:** What events qualify as verified possession for allowing ratings, and can users rate before delivery if possession is otherwise confirmed?
**Assumption:** Eligibility requires delivered order or admin-verified possession record tied to user and title.
**Solution:** Enforce eligibility checks against delivery/proof records before rating creation/update.

---

## 17. Multi-Dimension Rating Aggregation Formula

**Question:** How is aggregate score computed from dimensions (equal weight or configured weights), and what rounding precision is required?
**Assumption:** Equal-weight average with fixed decimal precision is required unless overridden by admin configuration.
**Solution:** Implement deterministic aggregation with configured precision and recalculate aggregates transactionally on rating changes.

---

## 18. Leaderboard Inclusion and Tie Handling

**Question:** What minimum activity threshold, moderation filters, and recency timestamp should determine leaderboard inclusion and tie-breaks?
**Assumption:** Only active, approved ratings count; ties break by higher rating count then most recent approved rating timestamp.
**Solution:** Generate leaderboard materialization using approved records only, apply threshold filters, and enforce deterministic tie ordering.

---

## 19. Review Submission Rounds and Deadlines

**Question:** What timezone governs round deadlines, are late submissions rejected or flagged, and can drafts be saved after cutoff?
**Assumption:** Deadlines are stored in UTC, late final submissions are rejected, drafts may persist but cannot be finalized.
**Solution:** Implement server-side UTC deadline checks, finalization gate, and immutable version trail for each round replacement.

---

## 20. Template Versioning for Structured Reviews

**Question:** If a form template changes mid-round, do existing submissions keep old schema or require migration to the latest template version?
**Assumption:** Submissions remain bound to the template version active at submission time.
**Solution:** Version templates explicitly, bind each submission to a template version, and render historical submissions with their original schema.

---

## 21. Attachment Constraints and Watermarked Download Controls

**Question:** What file types, size limits, virus-check policy, and watermark format/location are mandatory for evidence attachments?
**Assumption:** Strict allowlist and size caps are required; watermark must include requester username and timestamp on every approved download.
**Solution:** Enforce upload constraints, scan/validate metadata locally, and apply deterministic watermarking at download time with role authorization checks.

---

## 22. Audit Trail Immutability and Coverage

**Question:** Which exact actions are considered privileged, and should automated system jobs (auto-cancel, retention purge, backup) be audited as actors too?
**Assumption:** All admin/reviewer actions and all automated mutations must produce immutable audit records.
**Solution:** Implement append-only audit log with actor type (`user` or `system`), IP/context, object diffs, and tamper-evident sequencing.

---

## 23. Operational Reporting Definitions

**Question:** Which reports are mandatory (orders, payment status, moderation actions, risk events), and what granularity and export formats are required?
**Assumption:** Date-range reports must support summary plus detailed rows and export to CSV.
**Solution:** Provide report endpoints with timezone-aware date filtering, aggregate KPIs, detailed breakdowns, and CSV export.

---

## 24. Data Retention and Purge Behavior

**Question:** After 7-year order and 2-year auth-log retention windows, should data be hard-deleted, anonymized, or archived, and are legal holds supported?
**Assumption:** Auth logs are purged, order records are archived/anonymized unless under legal hold.
**Solution:** Add retention jobs with policy-driven purge/archive actions, legal-hold bypass, and audit logs for every retention execution.

---

## 25. Backup/Restore Verification

**Question:** What encryption standard, key source, integrity checks, and restore acceptance criteria define a valid local backup strategy?
**Assumption:** Nightly encrypted backups require checksum verification and periodic restore test validation.
**Solution:** Generate encrypted backup artifacts on schedule, retain 14 copies, verify integrity at creation/restore, and log restore drill outcomes.

---

## 26. Suspicious Activity Risk Rules

**Question:** What concrete thresholds and enforcement actions define bulk-order spikes and discount-misuse throttling, and who can override blocks?
**Assumption:** Rules are configurable with default thresholds and temporary throttles; admin override is audited.
**Solution:** Implement configurable risk rules engine, automatic throttling/step-up checks, and audited admin override controls.
