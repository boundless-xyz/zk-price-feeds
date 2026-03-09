#!/bin/bash
set -euo pipefail

# ─── Usage ────────────────────────────────────────────────────────────────────
usage() {
    cat <<EOF
Usage: ./run-keeper.sh [OPTIONS] [-- EXTRA_KEEPER_ARGS...]

Run the ZK Oracle keeper. Reads config from .env.

Options:
  --timeout DURATION   Stop after DURATION (e.g. "1h", "30m", "1d", "forever").
                       Default: forever.
  --restart            Auto-restart on crash (like systemd Restart=always).
                       Default: exit on crash.
  --restart-delay N    Seconds between restart attempts (default: 10).
  --background         Run in background (detach from terminal).
  -h, --help           Show this help.

Examples:
  ./run-keeper.sh                             # Run forever, exit on crash
  ./run-keeper.sh --restart                   # Run forever, auto-restart on crash
  ./run-keeper.sh --timeout 1d --restart      # Run for 1 day, restart on crash
  ./run-keeper.sh --background --restart      # Background + auto-restart

For production (Linux), use the systemd service instead:
  sudo cp zk-oracle-keeper.service /etc/systemd/system/
  sudo systemctl enable --now zk-oracle-keeper
EOF
    exit 0
}

# ─── Parse script args ───────────────────────────────────────────────────────
TIMEOUT=""
BACKGROUND=false
AUTO_RESTART=false
RESTART_DELAY=10
KEEPER_EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --timeout)       TIMEOUT="$2"; shift 2 ;;
        --restart)       AUTO_RESTART=true; shift ;;
        --restart-delay) RESTART_DELAY="$2"; shift 2 ;;
        --background)    BACKGROUND=true; shift ;;
        -h|--help)       usage ;;
        --)              shift; KEEPER_EXTRA_ARGS=("$@"); break ;;
        *)               KEEPER_EXTRA_ARGS+=("$1"); shift ;;
    esac
done

# Parse timeout into seconds (supports: 30s, 15m, 6h, 1d, forever, or raw seconds)
parse_timeout() {
    local val="$1"
    case "$val" in
        forever|"") echo 0 ;;
        *d) echo $(( ${val%d} * 86400 )) ;;
        *h) echo $(( ${val%h} * 3600 )) ;;
        *m) echo $(( ${val%m} * 60 )) ;;
        *s) echo "${val%s}" ;;
        *[0-9]) echo "$val" ;;
        *) echo "[run-keeper] ERROR: invalid timeout format '$val'" >&2; exit 1 ;;
    esac
}

TIMEOUT_SECS=$(parse_timeout "${TIMEOUT:-forever}")

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ─── Load .env ────────────────────────────────────────────────────────────────
if [[ ! -f .env ]]; then
    echo "[run-keeper] ERROR: .env file not found in $SCRIPT_DIR" >&2
    exit 1
fi
set -a
source .env
set +a

# ─── Validate required vars ──────────────────────────────────────────────────
REQUIRED_VARS=(PRIVATE_KEY POOL_ADDRESS ORACLE_ADDRESS RPC_URL)
MISSING=()
for var in "${REQUIRED_VARS[@]}"; do
    [[ -z "${!var:-}" ]] && MISSING+=("$var")
done
if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "[run-keeper] ERROR: Missing required env vars: ${MISSING[*]}" >&2
    exit 1
fi

# ─── Process management ──────────────────────────────────────────────────────
PROXY_PID=""
TIMER_PID=""
SHUTTING_DOWN=false

cleanup() {
    SHUTTING_DOWN=true
    echo ""
    echo "[run-keeper] Shutting down..."
    [[ -n "${TIMER_PID:-}" ]] && kill "$TIMER_PID" 2>/dev/null || true
    [[ -n "${PROXY_PID:-}" ]] && kill "$PROXY_PID" 2>/dev/null || true
    wait 2>/dev/null || true
    rm -f keeper.pid pinata-proxy.pid
    echo "[run-keeper] Stopped."
}
trap cleanup SIGTERM SIGINT EXIT

# ─── Start Pinata proxy if needed ────────────────────────────────────────────
if [[ "${PINATA_API_URL:-}" == *"127.0.0.1"* ]] || [[ "${PINATA_API_URL:-}" == *"localhost"* ]]; then
    if [[ ! -f pinata-proxy.py ]]; then
        echo "[run-keeper] ERROR: pinata-proxy.py not found (needed for local PINATA_API_URL)" >&2
        exit 1
    fi
    python3 pinata-proxy.py &
    PROXY_PID=$!
    echo "$PROXY_PID" > pinata-proxy.pid
    sleep 1
    if ! kill -0 "$PROXY_PID" 2>/dev/null; then
        echo "[run-keeper] ERROR: Pinata proxy failed to start" >&2
        exit 1
    fi
    echo "[run-keeper] Pinata v3->v1 proxy started (PID: $PROXY_PID)"
