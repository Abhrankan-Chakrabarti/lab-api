# lab-api

**A lightweight self-hosted Rust API for system snapshots and numerical computation.**

`lab-api` is a small REST API written in Rust using [Axum](https://github.com/tokio-rs/axum). It provides service health checks, public application metadata, authenticated system information, and numerical computation endpoints.

The application is designed to run as a **localhost-only systemd service**, with Nginx providing the public HTTPS interface and reverse proxy.

## Features

- ⚡ Lightweight asynchronous Rust backend
- 🦀 Built with Axum and Tokio
- 🔒 Binds exclusively to `127.0.0.1`
- 🌐 HTTPS termination through Nginx
- 🔑 HTTP Basic Authentication for sensitive system information
- 📊 Linux system snapshot endpoint
- ℹ️ Public application information endpoint
- 🔢 Catalan number computation
- ❤️ Simple health-check endpoint
- ⚙️ systemd service support
- 🐧 Linux-oriented system information
- ☁️ Suitable for self-hosted deployments and small cloud instances

## Architecture

```text
                         Internet
                            │
                            │ HTTPS
                            ▼
                  ┌─────────────────────┐
                  │        Nginx        │
                  │                     │
                  │  TLS termination    │
                  │  Reverse proxy      │
                  │  Basic Auth         │
                  └──────────┬──────────┘
                             │
                             │ HTTP
                             ▼
                  ┌─────────────────────┐
                  │      lab-api        │
                  │                     │
                  │     Axum + Tokio    │
                  │                     │
                  │   127.0.0.1:8088    │
                  └─────────────────────┘
```

The Rust application is **not directly exposed to the Internet**.

It listens only on:

```text
127.0.0.1:8088
```

Nginx provides the public HTTPS interface and forwards requests to the local application.

For the canonical API contract, see [`API.md`](API.md).

For the `v0.2.0` release changes, see [`RELEASE_NOTES.md`](RELEASE_NOTES.md).

---

## API

When deployed behind the example Nginx configuration, the public API is available under `/api/`.

### Health

```http
GET /api/health
```

Returns a simple service health response.

Example:

```json
{
  "ok": true,
  "service": "lab-api"
}
```

This endpoint is intended to remain publicly accessible for basic service monitoring.

### API Information

```http
GET /api/v1/info
```

Returns non-sensitive application metadata, including:

- API version
- Application version
- Available endpoints
- Build profile
- Optional environment label

This endpoint is public and can be used for lightweight service discovery and frontend display.

See [`API.md`](API.md) for the complete response contract.

### System Snapshot

```http
GET /api/v1/snapshot
```

Returns information about the host running `lab-api`.

Example:

```json
{
  "hostname": "ip-172-31-77-184.ec2.internal",
  "uptime": "up 2 days, 2 hours, 52 minutes",
  "loadavg": "0.13 0.15 0.13",
  "mem_available_kb": 580200
}
```

The snapshot contains:

- Hostname
- System uptime
- 1, 5, and 15 minute load averages
- Available memory in KiB

#### Authentication

Because the snapshot exposes host-level information, the public Nginx endpoint is protected with **HTTP Basic Authentication**.

Without credentials:

```text
HTTP/1.1 401 Unauthorized
```

With valid credentials:

```bash
curl -u 'username' https://example.com/api/v1/snapshot
```

Authentication is handled by Nginx rather than by the Rust application.

### Catalan Numbers

```http
GET /api/v1/catalan/:n
```

Computes the `n`th Catalan number.

The implementation uses the recurrence:

```text
C(0) = 1

C(n) = C(n-1) × 2(2n-1) / (n+1)
```

Results are calculated using Rust's `u128` integer type.

The maximum supported value is:

```text
n = 34
```

Example:

```bash
curl https://example.com/api/v1/catalan/10
```

Response:

```json
{
  "n": 10,
  "value": "16796"
}
```

The largest supported input can also be requested:

```bash
curl https://example.com/api/v1/catalan/34
```

Response:

```json
{
  "n": 34,
  "value": "812944042149730764"
}
```

Requests where `n > 34` return:

```http
400 Bad Request
```

with:

```json
{
  "error": "n must be <= 34 for this demo (u128 limit)"
}
```

---

## Project Structure

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

### `.gitignore`

Ignores Rust build artifacts:

```text
/target
```

### `Cargo.toml`

Defines the Rust package and its dependencies:

- `axum` — HTTP routing and server framework
- `serde` — serialization and deserialization
- `serde_json` — JSON support
- `tokio` — asynchronous runtime

### `src/main.rs`

Contains:

- API routes
- Health endpoint
- Application information endpoint
- System snapshot collection
- Catalan number calculation
- JSON response structures
- Local TCP listener
- Axum server initialization

### `API.md`

Documents the canonical API contract, including:

- Available endpoints
- HTTP methods
- Authentication requirements
- Request and response formats
- Error responses
- Deployment-facing API behavior

### `RELEASE_NOTES.md`

Contains release-specific changes and notes for the project versions.

### `DEPLOYMENT_GUIDE.md`

Documents deployment and operational procedures for the self-hosted instance, including:

- systemd hardening
- service verification
- health checks
- manual deployment
- troubleshooting
- operational verification

### `lab-api.service.hardened`

Provides the hardened systemd unit used as the basis for the production `lab-api.service`.

It applies restrictions such as:

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

The file is a deployment configuration rather than application source code.

### `deploy.sh`

Provides a manual deployment workflow for the website and `lab-api`.

It:

1. Updates the website repository.
2. Updates the `lab-api` source repository.
3. Builds the release binary.
4. Backs up the currently installed binary.
5. Installs the new binary.
6. Restarts `lab-api.service`.
7. Verifies the service status.
8. Performs a public API health check.

The script is intentionally manual; it does not introduce automatic deployment or scheduled jobs.

### `health-check.sh`

Provides a small manual health-check utility for verifying that the deployed API is responding.

It retries the health endpoint and exits with:

- `0` when the service is healthy
- `1` when the health check fails

---

## Requirements

### Software

- Rust
- Cargo
- Linux
- Nginx for public HTTPS deployment
- systemd for service management

The application does not require a database or external application service.

---

## Build

Clone the repository:

```bash
git clone https://github.com/Abhrankan-Chakrabarti/lab-api.git
cd lab-api
```

Build the release binary:

```bash
cargo build --release
```

The resulting binary will be located at:

```text
target/release/lab-api
```

---

## Run Locally

Run the application directly:

```bash
cargo run --release
```

The server listens on:

```text
127.0.0.1:8088
```

Test the health endpoint:

```bash
curl http://127.0.0.1:8088/health
```

Expected response:

```json
{
  "ok": true,
  "service": "lab-api"
}
```

Test the Catalan endpoint:

```bash
curl http://127.0.0.1:8088/v1/catalan/10
```

---

## Production Deployment

A typical deployment places Nginx in front of the Rust application:

```text
Client
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

The application itself remains bound to localhost.

### Install the Binary

After building:

```bash
sudo install -m 755 target/release/lab-api /usr/local/bin/lab-api
```

### systemd

A systemd service can keep the API running and start it automatically after reboot.

Example:

```ini
[Unit]
Description=lab-api (personal compute API)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/lab-api
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Save the service as:

```text
/etc/systemd/system/lab-api.service
```

Reload systemd:

```bash
sudo systemctl daemon-reload
```

Enable the service:

```bash
sudo systemctl enable lab-api
```

Start it:

```bash
sudo systemctl start lab-api
```

Check its status:

```bash
systemctl status lab-api
```

Or:

```bash
systemctl is-enabled lab-api
systemctl is-active lab-api
```

---

## Nginx Configuration

Nginx can expose the API through an HTTPS domain while keeping the Rust service bound to localhost.

A simplified configuration looks like:

```nginx
server {
    listen 80;
    listen [::]:80;

    server_name example.com;

    location /.well-known/acme-challenge/ {
        root /var/www/letsencrypt;
    }

    location / {
        return 301 https://$host$request_uri;
    }
}

server {
    listen 443 ssl;
    listen [::]:443 ssl;

    server_name example.com;

    ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;

    location /api/ {
        proxy_pass http://127.0.0.1:8088/;

        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location / {
        root /var/www/example;
        index index.html;
        try_files $uri $uri/ =404;
    }
}
```

For the sensitive snapshot endpoint, Basic Authentication can be applied specifically to that route:

```nginx
location = /api/v1/snapshot {
    auth_basic "Private API";
    auth_basic_user_file /etc/nginx/api.htpasswd;

    proxy_pass http://127.0.0.1:8088/v1/snapshot;

    proxy_http_version 1.1;

    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

The exact Nginx configuration depends on the surrounding website and deployment.

After modifying the configuration:

```bash
sudo nginx -t
```

If the configuration test succeeds:

```bash
sudo systemctl reload nginx
```

---

## Security Model

`lab-api` intentionally binds to localhost:

```rust
let addr = SocketAddr::from(([127, 0, 0, 1], 8088));
```

This prevents direct external access to port `8088`.

The intended security boundary is:

```text
Internet
   │
   ▼
 HTTPS
   │
   ▼
 Nginx
   │
   ├── Public endpoints
   │
   └── Basic Authentication
          │
          ▼
      lab-api
          │
          ▼
   127.0.0.1:8088
```

Nginx is responsible for:

- TLS termination
- Public routing
- Authentication for protected endpoints
- Forwarding requests to the local service

The `/v1/snapshot` endpoint should remain protected when exposed through a public reverse proxy because it reveals information about the underlying host.

---

## Error Handling

Invalid Catalan-number requests return JSON errors.

For example:

```http
GET /api/v1/catalan/35
```

returns:

```http
HTTP/1.1 400 Bad Request
```

with:

```json
{
  "error": "n must be <= 34 for this demo (u128 limit)"
}
```

---

## Design Notes

### Why `u128`?

Catalan numbers grow rapidly.

Using `u128` provides substantially more range than standard 64-bit integers while keeping the implementation simple and allocation-free for the supported range.

The current implementation deliberately limits the input to `34`.

### Why bind to `127.0.0.1`?

The API is designed to sit behind Nginx.

Binding to localhost means the application port does not need to be directly exposed to the network.

This makes Nginx the central point for:

- TLS
- Authentication
- Request forwarding
- Public routing

### Why systemd?

systemd provides:

- Automatic startup
- Process supervision
- Restart handling
- Centralized service management
- Easy status inspection

---

## Testing

Run the Rust checks:

```bash
cargo check
```

Build the release binary:

```bash
cargo build --release
```

For a deployed instance, verify the public API:

```bash
curl -fsS https://example.com/api/health
```

```bash
curl -fsS https://example.com/api/v1/catalan/10
```

Verify that the protected endpoint requires authentication:

```bash
curl -i https://example.com/api/v1/snapshot
```

Expected:

```text
HTTP/1.1 401 Unauthorized
```

Then authenticate:

```bash
curl -u 'username' https://example.com/api/v1/snapshot
```

---

## Current API Surface

| Endpoint | Method | Authentication | Purpose |
|---|---|---|---|
| `/api/health` | GET | None | Service health |
| `/api/v1/info` | GET | None | Application metadata |
| `/api/v1/catalan/:n` | GET | None | Catalan number calculation |
| `/api/v1/snapshot` | GET | Basic Auth | Host/system snapshot |

---

## Performance

`lab-api` is intentionally small.

The application uses:

- Rust
- Tokio
- Axum
- Minimal runtime state
- No database
- No external application services

This makes it suitable for lightweight deployments where a larger application stack would be unnecessary.

---

## Limitations

The current implementation is intentionally simple.

- Catalan computation is limited to `n <= 34`.
- The snapshot endpoint is Linux-oriented.
- System information is collected directly from Linux files such as `/proc/loadavg` and `/proc/meminfo`.
- Uptime is obtained through the system `uptime` command.
- Authentication is delegated to the reverse proxy.
- No persistent application database is used.
- No application-level authentication system is implemented.

---

## Roadmap

Potential future improvements include:

- [ ] Additional numerical algorithms
- [ ] More system metrics
- [ ] Structured application logging
- [ ] Automated tests
- [ ] OpenAPI documentation
- [ ] Graceful shutdown handling
- [ ] More granular authentication and authorization
- [ ] Container deployment
- [ ] CI builds and release artifacts

The project intentionally remains small until these capabilities are actually needed.

---

## License

This project is licensed under the **MIT License**.

See [`LICENSE`](LICENSE) for the full license text.

---

## Author

**Abhrankan Chakrabarti**

GitHub: [Abhrankan-Chakrabarti](https://github.com/Abhrankan-Chakrabarti)

---

> `lab-api` is a small personal infrastructure project focused on keeping the implementation simple, lightweight, and easy to self-host.