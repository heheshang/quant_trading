#!/usr/bin/env bash
set -Eeuo pipefail

APP_BIN=/usr/local/bin/quant-trading-system
MIGRATE_BIN=/usr/local/bin/migrate-db

run_app() {
    mkdir -p /app/logs /tmp/runtime-root
    export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/runtime-root}"
    export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
    export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
    exec dbus-run-session -- xvfb-run -a -s "-screen 0 1280x900x24" "$APP_BIN"
}

case "${1:-app}" in
    app)
        run_app
        ;;
    migrate)
        shift
        exec "$MIGRATE_BIN" "${@:-up}"
        ;;
    smoke)
        "$MIGRATE_BIN" up
        SMOKE_LOG="$(mktemp)"
        run_app >"$SMOKE_LOG" 2>&1 &
        APP_PID=$!
        trap 'kill "$APP_PID" 2>/dev/null || true' EXIT
        for _ in $(seq 1 "${SMOKE_WAIT_SECONDS:-30}"); do
            if grep -q "Application initialized successfully" "$SMOKE_LOG" 2>/dev/null; then
                echo "docker-smoke: application initialized successfully"
                kill "$APP_PID" 2>/dev/null || true
                wait "$APP_PID" 2>/dev/null || true
                exit 0
            fi
            if ! kill -0 "$APP_PID" 2>/dev/null; then
                echo "docker-smoke: application exited before initialization completed" >&2
                cat "$SMOKE_LOG" >&2 || true
                exit 1
            fi
            sleep 1
        done
        echo "docker-smoke: timed out waiting for application initialization" >&2
        cat "$SMOKE_LOG" >&2 || true
        exit 1
        ;;
    shell)
        shift
        exec "$@"
        ;;
    *)
        echo "unknown command: $1" >&2
        exit 1
        ;;
esac