fi

# ─── Log setup ────────────────────────────────────────────────────────────────
LOG_DIR="logs"
mkdir -p "$LOG_DIR"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
KEEPER_LOG="${LOG_DIR}/keeper-${TIMESTAMP}.log"
ln -sf "keeper-${TIMESTAMP}.log" "${LOG_DIR}/latest.log"

# ─── Print config summary ────────────────────────────────────────────────────
TIMEOUT_DISPLAY="forever"
[[ "$TIMEOUT_SECS" -gt 0 ]] && TIMEOUT_DISPLAY="${TIMEOUT:-${TIMEOUT_SECS}s}"

echo "=== ZK Oracle Keeper ==="
echo "  Oracle:     ${ORACLE_ADDRESS}"
echo "  Pool:       ${POOL_ADDRESS}"
echo "  Max price:  ${MAX_PRICE:-SDK default} ETH"
echo "  Ramp-up:    ${RAMP_UP_PERIOD:-900}s"
echo "  Lock:       ${LOCK_TIMEOUT:-600}s"
echo "  Heartbeat:  ${HEARTBEAT_SECONDS:-3600}s"
echo "  Check:      ${CHECK_INTERVAL:-60}s"
echo "  Program:    ${PROGRAM_URL:-embedded}"
echo "  Duration:   ${TIMEOUT_DISPLAY}"
echo "  Restart:    ${AUTO_RESTART} (delay: ${RESTART_DELAY}s)"
echo "  Log:        ${KEEPER_LOG}"
echo ""

# ─── Background mode ─────────────────────────────────────────────────────────
if [[ "$BACKGROUND" == true ]]; then
    RELAUNCH_ARGS=(--timeout "${TIMEOUT:-forever}")
    [[ "$AUTO_RESTART" == true ]] && RELAUNCH_ARGS+=(--restart --restart-delay "$RESTART_DELAY")
    [[ ${#KEEPER_EXTRA_ARGS[@]} -gt 0 ]] && RELAUNCH_ARGS+=(-- "${KEEPER_EXTRA_ARGS[@]}")

    nohup "$0" "${RELAUNCH_ARGS[@]}" > "$KEEPER_LOG" 2>&1 &
    BG_PID=$!
    echo "$BG_PID" > keeper.pid
    echo "[run-keeper] Running in background (PID: $BG_PID)"
    echo "[run-keeper] Log:  tail -f ${KEEPER_LOG}"
    echo "[run-keeper] Stop: kill $BG_PID"
    trap - EXIT
    exit 0
fi

# ─── Timeout timer ────────────────────────────────────────────────────────────
if [[ "$TIMEOUT_SECS" -gt 0 ]]; then
    (
        sleep "$TIMEOUT_SECS"
        echo "[run-keeper] Timeout reached (${TIMEOUT_DISPLAY}). Stopping."
        kill $$ 2>/dev/null
    ) &
    TIMER_PID=$!
fi

# ─── Run keeper (with optional auto-restart) ─────────────────────────────────
RESTART_COUNT=0
START_TIME=$(date +%s)

while true; do
    if [[ "$SHUTTING_DOWN" == true ]]; then
        break
    fi

    if [[ $RESTART_COUNT -gt 0 ]]; then
        echo "[run-keeper] Restarting keeper (attempt $RESTART_COUNT, delay ${RESTART_DELAY}s)..."
        sleep "$RESTART_DELAY"
    fi

    # Run keeper. Using exec in a subshell so signals propagate cleanly.
    ./target/release/keeper "${KEEPER_EXTRA_ARGS[@]}" 2>&1 | tee -a "$KEEPER_LOG" &
    KEEPER_PID=$!
    echo "$KEEPER_PID" > keeper.pid

    if [[ $RESTART_COUNT -eq 0 ]]; then
        echo "[run-keeper] Keeper started (PID: $KEEPER_PID)"
    fi

    wait "$KEEPER_PID" 2>/dev/null || true
    EXIT_CODE=$?

    if [[ "$SHUTTING_DOWN" == true ]]; then
        break
    fi

    # Check timeout
    if [[ "$TIMEOUT_SECS" -gt 0 ]]; then
        ELAPSED=$(( $(date +%s) - START_TIME ))
        if [[ $ELAPSED -ge $TIMEOUT_SECS ]]; then
            echo "[run-keeper] Timeout reached. Exiting."
            break
        fi
    fi

    if [[ "$AUTO_RESTART" != true ]]; then
        echo "[run-keeper] Keeper exited with code $EXIT_CODE"
        exit "$EXIT_CODE"
    fi

    RESTART_COUNT=$((RESTART_COUNT + 1))
    echo "[run-keeper] Keeper exited with code $EXIT_CODE. Auto-restarting..."
done

echo "[run-keeper] Keeper stopped after $RESTART_COUNT restart(s)."
