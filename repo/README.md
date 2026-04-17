Project Type: fullstack

# SilverScreen Commerce & Review Platform

A full-stack offline-first commerce and review platform built with **Actix-web** (Rust backend), **Yew** (Rust/WASM frontend), and **PostgreSQL**.

## Quick Start

**1. Create the environment file** (one-time setup):

```bash
cp .env.example .env
```

The `.env.example` template includes `SILVERSCREEN_DEV_MODE=true`. After copying,
edit `.env` and replace every `CHANGE_ME` value with real secrets. For local
development, `SILVERSCREEN_DEV_MODE=true` tells the backend to **warn** on weak
JWT secrets instead of panicking. **Encryption keys always require real values** —
even in dev mode — because weak keys produce trivially decryptable data.

> **Note:** `.env` is gitignored and must never be committed. Only `.env.example`
> (with clearly fake `CHANGE_ME` placeholders) is tracked in version control.

**For production**, generate real secrets and disable dev mode:
```bash
# Generate secrets — paste the output into .env:
openssl rand -hex 64   # → JWT_SECRET
openssl rand -hex 16   # → ENCRYPTION_KEY (produces exactly 32 hex chars)
openssl rand -hex 32   # → BACKUP_ENCRYPTION_KEY

# Then in .env:
SILVERSCREEN_DEV_MODE=false   # (or remove the line entirely)
```

**2. Start all services:**

```bash
docker-compose up --build
```

| Service    | URL                     | Description                        |
|------------|-------------------------|------------------------------------|
| Frontend   | http://localhost:8081   | Yew-based SPA                      |
| Backend    | http://localhost:8080   | Actix-web REST API                 |
| PostgreSQL | localhost:5432          | Database (internal to compose)     |

> **Note:** The `.env` file is required. All secrets (database password, JWT secret, encryption keys) are read from `.env` — the `docker-compose.yml` file contains zero plaintext secrets. See `.env.example` for the full list of required variables.

All dependencies are managed inside Docker. Run `docker-compose up --build` — no local toolchain installation is required.

## Verifying the Setup

After `docker-compose up --build` completes and all three services are healthy:

**Step 1 — Seed demo users** (one-time, idempotent):

```bash
./seed_demo_users.sh
```

This registers all three demo accounts and promotes the admin and reviewer roles automatically. No manual SQL is required.

**Step 2 — Verify the API:**

```bash
# Confirm the API is running
curl -s http://localhost:8080/health
# Expected: {"status":"ok"}

# Log in as each role to confirm auth works
curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"Admin1234!"}'
# Expected: {"access_token":"...","refresh_token":"...","token_type":"Bearer"}
```

**Step 3 — Verify each role in the UI** (http://localhost:8081):

| Role     | Steps |
|----------|-------|
| Shopper  | Log in as `shopper` / `Shop1234!`. Browse the catalog, add an item to the cart, proceed to checkout. Confirm the order appears in "My Orders". |
| Reviewer | Log in as `reviewer` / `Review1234!`. Navigate to `/reviewer/rounds`. Confirm the review rounds list loads (may be empty if none are seeded). |
| Admin    | Log in as `admin` / `Admin1234!`. Navigate to `/admin`. Confirm the admin dashboard loads with links to Users, Taxonomy, Audit Log, Reports, and Backup. |

## Demo Credentials

All demo accounts are seeded by `./seed_demo_users.sh` (run once after startup):

| Role     | Username   | Email                    | Password      |
|----------|------------|--------------------------|---------------|
| Admin    | admin      | admin@example.com        | Admin1234!    |
| Reviewer | reviewer   | reviewer@example.com     | Review1234!   |
| Shopper  | shopper    | shopper@example.com      | Shop1234!     |

The seed script registers the users via the API and promotes roles via the database container. It is idempotent — running it multiple times is safe.

## Architecture

```
repo/
├── backend/           # Actix-web REST API (Rust)
│   ├── config/        # Centralized config module
│   ├── logging/       # Structured logging with redaction
│   ├── src/           # Application source code
│   │   ├── models/    # Domain models
│   │   ├── routes/    # API route handlers
│   │   ├── services/  # Business logic services
│   │   └── middleware/ # Auth, RBAC, rate limiting, risk
│   └── tests/
│       ├── unit/      # Unit tests
│       └── api/       # API integration tests
├── frontend/          # Yew SPA (Rust/WASM)
│   ├── src/
│   │   ├── pages/     # Page components
│   │   ├── components/# Reusable UI components
│   │   └── api/       # Backend API client
│   └── tests/
│       ├── unit/      # Unit tests
│       └── e2e/       # End-to-end tests
├── docker-compose.yml # Single orchestration file
├── run_tests.sh       # Global test runner
└── README.md
```

## User Roles

| Role     | Capabilities |
|----------|-------------|
| Shopper  | Browse catalog, manage cart, place orders, rate delivered items |
| Reviewer | Submit structured evaluations with version history and attachments |
| Admin    | Manage taxonomy, moderate content, assign roles, run audits/reports |

## Running Tests

```bash
./run_tests.sh
```

This script runs all backend unit + API tests and frontend tests inside Docker containers, outputting a clear summary of totals, passes, and failures. No local toolchain (rustup, cargo, psql) is required.

All development and testing is container-contained. Do not install Rust, wasm toolchains, or database clients on the host.

## Environment

All environment variables are defined in `docker-compose.yml`. The backend's config module is the single source of truth — application code never reads environment variables directly.

### Key Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ENABLE_TLS` | `false` | Toggle TLS support |
| `JWT_ACCESS_EXPIRY_MINUTES` | `30` | Access token lifetime |
| `JWT_REFRESH_EXPIRY_DAYS` | `7` | Refresh token lifetime |
| `RATE_LIMIT_LOGIN_MAX` | `10` | Max login attempts per window |
| `RATE_LIMIT_LOGIN_WINDOW_SECONDS` | `900` | Rate limit window (15 min) |
| `BACKUP_RETENTION_COUNT` | `14` | Number of backup copies kept |
| `RETENTION_ORDERS_YEARS` | `7` | Order data retention period |
| `RETENTION_AUTH_LOGS_YEARS` | `2` | Auth log retention period |

## Security

- **Authentication**: Salted password hashing (Argon2), JWT access + refresh tokens
- **Authorization**: Role-based access control (RBAC) enforced at route and object level
- **Encryption**: AES-256-GCM per-field encryption for phone/address with key versioning
- **Rate Limiting**: Per-username, per-IP, and combined rate limiting
- **Risk Detection**: Bulk ordering and discount abuse pattern detection with throttling
- **Audit Trail**: Immutable log of all privileged and automated actions
- **Data Masking**: Sensitive fields displayed masked by default; unmask requires justification

## Offline-First Design

- No external payment processor dependencies
- Local payment event model with callback simulator for testing
- All data persisted to local PostgreSQL
- Backup/restore to local encrypted files

## API Documentation

All API endpoints follow REST conventions with structured JSON responses, pagination, and proper HTTP status codes.
