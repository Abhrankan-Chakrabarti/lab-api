#!/bin/bash

# Manual deployment script for website + lab-api
#
# Usage:
#   ./deploy.sh
#
# Run as ec2-user, not with sudo.
# The script uses sudo only for privileged installation and service management.

set -e

WEBSITE_DIR="/var/www/abhrankan"
LAB_API_SRC="/home/ec2-user/lab-api"
LAB_API_BIN="/usr/local/bin/lab-api"
SERVICE_NAME="lab-api.service"

LOG_FILE="/tmp/deploy-$(date +%Y%m%d-%H%M%S).log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

error() {
    log "ERROR: $*"
    exit 1
}

# The deployment workflow is intentionally run as ec2-user.
# Git and Cargo must run with the normal user's environment and ownership.
if [ "$(id -u)" -eq 0 ]; then
    error "Do not run deploy.sh with sudo. Run it as ec2-user: ./deploy.sh"
fi

# Required commands
for command in git cargo curl sudo; do
    if ! command -v "$command" >/dev/null 2>&1; then
        error "Required command not found: $command"
    fi
done

log "=== Deployment started ==="

# 1. Update website
log "Pulling website repository..."

cd "$WEBSITE_DIR" || error "Website directory not found: $WEBSITE_DIR"

git fetch origin || error "Failed to fetch website repo"
git pull origin main || error "Failed to pull website repo"

log "Website updated."

# 2. Update and build lab-api
log "Pulling lab-api repository..."

cd "$LAB_API_SRC" || error "Lab-api source directory not found: $LAB_API_SRC"

git fetch origin || error "Failed to fetch lab-api repo"
git pull origin main || error "Failed to pull lab-api repo"

log "Lab-api source updated."

log "Building lab-api..."

cargo build --release || error "Lab-api build failed"

log "Lab-api build successful."

# 3. Backup and install new binary
log "Installing new binary..."

if [ -f "$LAB_API_BIN" ]; then
    sudo cp "$LAB_API_BIN" \
        "$LAB_API_BIN.backup-$(date +%s)" \
        || error "Failed to back up previous binary"

    log "Backed up previous binary."
fi

sudo install -m 755 \
    "$LAB_API_SRC/target/release/lab-api" \
    "$LAB_API_BIN" \
    || error "Failed to install binary"

log "Binary installed: $LAB_API_BIN"

# 4. Restart service
log "Restarting $SERVICE_NAME..."

sudo systemctl restart "$SERVICE_NAME" \
    || error "Failed to restart service"

sleep 2

# 5. Check service status
if sudo systemctl is-active --quiet "$SERVICE_NAME"; then
    log "Service is running."
else
    error "Service failed to start"
fi

# 6. Health check
log "Running health check..."

if curl -fsS \
    --connect-timeout 5 \
    "https://abhrankan.duckdns.org/api/health" \
    > /dev/null; then

    log "Health check passed."

else
    log "WARNING: Health check failed (service may still be starting)"
fi

log "=== Deployment complete ==="
log "Log saved to: $LOG_FILE"
