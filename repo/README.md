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

### Using Docker (container)

```bash
./run_tests.sh
```

This script runs all backend unit + API tests and frontend tests, outputting a clear summary of totals, passes, and failures.

### Running Natively (without Docker)

If you prefer running tests directly on the host, ensure the following prerequisites:

1. **Rust toolchain** with `wasm32-unknown-unknown` target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. **PostgreSQL** running locally with the schema applied:
   ```bash
   psql -U postgres -d silverscreen -f backend/migrations/001_initial.sql
   ```

3. **Environment variables** — export these or create a `.env` in the backend directory:
   ```bash
   export DATABASE_URL="postgresql://postgres:yourpassword@localhost:5432/silverscreen"
   export JWT_SECRET="your_jwt_secret_at_least_32_chars_long"
   export ENCRYPTION_KEY="your_encryption_key_at_least_32_chars"
   ```

4. **Backend tests** (unit + integration):
   ```bash
   cd backend
   cargo test --lib -- --test-threads=1       # Unit tests
   cargo test --test '*' -- --test-threads=1   # API integration tests
   ```

5. **Frontend tests** (unit + contract):
   ```bash
   cd frontend
   cargo test --all-targets     # Unit tests (native target)
   cargo test --test '*'        # Contract / E2E tests
   ```

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
