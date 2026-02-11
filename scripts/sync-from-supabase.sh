#!/bin/bash
set -e

# Load .env for Supabase DATABASE_URL
if [ -f .env ]; then
  export $(grep -v '^#' .env | grep DATABASE_URL | xargs)
fi

if [ -z "$DATABASE_URL" ]; then
  echo "ERROR: DATABASE_URL not set. Check your .env file."
  exit 1
fi

LOCAL_DB="postgresql://postgres:postgres@localhost:5432/nad8004"

echo "==> Dumping data from Supabase..."
pg_dump "$DATABASE_URL" \
  --data-only \
  --no-owner \
  --no-privileges \
  --exclude-table='_sqlx_migrations' \
  -f /tmp/nad8004_dump.sql

echo "==> Restoring to local Postgres..."
psql "$LOCAL_DB" -c "TRUNCATE agents, feedbacks, feedback_responses, activity_log, indexer_state CASCADE;" 2>/dev/null || true
psql "$LOCAL_DB" -f /tmp/nad8004_dump.sql

rm /tmp/nad8004_dump.sql

echo "==> Done! Local DB synced with Supabase data."
