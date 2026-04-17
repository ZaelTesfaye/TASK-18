#!/bin/bash
# Seed demo users for all three roles (Admin, Reviewer, Shopper).
#
# This script is idempotent — duplicate registrations return 409 and are
# ignored. It must be run AFTER docker-compose up has the backend healthy.
#
# Usage (from repo root):
#   ./seed_demo_users.sh
#
# Or automatically via docker-compose (see docker-compose.yml seed service).

set -e

API="${API_BASE_URL:-http://localhost:8080}"

echo "Seeding demo users against $API ..."

register() {
  local username="$1" email="$2" password="$3" role="$4"
  local status
  status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API/api/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$username\",\"email\":\"$email\",\"password\":\"$password\"}")
  if [ "$status" = "201" ]; then
    echo "  Registered $username ($email)"
  elif [ "$status" = "409" ]; then
    echo "  $username already exists (skipped)"
  else
    echo "  WARNING: $username registration returned HTTP $status"
  fi
}

# Register all three demo users (all start as Shopper)
register "admin"    "admin@example.com"    "Admin1234!"    "Admin"
register "reviewer" "reviewer@example.com" "Review1234!"   "Reviewer"
register "shopper"  "shopper@example.com"  "Shop1234!"     "Shopper"

# Promote admin and reviewer via direct DB update (Argon2 hashing prevents
# role assignment via the API without an existing admin).
if command -v docker >/dev/null 2>&1; then
  DB_CONTAINER=$(docker ps -qf "ancestor=postgres:15-alpine" 2>/dev/null || true)
  if [ -n "$DB_CONTAINER" ]; then
    echo "Promoting roles via database..."
    docker exec -i "$DB_CONTAINER" psql -U silverscreen -d silverscreen -c \
      "UPDATE users SET role = 'Admin' WHERE username = 'admin' AND role != 'Admin';" 2>/dev/null && \
      echo "  admin -> Admin" || echo "  admin role update skipped"
    docker exec -i "$DB_CONTAINER" psql -U silverscreen -d silverscreen -c \
      "UPDATE users SET role = 'Reviewer' WHERE username = 'reviewer' AND role != 'Reviewer';" 2>/dev/null && \
      echo "  reviewer -> Reviewer" || echo "  reviewer role update skipped"
  else
    echo "  WARNING: PostgreSQL container not found. Role promotion skipped."
    echo "  Run manually: docker exec -i <db_container> psql -U silverscreen -d silverscreen"
    echo "    -c \"UPDATE users SET role='Admin' WHERE username='admin';\""
  fi
fi

echo "Done. Demo credentials:"
echo "  Admin:    admin@example.com     / Admin1234!"
echo "  Reviewer: reviewer@example.com  / Review1234!"
echo "  Shopper:  shopper@example.com   / Shop1234!"
