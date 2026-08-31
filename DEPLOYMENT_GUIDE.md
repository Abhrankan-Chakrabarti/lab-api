# EC2 Setup: Systemd Hardening + Health Check

This guide documents the current self-hosted deployment of `lab-api` on an EC2 instance.

The setup is intentionally lean:

- Nginx provides the public HTTPS interface.
- `lab-api` runs as a localhost-only systemd service.
- systemd provides process supervision and security hardening.
- `health-check.sh` provides a manual health check.
- `deploy.sh` provides manual deployment.
- No cron-based automation is required.
- No external monitoring stack is required.

---

## 1. Systemd Hardening

`lab-api` is intended to run as a dedicated systemd service rather than as a manually managed process.

The repository contains the hardened service configuration:

```text
lab-api.service.hardened
```

### Current Security Posture

The hardened service uses:

- `User=ec2-user`
- `NoNewPrivileges=true`
- `PrivateTmp=true`
- `ProtectSystem=strict`
- `ProtectHome=true`
- `RestrictAddressFamilies=AF_INET AF_INET6`
- `RestrictNamespaces=true`
- `LockPersonality=true`
- `ProtectKernelTunables=true`
- `ProtectKernelModules=true`
- `ProtectControlGroups=true`
- `Restart=on-failure`

The application itself listens only on:

```text
127.0.0.1:8088
```

Therefore the Rust process is not directly exposed to the Internet.

Nginx is responsible for the public HTTPS interface and reverse proxying.

### Apply the Hardened Service

Back up the currently installed service:

```bash
sudo cp /etc/systemd/system/lab-api.service \
    /etc/systemd/system/lab-api.service.backup
```

From the `lab-api` repository directory, install the hardened unit:

```bash
sudo cp lab-api.service.hardened \
    /etc/systemd/system/lab-api.service
```

Reload systemd:

```bash
sudo systemctl daemon-reload
```

Restart the service:

```bash
sudo systemctl restart lab-api.service
```

Enable it to start automatically after reboot:

```bash
sudo systemctl enable lab-api.service
```

Check the service:

```bash
systemctl status lab-api.service --no-pager
```

You can also verify:

```bash
systemctl is-enabled lab-api.service
systemctl is-active lab-api.service
```

### Testing the Hardened Service

After restarting the service, verify the public API:

```bash
curl -fsS https://abhrankan.duckdns.org/api/health
```

Test the Catalan endpoint:

```bash
curl -fsS \
    https://abhrankan.duckdns.org/api/v1/catalan/10
```

Test the protected snapshot endpoint:

```bash
curl -u 'abhrankan' \
    https://abhrankan.duckdns.org/api/v1/snapshot
```

Check recent service logs:

```bash
journalctl -u lab-api -n 20 --no-pager
```

### If the Service Fails

Inspect the service journal:

```bash
journalctl -u lab-api -e
```

If the failure appears to be caused by filesystem restrictions, temporarily revert to the previous unit:

```bash
sudo cp /etc/systemd/system/lab-api.service.backup \
    /etc/systemd/system/lab-api.service

sudo systemctl daemon-reload
sudo systemctl restart lab-api.service
```

The hardening should only be tightened as far as the application actually permits.

---

## 2. Manual Health Check

The repository includes:

```text
health-check.sh
```

This script performs a small number of retries against the public health endpoint and returns an appropriate exit status.

### Make It Executable

From the repository:

```bash
chmod +x health-check.sh
```

### Run It

```bash
./health-check.sh
```

A successful check returns exit code `0`.

A failed check returns exit code `1`.

The script is intentionally manual. It is not configured as a cron job or background monitoring service.

This keeps the deployment simple while still providing a convenient way to verify the service whenever needed.

---

## 3. Manual Deployment

The repository also includes:

```text
deploy.sh
```

This script performs the current manual deployment workflow.

### What It Does

The deployment script:

1. Pulls the latest website repository.
2. Pulls the latest `lab-api` source.
3. Builds the Rust application in release mode.
4. Backs up the currently installed binary.
5. Installs the new binary.
6. Restarts `lab-api.service`.
7. Verifies that the service is running.
8. Performs a public health check.
9. Writes deployment information to a timestamped log.

The script currently expects the production environment to use:

```text
Website:
    /var/www/abhrankan

lab-api source:
    /home/ec2-user/lab-api

Installed binary:
    /usr/local/bin/lab-api

systemd service:
    lab-api.service
```

