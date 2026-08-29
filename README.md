# lab-api

**A lightweight self-hosted Rust API for system snapshots and numerical computation.**

`lab-api` is a small REST API written in Rust using [Axum](https://github.com/tokio-rs/axum). It provides health checks, authenticated system information, and numerical computation endpoints.

The application is designed to run as a **localhost-only systemd service**, with Nginx handling public HTTPS access and reverse proxying.

## Features

- ⚡ Lightweight asynchronous Rust backend
- 🦀 Built with Axum and Tokio
- 🔒 Backend binds exclusively to `127.0.0.1`
- 🌐 HTTPS termination through Nginx
- 🔑 HTTP Basic Authentication for sensitive system information
- 📊 System snapshot endpoint
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
                ┌─────────────────────────┐
                │          Nginx          │
                │                         │
                │ TLS termination         │
                │ Reverse proxy           │
                │ Basic Authentication    │
                └────────────┬────────────┘
                             │
                             │ HTTP
                             ▼
                ┌─────────────────────────┐
                │        lab-api          │
                │                         │
                │       Axum + Tokio      │
                │                         │
                │  127.0.0.1:8088        │
                └─────────────────────────┘
```

The Rust application is **not directly exposed to the Internet**.

It listens only on:

```text
127.0.0.1:8088
```

Nginx provides the public HTTPS interface and proxies requests to the local application.

## API

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

---

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

The endpoint collects:

- Hostname
- System uptime
- 1, 5, and 15 minute load averages
- Available memory in KiB

### Authentication

Because the snapshot exposes host-level information, the public Nginx endpoint is protected with **HTTP Basic Authentication**.

Without credentials:

```text
HTTP/1.1 401 Unauthorized
```

With valid credentials:

```bash
curl -u 'username' https://example.com/api/v1/snapshot
```

The credentials are handled by Nginx rather than by the Rust application.

---

### Catalan Numbers

```http
GET /api/v1/catalan/:n
```

Computes the `n`th Catalan number.

The implementation uses:

```text
C(0) = 1

C(n) = C(n-1) × 2(2n-1) / (n+1)
```

The result is calculated using Rust's `u128` integer type.

For this implementation, the maximum supported value is:

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

Another example:

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

with a JSON error response:

```json
{
  "error": "n must be <= 34 for this demo (u128 limit)"
}
```

## Project Structure

```text
lab-api/
├── .gitignore
├── Cargo.toml
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
- `serde` — JSON serialization
- `serde_json` — JSON support
- `tokio` — asynchronous runtime

### `src/main.rs`

Contains:

- API routes
- Health endpoint
- System snapshot collection
- Catalan number calculation
- JSON response structures
- Local TCP listener
- Axum server initialization

## Requirements

### Software

- Rust
- Cargo
- Linux
- Nginx for public HTTPS deployment
- systemd for service management

The application itself does not require a database.

## Build

Clone the repository and enter the project directory:

```bash
git clone https://github.com/Abhrankan-Chakrabarti/lab-api.git
cd lab-api
```

Build the project:

```bash
cargo build --release
```

The resulting binary will be located at:

```text
target/release/lab-api
```

## Run Locally

Run the application directly:

```bash
cargo run --release
```

The server listens on:

```text
127.0.0.1:8088
```

Test it locally:

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

## Production Deployment

A recommended deployment places Nginx in front of the Rust application.

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

### Install the Binary

After building:

```bash
sudo install -m 755 target/release/lab-api /usr/local/bin/lab-api
```

### systemd

A systemd service can be used to keep the API running and start it automatically after reboot.

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

For the sensitive snapshot endpoint, Basic Authentication can be applied specifically to:

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

After modifying Nginx:

```bash
sudo nginx -t
```

If the configuration test succeeds:

```bash
sudo systemctl reload nginx
```

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
   └── Basic Auth
          │
          ▼
      lab-api
          │
          ▼
   127.0.0.1:8088
```

The `/v1/snapshot` endpoint should remain protected when exposed through a public reverse proxy because it reveals information about the underlying host.

## Error Handling

The API returns JSON errors for invalid Catalan-number requests.

Example:

```http
GET /api/v1/catalan/35
```

Response:

```http
HTTP/1.1 400 Bad Request
```

```json
{
  "error": "n must be <= 34 for this demo (u128 limit)"
}
```

## Design Notes

### Why `u128`?

Catalan numbers grow very quickly.

Using `u128` provides substantially more range than standard 64-bit integers while keeping the implementation simple and allocation-free for the supported range.

The current implementation deliberately limits the input to `34`.

### Why bind to `127.0.0.1`?

The API is intended to sit behind Nginx.

Binding to localhost means the backend does not need to expose its application port directly to the network.

This also makes Nginx the central point for:

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

## Testing

Run the Rust test/build checks:

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

## Current API Surface

| Endpoint | Method | Authentication | Purpose |
|---|---|---|---|
| `/api/health` | GET | None | Service health |
| `/api/v1/catalan/:n` | GET | None | Catalan number calculation |
| `/api/v1/snapshot` | GET | Basic Auth | Host/system snapshot |

## Performance

`lab-api` is intentionally small.

The application uses:

- Rust
- Tokio
- Axum
- Minimal runtime state
- No database
- No external services required by the application

This makes it suitable for lightweight deployments where a full application stack would be unnecessary.

## Limitations

The current implementation is intentionally simple.

- The Catalan endpoint is limited to `n <= 34`.
- The snapshot endpoint is Linux-oriented.
- System information is collected directly from Linux files such as `/proc/loadavg` and `/proc/meminfo`.
- Uptime is obtained through the system `uptime` command.
- Authentication is delegated to the reverse proxy.
- No persistent application database is used.
- No application-level authentication system is implemented.

## Roadmap

Potential future improvements include:

- [ ] Additional numerical algorithms
- [ ] More system metrics
- [ ] Structured application logging
- [ ] Automated tests
- [ ] API documentation / OpenAPI
- [ ] Graceful shutdown handling
- [ ] More granular authentication and authorization
- [ ] Container deployment
- [ ] CI builds and release artifacts

The project intentionally remains small until those capabilities are actually needed.

## License

This project is licensed under the **MIT License**.

See [`LICENSE`](LICENSE) for the full license text.

## Author

**Abhrankan Chakrabarti**

GitHub: [Abhrankan-Chakrabarti](https://github.com/Abhrankan-Chakrabarti)

---

> `lab-api` is a small personal infrastructure project focused on keeping the implementation simple, lightweight, and easy to self-host.