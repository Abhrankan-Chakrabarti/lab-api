#!/bin/bash
# Simple health check for lab-api service
# Usage: ./health-check.sh
# Exit code: 0 = healthy, 1 = unhealthy

HEALTH_URL="https://abhrankan.duckdns.org/api/health"
TIMEOUT=5
RETRIES=2

for attempt in $(seq 1 $RETRIES); do
    response=$(curl -fsS --connect-timeout $TIMEOUT "$HEALTH_URL" 2>&1)
    http_code=$?
    
    if [ $http_code -eq 0 ]; then
        echo "[OK] Health check passed: $response"
        exit 0
    fi
    
    if [ $attempt -lt $RETRIES ]; then
        echo "[RETRY] Attempt $attempt/$RETRIES failed, retrying..."
        sleep 1
    fi
done

echo "[FAIL] Health check failed after $RETRIES attempts"
exit 1