### Run a Deployment

Make the script executable:

```bash
chmod +x deploy.sh
```

Run it from the repository:

```bash
./deploy.sh
```

The script can be run manually whenever a deployment is required.

It does not automatically execute after a Git push.

### Deployment Logs

Each deployment creates a timestamped log under:

```text
/tmp/deploy-YYYYMMDD-HHMMSS.log
```

For example:

```bash
ls -lt /tmp/deploy-*.log
```

The deployment script prints its progress while also writing the same information to the log.

---

## 4. Verification Checklist

After deploying or changing the systemd configuration, verify:

- [ ] `systemctl status lab-api` shows `active (running)`
- [ ] `systemctl is-enabled lab-api` reports `enabled`
- [ ] `curl https://abhrankan.duckdns.org/api/health` returns a successful response
- [ ] `curl https://abhrankan.duckdns.org/api/v1/catalan/5` returns a valid result
- [ ] `/api/v1/snapshot` requires Basic Authentication
- [ ] Authenticated snapshot requests succeed
- [ ] Nginx successfully proxies API requests
- [ ] `journalctl -u lab-api` shows no startup errors
- [ ] `health-check.sh` exits successfully

---

## 5. Nginx and HTTPS

The Rust application is not responsible for public TLS.

The intended request path is:

```text
Internet
    │
    │ HTTPS :443
    ▼
  Nginx
    │
    │ HTTP
    ▼
127.0.0.1:8088
    │
    ▼
 lab-api
```

Nginx handles:

- TLS termination
- HTTP → HTTPS redirection
- Public API routing
- Basic Authentication for the protected snapshot endpoint
- Reverse proxying to the localhost-only Rust service

The application should remain bound to:

```text
127.0.0.1:8088
```

This means port `8088` does not need to be publicly exposed in the EC2 security configuration.

After changing Nginx configuration, test it:

```bash
sudo nginx -t
```

If the configuration is valid:

```bash
sudo systemctl reload nginx
```

---

## 6. Operational Model

The current deployment intentionally avoids unnecessary infrastructure.

### Included

```text
GitHub
   │
   │ manual pull
   ▼
EC2
   │
   ├── Nginx
   │     └── HTTPS
   │
   └── lab-api
         └── systemd
              └── automatic restart
```

The repository provides the operational components required for this setup:

```text
lab-api/
├── .gitignore
├── Cargo.toml
├── API.md
├── DEPLOYMENT_GUIDE.md
├── RELEASE_NOTES.md
├── deploy.sh
├── health-check.sh
├── lab-api.service.hardened
└── src/
    └── main.rs
```

The deployment assets are therefore versioned alongside the application they operate.

---

## 7. What Is Deliberately Not Automated

The current setup does **not** use:

- Cron-based deployments
- GitHub webhooks
- Automatic `git pull`
- Docker
- Kubernetes
- Prometheus
- Grafana
- A dedicated monitoring stack
- Automatic alerting
- Scheduled S3 repository backups

These can be introduced later if the operational requirements justify them.

For a small personal service, manual deployment combined with systemd process supervision is sufficient.

---

## 8. Troubleshooting

| Issue | Solution |
|---|---|
| Service fails after hardening | Check `journalctl -u lab-api -e` and temporarily restore the backed-up unit |
| Health check fails | Verify Nginx is running and test `/api/health` manually |
| API is unreachable | Check `systemctl status lab-api` and Nginx configuration |
| Deployment fails during build | Verify the Rust toolchain with `cargo --version` |
| Binary does not update | Verify permissions for `/usr/local/bin/lab-api` |
| Snapshot returns `401 Unauthorized` | Supply valid Basic Authentication credentials |
| Nginx configuration fails | Run `sudo nginx -t` before reloading |
| Service does not restart after reboot | Verify `systemctl is-enabled lab-api` |

---

## Summary

The current EC2 deployment consists of four repository-managed operational assets:

1. `lab-api.service.hardened` — hardened systemd configuration
2. `health-check.sh` — manual service health verification
3. `deploy.sh` — manual deployment workflow
4. `DEPLOYMENT_GUIDE.md` — deployment and operational documentation

The application remains intentionally lightweight:

```text
Nginx
  │
  ▼
localhost:8088
  │
  ▼
lab-api
  │
  ▼
systemd
```

This provides HTTPS, reverse proxying, process supervision, service hardening, manual deployment, and basic health verification without introducing unnecessary infrastructure.