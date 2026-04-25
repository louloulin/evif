#!/bin/bash
# EVIF SQLite Database Backup Script
#
# Usage:
#   ./scripts/backup.sh                    # Backup to ./backups/ with timestamp
#   EVIF_BACKUP_DIR=/data/backups ./scripts/backup.sh   # Custom backup directory
#   EVIF_DB_PATH=/data/evif.db ./scripts/backup.sh     # Custom DB path
#   EVIF_BACKUP_COMPRESSION=1 ./scripts/backup.sh      # Compress with gzip
#
# Environment variables:
#   EVIF_DB_PATH              — Path to SQLite database (default: /var/lib/evif/memory.db)
#   EVIF_BACKUP_DIR           — Directory to store backups (default: ./backups/)
#   EVIF_BACKUP_RETENTION     — Number of backups to keep (default: 30)
#   EVIF_BACKUP_COMPRESSION   — Set to "1" to gzip compress backups
#   EVIF_S3_BUCKET            — S3 bucket name (if set, upload to S3)
#   EVIF_S3_PREFIX            — S3 key prefix (default: evif/backups/)
#
# Exit codes:
#   0  — Backup succeeded
#   1  — Backup failed

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────
DB_PATH="${EVIF_DB_PATH:-/var/lib/evif/memory.db}"
BACKUP_DIR="${EVIF_BACKUP_DIR:-./backups}"
RETENTION="${EVIF_BACKUP_RETENTION:-30}"
USE_COMPRESSION="${EVIF_BACKUP_COMPRESSION:-0}"
S3_BUCKET="${EVIF_S3_BUCKET:-}"
S3_PREFIX="${EVIF_S3_PREFIX:-evif/backups/}"

# ── Validation ─────────────────────────────────────────────
if [[ ! -f "$DB_PATH" ]]; then
    echo "ERROR: Database file not found: $DB_PATH" >&2
    echo "Set EVIF_DB_PATH to point to your SQLite database." >&2
    exit 1
fi

BACKUP_DIR="$(realpath "$BACKUP_DIR")"
mkdir -p "$BACKUP_DIR"

# ── Timestamp ──────────────────────────────────────────────
TIMESTAMP=$(date '+%Y%m%d-%H%M%S')
HOSTNAME=$(hostname 2>/dev/null || echo "unknown")
BACKUP_FILENAME="evif-db-${HOSTNAME}-${TIMESTAMP}.sqlite"
BACKUP_PATH="${BACKUP_DIR}/${BACKUP_FILENAME}"

# ── Acquire shared lock and copy ───────────────────────────
# Use SQLite's .backup command for a consistent point-in-time snapshot.
# Falls back to cp if sqlite3 CLI is not available.
if command -v sqlite3 &>/dev/null; then
    echo "[$(date -Iseconds)] Performing online backup via sqlite3 VACUUM INTO ..."
    sqlite3 "$DB_PATH" ".backup '$BACKUP_PATH'" 2>&1
else
    echo "[$(date -Iseconds)] sqlite3 CLI not found — using cp (file-level copy) ..."
    cp --reflink=auto "$DB_PATH" "$BACKUP_PATH"
fi

if [[ ! -f "$BACKUP_PATH" ]]; then
    echo "ERROR: Backup file was not created: $BACKUP_PATH" >&2
    exit 1
fi

# ── Compression ─────────────────────────────────────────────
if [[ "$USE_COMPRESSION" == "1" ]]; then
    if command -v gzip &>/dev/null; then
        gzip -9 "$BACKUP_PATH"
        BACKUP_PATH="${BACKUP_PATH}.gz"
        echo "[$(date -Iseconds)] Compressed backup: ${BACKUP_PATH}"
    else
        echo "WARNING: gzip not found — skipping compression." >&2
    fi
fi

FINAL_SIZE=$(du -h "$BACKUP_PATH" | cut -f1)
echo "[$(date -Iseconds)] Backup complete: $BACKUP_PATH (${FINAL_SIZE})"

# ── S3 Upload ──────────────────────────────────────────────
if [[ -n "$S3_BUCKET" ]]; then
    if command -v aws &>/dev/null; then
        S3_KEY="${S3_PREFIX}${BACKUP_FILENAME}"
        if [[ "$USE_COMPRESSION" == "1" ]]; then
            S3_KEY="${S3_KEY}.gz"
        fi
        echo "[$(date -Iseconds)] Uploading to s3://${S3_BUCKET}/${S3_KEY} ..."
        aws s3 cp "$BACKUP_PATH" "s3://${S3_BUCKET}/${S3_KEY}" \
            --storage-class STANDARD_IA \
            --metadata "evif-timestamp=${TIMESTAMP},evif-host=${HOSTNAME}"
        echo "[$(date -Iseconds)] S3 upload complete."
    else
        echo "WARNING: aws CLI not found — skipping S3 upload." >&2
    fi
fi

# ── Retention — delete old backups ─────────────────────────
BACKUP_COUNT=$(ls -1 "$BACKUP_DIR"/evif-db-*.sqlite* 2>/dev/null | wc -l | tr -d ' ')
if [[ -n "$BACKUP_COUNT" && "$BACKUP_COUNT" -gt "$RETENTION" ]]; then
    # Sort by name (timestamp) and delete oldest
    echo "[$(date -Iseconds)] Retention: $BACKUP_COUNT backups > $RETENTION — deleting oldest ..."
    ls -1t "$BACKUP_DIR"/evif-db-*.sqlite* \
        | tail -n +$((RETENTION + 1)) \
        | xargs -r rm -v
fi

echo "[$(date -Iseconds)] Backup script done."
