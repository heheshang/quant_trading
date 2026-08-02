#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export APP_IMAGE="${APP_IMAGE:-quant-trading-system:docker-test}"

echo "==> Building application image"
docker compose build app

echo "==> Starting PostgreSQL and Redis"
docker compose up -d postgres redis
docker compose ps

echo "==> Waiting for PostgreSQL and Redis health"
for i in $(seq 1 60); do
    if docker compose exec -T postgres pg_isready -U "${DATABASE_USERNAME:-quant}" -d "${POSTGRES_DB:-quant_trading}" >/dev/null 2>&1 \
        && docker compose exec -T redis redis-cli ping >/dev/null 2>&1; then
        break
    fi
    sleep 2
done

echo "==> Running migrations"
docker compose run --rm migrate

echo "==> Verifying migrated tables"
docker compose exec -T postgres \
    psql -U "${DATABASE_USERNAME:-quant}" -d "${POSTGRES_DB:-quant_trading}" \
    -tAc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';"

echo "==> Starting application smoke test"
docker compose run --rm app smoke

echo "==> Starting persistent application service"
docker compose up -d app
APP_CONTAINER="$(docker compose ps -q app)"
for i in $(seq 1 24); do
    if [ "$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$APP_CONTAINER" 2>/dev/null)" = "healthy" ]; then
        echo "==> Application is healthy"
        docker compose ps
        docker compose logs --tail=80 app
        exit 0
    fi
    sleep 5
done

echo "==> Application health check timed out"
docker compose ps
docker compose logs --tail=120 app
exit 1
