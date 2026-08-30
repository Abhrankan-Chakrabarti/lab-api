# EC2 Setup: Systemd Hardening + Health Check

## Overview
This guide applies security hardening to `lab-api.service` and sets up basic health monitoring—no auto-deployment, no cron jobs, no monitoring stack.

---

## 1. Apply Systemd Hardening

### Current Security Posture
Your current `lab-api.service` already has:
- `NoNewPrivileges=true`
- `ProtectSystem=strict`
- `ProtectHome=true`
- `PrivateTmp=true`

### Enhancements
Add network-level restrictions and additional kernel/namespace protections:

**Step 1:** Back up current unit file
```bash
sudo cp /etc/systemd/system/lab-api.service /etc/systemd/system/lab-api.service.backup
```

**Step 2:** Copy the hardened unit file to the EC2 instance
```bash
# From your local machine:
scp lab-api.service.hardened ec2-user@<your-ec2-ip>:~/

# Then on the EC2 instance:
sudo cp ~/lab-api.service.hardened /etc/systemd/system/lab-api.service
```

**Step 3:** Reload systemd and restart the service
```bash
sudo systemctl daemon-reload
sudo systemctl restart lab-api.service
```

### Testing the Hardened Service

**Immediately test all API endpoints:**
```bash
# Health check
curl -fsS https://abhrankan.duckdns.org/api/health

# Catalan number lookup
curl -fsS https://abhrankan.duckdns.org/api/v1/catalan/10

# Snapshot (requires Basic Auth)
curl -u 'abhrankan' https://abhrankan.duckdns.org/api/v1/snapshot
```

**Check service status:**
```bash
systemctl status lab-api --no-pager
journalctl -u lab-api -n 20 --no-pager
```

**If the service fails to start:**
1. Check the journal: `journalctl -u lab-api -e`
2. The most likely culprit would be `ProtectSystem=strict` being too aggressive
3. Revert to the backup and try `ProtectSystem=full` instead
4. Re-test and gradually add restrictions

---

## 2. Simple Health Check

The `health-check.sh` script is a lightweight way to verify the API is responding.

**Setup:**
```bash
# Copy to EC2
scp health-check.sh ec2-user@<your-ec2-ip>:~/

# Make executable
chmod +x ~/health-check.sh

# Test it
./health-check.sh
```

**Usage (manual):**
```bash
./health-check.sh
# Exit code 0 = healthy, 1 = unhealthy
```

**Optional: Run manually periodically**
If you want to check health occasionally (not automated):
```bash
# Just run it manually when you log in
~/health-check.sh
```

**Do NOT add to cron yet** — avoid auto-alerting until you have a notification channel (email, Slack, PagerDuty, etc.)

---

## 3. Manual Deployment

The `deploy.sh` script handles updating both repositories and restarting services.

**Setup:**
```bash
# Copy to EC2
scp deploy.sh ec2-user@<your-ec2-ip>:~/

# Make executable
chmod +x ~/deploy.sh
```

**Usage (when you want to deploy):**
```bash
# SSH to EC2
ssh ec2-user@<your-ec2-ip>

# Run deployment
~/deploy.sh

# Monitor the log
tail -f /tmp/deploy-*.log
```

**What it does:**
1. Pulls latest website repo → `/var/www/abhrankan`
2. Pulls latest lab-api repo → `/home/ec2-user/lab-api`
3. Rebuilds lab-api binary
4. Backs up previous binary
5. Installs new binary to `/usr/local/bin/lab-api`
6. Restarts `lab-api.service`
7. Verifies service is running
8. Runs a health check

**Important:** This is purely manual. It doesn't auto-run on git push—you control when it executes.

---

## 4. Verification Checklist

After applying hardening, verify:

- [ ] `systemctl status lab-api` shows `active (running)`
- [ ] `curl https://abhrankan.duckdns.org/api/health` returns `{"ok":true,"service":"lab-api"}`
- [ ] `curl https://abhrankan.duckdns.org/api/v1/catalan/5` returns a valid number
- [ ] `curl -u abhrankan https://abhrankan.duckdns.org/api/v1/snapshot` succeeds (with Basic Auth)
- [ ] Nginx access logs show successful requests
- [ ] `journalctl -u lab-api` has no startup errors

---

## 5. Next Steps

**Immediate (done now):**
- ✅ Systemd hardening
- ✅ Health check script
- ✅ Manual deployment script

**Optional future work (only when needed):**
- Cron-based health monitoring (if you set up alerting)
- Automated nightly backups of repos to S3
- Nginx log rotation and retention policies
- CloudWatch metrics for instance CPU/memory

**Do NOT add yet:**
- Auto-deployment via cron
- Docker/Kubernetes
- Prometheus/Grafana
- Additional AWS services

---

## 6. Troubleshooting

| Issue | Solution |
|-------|----------|
| Service fails after hardening | Check `journalctl -u lab-api -e`; revert to `ProtectSystem=full` |
| Health check timeout | Verify Nginx is running and proxying correctly; check `/api/health` manually |
| Deploy script fails at build | Ensure Rust toolchain is installed; run `cargo --version` |
| Binary doesn't update | Confirm `/usr/local/bin/lab-api` has write permissions for ec2-user |

---

## Summary

You now have:
1. A hardened, resilient `lab-api.service` that restarts on failure
2. A manual health check script for verification
3. A manual deployment script you control completely
4. No auto-deployment, no cron jobs, no monitoring stack

This is a production-ready setup that remains lean and under your explicit control.
