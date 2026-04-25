#!/bin/bash
# EVIF SQLite Database Restore Script
#
# Usage:
#   ./scripts/restore.sh ./backups/evif-db-hostname-20260408-120000.sqlite
#   ./scripts/restore.sh latest                                # Restore most recent backup
#   EVIF_DB_PATH=/data/evif.db ./scripts/restore.sh /path/to/backup.sqlite
#
# Environment variables:
#   EVIF_DB_PATH      — Path to SQLite database to restore to (default: /var/lib/evif/memory.db)
#   EVIF_DB_COPY_PATH — Path to save a pre-restore copy of the current DB (default: ${DB_PATH}.pre-restore)
#
# Exit codes:
#   0  — Restore succeeded
#   1  — Restore failed

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────
DB_PATH="${EVIF_DB_PATH:-/var/lib/evif/memory.db}"
DB_COPY="${EVIF_DB_COPY_PATH:-${DB_PATH}.pre-restore}"
BACKUP_SOURCE="${1:-}"

if [[ -z "$BACKUP_SOURCE" ]]; then
    echo "Usage: $0 <backup_file> | latest" >&2
    echo "" >&2
    echo "Examples:" >&2
    echo "  $0 ./backups/evif-db-server-20260408-120000.sqlite" >&2
    echo "  $0 latest" >&2
    echo "" >&2
    echo "Environment:" >&2
    echo "  EVIF_DB_PATH      = $DB_PATH" >&2
    echo "  EVIF_DB_COPY_PATH = $DB_COPY (pre-restore backup)" >&2
    exit 1
fi

# ── Locate backup file ─────────────────────────────────────
if [[ "$BACKUP_SOURCE" == "latest" ]]; then
    BACKUP_SOURCE=$(ls -1t ./backups/evif-db-*.sqlite* 2>/dev/null | head -1 || true)
    if [[ -z "$BACKUP_SOURCE" ]]; then
        echo "ERROR: No backup files found in ./backups/." >&2
        exit 1
    fi
    echo "[$(date -Iseconds)] Restoring from latest backup: $BACKUP_SOURCE"
fi

BACKUP_SOURCE="$(realpath "$BACKUP_SOURCE" 2>/dev/null || echo "$BACKUP_SOURCE")"

if [[ ! -f "$BACKUP_SOURCE" ]]; then
    echo "ERROR: Backup file not found: $BACKUP_SOURCE" >&2
    exit 1
fi

# ── Validate backup ────────────────────────────────────────
# Check it's a valid SQLite file
if [[ "$BACKUP_SOURCE" == *.gz ]]; then
    if ! command -v gzip &>/dev/null; then
        echo "ERROR: .gz backup but gzip is not installed." >&2
        exit 1
    fi
    # Gunzip to temp file for validation
    TEMP_DECOMPRESS=$(mktemp "${BACKUP_SOURCE%.gz}.XXXXXX")
    gzip -dc "$BACKUP_SOURCE" > "$TEMP_DECOMPRESS"
    if ! file "$TEMP_DECOMPRESS" | grep -q "SQLite"; then
        echo "ERROR: Decompressed file is not a valid SQLite database." >&2
        rm -f "$TEMP_DECOMPRESS"
        exit 1
    fi
    BACKUP_SOURCE="$TEMP_DECOMPRESS"
    ON_GZIP=1
else
    if ! file "$BACKUP_SOURCE" | grep -q "SQLite"; then
        echo "ERROR: File is not a valid SQLite database: $BACKUP_SOURCE" >&2
        exit 1
    fi
    ON_GZIP=0
fi

# ── Stop the server (if running) ───────────────────────────
# Attempt graceful shutdown via SIGTERM. If a PID file exists, use it.
EVIF_PID_FILE="${EVIF_PID_FILE:-/var/run/evif-rest.pid}"
if [[ -f "$EVIF_PID_FILE" ]]; then
    EVIF_PID=$(cat "$EVIF_PID_FILE")
    echo "[$(date -Iseconds)] Sending SIGTERM to EVIF (PID=$EVIF_PID) ..."
    kill -TERM "$EVIF_PID" 2>/dev/null || true
    # Wait up to 30s for graceful shutdown
    for i in $(seq 1 30); do
        if ! kill -0 "$EVIF_PID" 2>/dev/null; then
            echo "[$(date -Iseconds)] EVIF stopped."
            break
        fi
        sleep 1
    done
    if kill -0 "$EVIF_PID" 2>/dev/null; then
        echo "WARNING: EVIF did not stop gracefully — proceeding anyway." >&2
    fi
fi

# ── Backup current DB before overwriting ───────────────────
if [[ -f "$DB_PATH" ]]; then
    echo "[$(date -Iseconds)] Backing up current DB to: $DB_COPY"
    cp --reflink=auto "$DB_PATH" "$DB_COPY"
fi

# ── Restore ───────────────────────────────────────────────
# Ensure target directory exists
mkdir -p "$(dirname "$DB_PATH")"

if command -v sqlite3 &>/dev/null; then
    # Use SQLite's .restore for a clean restore
    echo "[$(date -Iseconds)] Restoring via sqlite3 .recover ..."
    sqlite3 "$DB_PATH" ".restore '$BACKUP_SOURCE'" 2>&1
else
    # Fall back to direct copy
    cp "$BACKUP_SOURCE" "$DB_PATH"
fi

# Clean up temp file if we decompressed
if [[ "${ON_GZIP:-0}" == "1" ]]; then
    rm -f "$BACKUP_SOURCE"
fi

# ── Verify restored DB ─────────────────────────────────────
DB_SIZE=$(du -h "$DB_PATH" | cut -f1)
BACKUP_SIZE=$(du -h "${EVIF_DB_COPY_PATH:-${DB_PATH}.pre-restore}" 2>/dev/null | cut -f1 || echo "unknown")
echo "[$(date -Iseconds)] Restore complete: $DB_PATH (size=${DB_SIZE})"
echo "[$(date -Iseconds)] Pre-restore copy: ${DB_COPY} (size=${BACKUP_SIZE})"

# ── Restart ────────────────────────────────────────────────
if [[ -f "$EVIF_PID_FILE" ]]; then
    echo "[$(date -Iseconds)] Restarting EVIF ..."
    # shellcheck disable=SC2086
    evif-rest &   # Assumes evif-rest is in PATH
fi

echo "[$(date -Iseconds)] Restore script done."
echo "" >&2
echo "IMPORTANT: Verify EVIF is running and healthy before removing the pre-restore copy:" >&2
echo "  curl http://localhost:8081/api/v1/health" >&2
echo "  # Then once verified:" >&2
echo "  rm '$DB_COPY'" >&2
