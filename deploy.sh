#!/bin/bash

# Manual deployment script for website + lab-api
#
# Usage: ./deploy.sh
#
# Run as ec2-user (without sudo).
# The script uses sudo only for privileged operations.

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

# Deployment must be run as the normal user so Git and Cargo
# use the correct ownership and environment.
if [ "$EUID" -eq 0 ]; then
    error "Do not run deploy.sh with sudo. Run it as ec2-user: ./deploy.sh"
fi

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

# 3. Install new binary

log "Installing new binary..."

NEW_BINARY="$LAB_API_SRC/target/release/lab-api"

if [ ! -f "$NEW_BINARY" ]; then
    error "Built binary not found: $NEW_BINARY"
fi

# Remove any previous backup binaries from earlier deployments.
log "Removing previous binary backups..."

sudo find "$(dirname "$LAB_API_BIN")" \
    -maxdepth 1 \
    -type f \
    -name "$(basename "$LAB_API_BIN").backup-*" \
    -delete

# Install the new binary atomically.
TEMP_BINARY="${LAB_API_BIN}.new"

sudo install -m 755 "$NEW_BINARY" "$TEMP_BINARY" \
    || error "Failed to install new binary"

sudo mv "$TEMP_BINARY" "$LAB_API_BIN" \
    || error "Failed to replace existing binary"

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

HEALTH_URL="https://abhrankan.duckdns.org/api/health"

if curl -fsS --connect-timeout 5 "$HEALTH_URL" > /dev/null; then
    log "Health check passed."
else
    log "WARNING: Health check failed (service may still be starting)"
fi

log "=== Deployment complete ==="
log "Log saved to: $LOG_FILE"
