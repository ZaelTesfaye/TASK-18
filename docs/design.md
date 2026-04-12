# System Design

Architecture and design reference for the SilverScreen Commerce and Review platform.

## 1. Overview

SilverScreen is an offline-first commerce and review system composed of:

- Rust backend using Actix-web
- Rust/WASM frontend using Yew
- PostgreSQL for durable transactional storage
- Docker Compose for single-command local orchestration

The platform supports catalog browsing, order management, structured reviews, rating leaderboards, risk controls, immutable audit logging, and encrypted backups.

## 2. Architecture

```
Browser (Yew SPA)
	|
	v
Frontend container (Nginx + WASM assets :8081)
	|
	v
Actix-web API (:8080)
	|
	v
PostgreSQL (:5432)

Side processes in backend:
- Expired-order reconciliation loop
- Nightly encrypted backup loop
```

### Runtime boundaries

- Frontend only calls internal backend APIs.
- Backend performs no third-party payment calls.
- All persistent state is local to PostgreSQL and backup volume.

## 3. Backend Design (repo/backend)

### Modules

- `config`: centralized typed configuration from environment variables
- `db`: database pool initialization and migration bootstrap
- `routes`: HTTP endpoint registration grouped by domain
- `services`: business logic (orders, auth, backup, review, retention, audit)
- `middleware`: auth, RBAC, rate limiting
- `models`: request/response/domain models
- `logging`: structured logging and redaction
- `errors`: typed app errors mapped to HTTP responses

### Route domains

- `auth`, `users`, `products`, `cart`, `orders`, `payment`
- `ratings`, `reviews`, `leaderboards`
- `taxonomy`, `custom_fields`
- `admin`, `audit`, `reports`, `backup`

### Request pipeline

1. Route and deserialize request.
2. Apply auth/RBAC/rate-limiting middleware as needed.
3. Execute domain service logic in transaction-safe sequence.
4. Persist changes and emit audit events for privileged/system actions.
5. Return JSON response.

## 4. Core Business Flows

### 4.1 Authentication and session lifecycle

- Register creates shopper accounts.
- Login validates password and lockout/rate-limit state.
- Access and refresh tokens are issued separately.
- Refresh endpoint accepts only refresh tokens.
- Logout revokes token identifiers in server-side denylist.

### 4.2 Cart and order state machine

- Cart operations are user-scoped.
- Creating an order reserves stock transactionally.
- Unpaid orders expire after 30 minutes.
- Background reconciliation runs periodically and on startup.
- Post-delivery windows govern return/refund/exchange eligibility.
- Split/merge operations preserve item and accounting consistency.

### 4.3 Ratings and review rounds

- Ratings enforce eligibility (delivery or verified possession).
- Aggregation computes deterministic product scores.
- Leaderboards derive rank slices by period/category with stable tie-breaks.
- Review rounds support templated multi-round submissions and version history.
- Attachment approval and download are role-gated and audited.

### 4.4 Taxonomy and custom field migration

- Topics/tags provide many-to-many organization.
- Custom field definitions support versioned schema evolution.
- Schema changes can produce conflict rows.
- Publishing new schema versions is blocked until conflicts are resolved.

### 4.5 Administration and compliance

- Admin APIs manage role assignment, unlocks, and risk overrides.
- Retention jobs implement policy windows and legal hold exceptions.
- Audit logs are append-only and include actor, object, and change summary.
- Reports provide date-range operational visibility.

## 5. Security Model

### Authentication and authorization

- JWT bearer tokens for API access
- Role-based route enforcement
- Object-level checks where required by action semantics

### Data protection

- Passwords hashed with salted secure hashing
- Sensitive fields (phone/address) encrypted at rest
- Default masked display for sensitive contact data
- Explicit unmask actions with auditable justification

### Abuse prevention

- Login rate limiting by username, IP, and combined key
- Account lockout and admin unlock flow
- Risk-event generation for suspicious order/discount behavior

## 6. Reliability and Operations

### Background jobs

- Order expiry reconciliation every minute
- Nightly encrypted backup task with retention cap

### Backup and restore

- Backups are encrypted and stored locally.
- Verify and restore endpoints support integrity and recovery drills.
- Backup actions produce system audit events.

### Data retention defaults

- Orders: 7 years
- Auth logs: 2 years

Policy actions are auditable and legal-hold aware.

## 7. Deployment Model

### Local/dev deployment

- `docker-compose.yml` starts frontend, backend, and database.
- `.env` contains required secrets/config.
- Backend validates secret quality at startup.

### Environment assumptions

- Single-node deployment
- No internet dependency for core workflows
- TLS support controlled by runtime config flag

## 8. Design Decisions

### Why Actix-web + Yew

- Shared Rust ecosystem across backend and frontend
- Strong typing and compile-time guarantees
- Good fit for deterministic offline domain logic

### Why strict service boundaries

- Keeps route handlers thin and testable
- Centralizes domain invariants in services
- Makes audit and policy controls easier to enforce consistently

### Why explicit state-machine behavior

- Prevents illegal order and payment transitions
- Improves traceability for support and compliance
- Simplifies automated reconciliation after downtime
