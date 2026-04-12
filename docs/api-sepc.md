# API Specification

API contract for the SilverScreen Commerce and Review platform.

## 1. Base URL and Conventions

- Base URL (backend): `http://localhost:8080`
- API prefix: `/api`
- Health endpoint: `GET /health` (outside `/api`)
- Content type: `application/json` unless uploading attachments (`multipart/form-data`)
- Auth scheme: `Authorization: Bearer <access_token>`

### Roles

- `Shopper`: browse catalog, cart, order, rate eligible products
- `Reviewer`: review rounds, submit template answers, upload attachments
- `Admin`: moderation, role changes, retention/legal hold, backups, reports

## 2. Response Patterns

Responses are JSON. Success may return either resource objects or list envelopes (some list endpoints use pagination metadata).

Example success:

```json
{
  "id": "9d8aef41-8f8e-4bc0-bf86-6ea1ee1db986",
  "status": "Created"
}
```

Example paginated success:

```json
{
  "items": [],
  "page": 1,
  "per_page": 20,
  "total": 0
}
```

Example error:

```json
{
  "error": "ValidationError",
  "message": "Username is required"
}
```

## 3. Authentication and Session Flow

### Endpoints

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `POST /api/auth/reset-password`

### Login request

```json
{
  "username": "shopper1",
  "password": "StrongPassword123!"
}
```

### Login response

```json
{
  "access_token": "<jwt>",
  "refresh_token": "<jwt>",
  "token_type": "Bearer"
}
```

### Notes

- Access token and refresh token are distinct token types.
- Refresh endpoint accepts only refresh tokens.
- Logout revokes both submitted refresh token and current access token.
- Password reset token is admin-issued via admin API.

## 4. User Endpoints

- `GET /api/users/me`
- `PUT /api/users/me`
- `POST /api/users/me/unmask`
- `GET /api/users/{id}`

### Unmask flow

- Sensitive fields (phone, address) are masked by default.
- Unmask requires authenticated user and justification workflow at API level.
- Unmask requests are auditable events.

## 5. Catalog, Taxonomy, and Custom Fields

### Product endpoints

- `GET /api/products`
- `GET /api/products/{id}`
- `POST /api/products`
- `PUT /api/products/{id}`
- `DELETE /api/products/{id}`

### Taxonomy endpoints

- `GET /api/taxonomy/topics`
- `POST /api/taxonomy/topics`
- `PUT /api/taxonomy/topics/{id}`
- `DELETE /api/taxonomy/topics/{id}`
- `GET /api/taxonomy/tags`
- `POST /api/taxonomy/tags`
- `DELETE /api/taxonomy/tags/{id}`

### Custom field endpoints

- `GET /api/custom-fields`
- `POST /api/custom-fields`
- `PUT /api/custom-fields/{id}`
- `POST /api/custom-fields/{id}/publish`
- `GET /api/custom-fields/{id}/conflicts`
- `PUT /api/custom-fields/{id}/conflicts/{product_id}`

### Custom field lifecycle

- New/edited definitions stay draft until publish.
- Type/enum changes may create per-product migration conflicts.
- Publish is blocked until conflicts are resolved.

## 6. Cart and Checkout

### Cart endpoints

- `GET /api/cart`
- `POST /api/cart/items`
- `PUT /api/cart/items/{id}`
- `DELETE /api/cart/items/{id}`
- `DELETE /api/cart`

### Order endpoints

- `GET /api/orders`
- `GET /api/orders/{id}`
- `POST /api/orders`
- `PUT /api/orders/{id}/status`
- `POST /api/orders/{id}/return`
- `POST /api/orders/{id}/exchange`
- `POST /api/orders/{id}/refund`
- `POST /api/orders/{id}/split`
- `POST /api/orders/{id}/merge`
- `GET /api/orders/{id}/invoice`

### Order behavior rules

- Order creation reserves inventory transactionally.
- Unpaid orders auto-cancel after 30 minutes.
- Startup reconciliation cancels already-expired unpaid orders after downtime.
- Return/refund/exchange require reason codes and eligibility checks.

## 7. Payment Simulation

- `POST /api/payment/simulate`

Used for offline testing of payment callbacks and status transitions. No third-party payment provider integration is required.

## 8. Ratings, Reviews, and Leaderboards

### Ratings endpoints

- `POST /api/ratings`
- `GET /api/ratings/product/{id}`
- `GET /api/ratings/{id}`
- `PUT /api/ratings/{id}`
- `DELETE /api/ratings/{id}`

### Review workflow endpoints

- `GET /api/reviews/rounds`
- `GET /api/reviews/rounds/{id}`
- `POST /api/reviews/rounds/{id}/submit`
- `GET /api/reviews/submissions/{id}`
- `GET /api/reviews/submissions/{id}/history`
- `POST /api/reviews/submissions/{id}/attachments`
- `GET /api/reviews/attachments/{id}/download`
- `POST /api/reviews/attachments/{id}/approve`

### Leaderboard endpoint

- `GET /api/leaderboards`

### Rules

- Ratings are allowed only after delivery or verified possession.
- Dimension scores aggregate deterministically.
- Leaderboards support period/genre slicing with tie breaks (count, then recency).
- Attachment download should apply watermarking and authorization.

## 9. Admin, Audit, Reporting, and Retention

### Admin endpoints

- `GET /api/admin/users`
- `PUT /api/admin/users/{id}/role`
- `POST /api/admin/users/{id}/reset-password`
- `POST /api/admin/users/{id}/unlock`
- `GET /api/admin/risk-events`
- `PUT /api/admin/risk-events/{id}`
- `POST /api/admin/moderation/ratings/{id}`
- `POST /api/admin/retention/run`
- `POST /api/admin/retention/legal-hold/{order_id}`

### Audit and reports

- `GET /api/audit`
- `GET /api/reports`

### Backup endpoints

- `POST /api/backups`
- `GET /api/backups`
- `POST /api/backups/{id}/verify`
- `POST /api/backups/{id}/restore`

### Operational rules

- Every privileged action writes immutable audit records.
- Nightly encrypted backups are retained with a fixed count window.
- Retention policy defaults: orders 7 years, auth logs 2 years.

## 10. HTTP Status Usage

- `200 OK`: successful reads/updates/actions
- `201 Created`: successful create
- `400 Bad Request`: invalid request shape
- `401 Unauthorized`: missing/invalid auth
- `403 Forbidden`: role or policy violation
- `404 Not Found`: entity not found
- `409 Conflict`: state transition or business conflict
- `422 Unprocessable Entity`: semantic validation failure
- `429 Too Many Requests`: login or API throttling
- `500 Internal Server Error`: unexpected server issue

## 11. Security Requirements

- Passwords: salted password hashing
- Tokens: short-lived access + revocable refresh
- PII fields: encrypted at rest and masked in responses
- Login protection: per-username/IP and combined rate limits
- Risk controls: bulk ordering and discount abuse rules with audited admin override
- Audit: append-only records for user and system actors
