# lab-api

**A lightweight self-hosted Rust API for system snapshots, mathematics, and School data.**

`lab-api` is a Rust/Axum service with a public read-only compute API and a separate School data module. The core API provides health, metadata, mathematical calculations, and an authenticated host snapshot. School provides privacy-filtered student data and separately authorized admin details backed by SQLite.

The application is designed to run as a **localhost-only systemd service**, with Nginx providing the public HTTPS interface and reverse proxy.

## Features

- ⚡ Lightweight asynchronous Rust backend
- 🦀 Built with Axum and Tokio
- 🔒 Binds exclusively to `127.0.0.1`
- 🌐 HTTPS termination through Nginx
- 🔑 HTTP Basic Authentication for sensitive system information
- 📊 Linux system snapshot endpoint
- ℹ️ Public application information endpoint
- 🔢 Catalan, Fibonacci, GCD, prime-number, factorisation, totient, and Möbius computation
- 🏫 Read-only School database API with privacy-filtered student details
- 🛡️ Allowlisted admin access to full School student records
- 📥 Validated one-shot School database importer
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

For the `v0.9.0` release changes, see [`RELEASE_NOTES.md`](RELEASE_NOTES.md).

This README focuses on the public core `/api/*` surface. The separate School module is summarized below; see [`API.md`](API.md) and [`RELEASE_NOTES.md`](RELEASE_NOTES.md) for its routes, privacy boundary, admin authorization, and importer history.

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

### Math routes

The core math routes are public, read-only GET endpoints:

```http
GET /api/v1/math/catalan/:n
GET /api/v1/math/fibonacci/:n
GET /api/v1/math/gcd/:a/:b
GET /api/v1/math/is-prime/:n
GET /api/v1/math/next-prime/:n
GET /api/v1/math/prime-gap/:n
GET /api/v1/math/prime-pi/:n
GET /api/v1/math/pi/:n
GET /api/v1/math/factor/:n
GET /api/v1/math/totient/:n
GET /api/v1/math/mobius/:n
```

The original route remains available as a deprecated compatibility alias for
existing clients. New clients should use the canonical route above:

```http
GET /api/v1/catalan/:n
```

Catalan values use `u128` arithmetic and support `0 ≤ n ≤ 34`. Fibonacci values use `u128` arithmetic and support `0 ≤ n ≤ 186`; `F(186)` is the largest value in range and `n = 187` returns `400 Bad Request`.

Catalan response for `n = 10`:

```json
{
  "n": 10,
  "value": "16796"
}
```

Fibonacci response for `n = 10`:

```json
{
  "n": 10,
  "value": "55"
}
```

GCD uses the Euclidean algorithm and accepts any `u64` path values, including zero:

```bash
curl https://abhrankan.duckdns.org/api/v1/math/gcd/84/30
```

Prime-number routes are public and read-only:

```http
GET /api/v1/math/is-prime/:n
GET /api/v1/math/next-prime/:n
GET /api/v1/math/prime-gap/:n
GET /api/v1/math/prime-pi/:n
GET /api/v1/math/pi/:n
GET /api/v1/math/factor/:n
GET /api/v1/math/totient/:n
GET /api/v1/math/mobius/:n
```

Prime searches and prime gaps support `0 ≤ n ≤ 1,000,000`. The prime-counting
function π(n) uses the same bounded limit to keep memory usage predictable;
`/api/v1/math/pi/:n` is an alias for `/api/v1/math/prime-pi/:n`.

Factorisation is available at `/api/v1/math/factor/:n` for
`0 ≤ n ≤ 1,000,000`. It uses bounded trial division for this demonstration,
not as a general factoring service, and returns distinct prime factors with
their exponents in ascending order:

```json
{
  "n": 360,
  "factors": [
    { "prime": 2, "power": 3 },
    { "prime": 3, "power": 2 },
    { "prime": 5, "power": 1 }
  ]
}
```

Euler's totient and Möbius functions build on the same bounded factorisation
module:

```text
GET /api/v1/math/totient/:n
GET /api/v1/math/mobius/:n
```

Both endpoints accept `0 ≤ n ≤ 1,000,000`. Totient returns a string-valued
result, while Möbius returns `-1`, `0`, or `1`.

Examples:

```bash
curl https://abhrankan.duckdns.org/api/v1/math/is-prime/97
curl https://abhrankan.duckdns.org/api/v1/math/next-prime/100
curl https://abhrankan.duckdns.org/api/v1/math/prime-gap/1000
curl https://abhrankan.duckdns.org/api/v1/math/prime-pi/1000
curl https://abhrankan.duckdns.org/api/v1/math/factor/360
curl https://abhrankan.duckdns.org/api/v1/math/totient/36
curl https://abhrankan.duckdns.org/api/v1/math/mobius/30
```

```json
{
  "a": 84,
  "b": 30,
  "gcd": 6
}
```

The math routes do not require authentication; the snapshot route remains protected by Basic Auth at Nginx.

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
curl -u 'username' https://abhrankan.duckdns.org/api/v1/snapshot
```

Authentication is handled by Nginx rather than by the Rust application.

### Catalan Numbers

```http
GET /api/v1/math/catalan/:n
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
curl https://abhrankan.duckdns.org/api/v1/math/catalan/10
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
curl https://abhrankan.duckdns.org/api/v1/math/catalan/34
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

## School Module

The School API is a separate read-only SQLite surface under `/school/`; it is not part of the database-free core `/api/*` service. The active database path is configured with `SCHOOL_DB_PATH` and is opened read-only during normal operation.

In production, `/school/` serves the public portal UI and static assets. The exact `/school/api/health` route is also public. Nginx Basic Auth protects the remaining `/school/api/` routes; the backend does not re-implement that edge gate. Admin routes add an allowlist on top of it.

- `GET /school/` serves the public School portal.
- `GET /school/api/health` provides public School API health.
- `GET /school/api/tables` lists available tables; table pages and schemas require Nginx Basic Auth.
- `GET /school/api/tables/{table}/students/{student_code}` returns privacy-filtered student details (Nginx Basic Auth).
- `GET /school/api/admin/tables/{table}/students/{student_code}` returns full schema details only when Nginx Basic Auth succeeds and `X-Authenticated-User` names a user in `LAB_API_ADMIN_USERS`; access is recorded in the admin audit log.
- `GET /school/api/admin/audit` returns paginated admin student-detail audit events for allowlisted users.
- A validated one-shot importer can replace the School database after checking the candidate file; normal API requests do not write to it.

See [`API.md`](API.md) for the route and security contract and [`RELEASE_NOTES.md`](RELEASE_NOTES.md) for the School feature history.

---

## Project Structure

```text
lab-api/
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── API.md
├── DEPLOYMENT_GUIDE.md
├── RELEASE_NOTES.md
├── LICENSE
├── deploy.sh
├── health-check.sh
├── lab-api.service.hardened
├── static/
│   └── school/
│       ├── index.html
│       ├── login.html
│       ├── README.txt
│       └── school.js
└── src/
    ├── main.rs
    ├── prime.rs
    ├── factor.rs
    └── school/
        ├── api.rs
        ├── audit.rs
        ├── db.rs
        ├── import.rs
        └── mod.rs
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
- `serde_json` — JSON serialization and School query result values
- `tokio` — asynchronous runtime
- `rusqlite` — read-only School SQLite access and validation
- `tower-http` — static-file serving for the School portal

Test-only dependencies:

- `tower` — Axum router testing utilities

### `src/main.rs`

Contains:

- Core `/api/*` route wiring
- Health endpoint
- Application information endpoint
- System snapshot collection
- Catalan, Fibonacci, and GCD calculation
- Prime-number and prime-counting route wiring
- Prime factorisation route wiring
- Backward-compatible Catalan route
- Mounts the School API router from `src/school/`
- `lab-api import` command dispatch for the School database importer
- Unit and HTTP router tests for the core API
- Local TCP listener and Axum server initialization

### `src/prime.rs`

Contains bounded prime utilities used by the public math API:

- Primality testing
- Next- and previous-prime search
- Prime gaps
- Prime-counting function π(n)

### `src/factor.rs`

Contains bounded trial-division utilities used by the public number-theory API:

- Prime factorisation
- Euler's totient function φ(n)
- Möbius function μ(n)

### `src/school/`

Contains the School API, SQLite access, privacy filtering, admin authorization,
audit logging, and validated database import logic.

### `static/school/`

Contains the public School portal UI and its static assets. The directory is
served below `/school/`; the API routes remain below `/school/api/`.

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

The core `/api/*` service does not use a database. The School module uses local SQLite at `SCHOOL_DB_PATH`; it requires no database server or other external service.

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
curl http://127.0.0.1:8088/v1/math/catalan/10
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

    server_name abhrankan.duckdns.org;

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

    server_name abhrankan.duckdns.org;

    ssl_certificate /etc/letsencrypt/live/abhrankan.duckdns.org/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/abhrankan.duckdns.org/privkey.pem;

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
    auth_basic_user_file /etc/nginx/status.htpasswd;

    proxy_pass http://127.0.0.1:8088/v1/snapshot;

    proxy_http_version 1.1;

    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

Protect the School API while leaving the portal UI and exact health route public. Forward the authenticated username for admin checks:

```nginx
location = /school/api/health {
    proxy_pass http://127.0.0.1:8088;
}

location /school/api/ {
    auth_basic "School Database";
    auth_basic_user_file /etc/nginx/school.htpasswd;

    proxy_pass http://127.0.0.1:8088;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Authenticated-User $remote_user;
}

location /school/ {
    proxy_pass http://127.0.0.1:8088;
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
   ├── Public /api endpoints (health, info, math)
   │
   ├── Basic Authentication → /api/v1/snapshot
   │
   ├── Public UI and health → /school/ and /school/api/health
   └── Basic Authentication → /school/api/*
          │
          └── admin allowlist for full student detail and audit
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
- Authentication for protected endpoints (`/api/v1/snapshot`, `/school/api/*`)
- Forwarding requests to the local service
- Setting `X-Authenticated-User` from `$remote_user` for School admin authorization

The `/v1/snapshot` endpoint should remain protected when exposed through a public reverse proxy because it reveals information about the underlying host. The School API routes should remain protected because they expose student records; the portal UI and health check are intentionally public.

---

## Error Handling

Invalid Catalan-number requests return JSON errors.

For example:

```http
GET /api/v1/math/catalan/35
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
cargo fmt -- --check
cargo test
cargo build --release
```

Build the release binary:

```bash
cargo build --release
```

For a deployed instance, verify the public API:

```bash
curl -fsS https://abhrankan.duckdns.org/api/health
```

```bash
curl -fsS https://abhrankan.duckdns.org/api/v1/math/catalan/10
```

Verify that the protected endpoint requires authentication:

```bash
curl -i https://abhrankan.duckdns.org/api/v1/snapshot
```

Expected:

```text
HTTP/1.1 401 Unauthorized
```

Then authenticate:

```bash
curl -u 'username' https://abhrankan.duckdns.org/api/v1/snapshot
```

---

## Current API Surface

| Endpoint | Method | Authentication | Purpose |
|---|---|---|---|
| `/api/health` | GET | None | Service health |
| `/api/v1/info` | GET | None | Application metadata |
| `/api/v1/math/catalan/:n` | GET | None | Catalan number calculation |
| `/api/v1/math/fibonacci/:n` | GET | None | Fibonacci number calculation |
| `/api/v1/math/gcd/:a/:b` | GET | None | Greatest common divisor |
| `/api/v1/math/is-prime/:n` | GET | None | Primality test |
| `/api/v1/math/next-prime/:n` | GET | None | Next prime |
| `/api/v1/math/prime-gap/:n` | GET | None | Surrounding prime gap |
| `/api/v1/math/prime-pi/:n` | GET | None | Prime-counting function π(n) |
| `/api/v1/math/pi/:n` | GET | None | Prime-counting alias |
| `/api/v1/catalan/:n` | GET | None | Deprecated Catalan compatibility alias |
| `/api/v1/snapshot` | GET | Nginx Basic Auth | Host/system snapshot |
| `/school/` | GET | None | Public School portal UI |
| `/school/api/health` | GET | None | School API health |
| `/school/api/tables` | GET | Nginx Basic Auth | Available School tables |
| `/school/api/tables/{table}` | GET | Nginx Basic Auth | Paginated safe student rows |
| `/school/api/tables/{table}/schema` | GET | Nginx Basic Auth | Table schema |
| `/school/api/tables/{table}/students/{student_code}` | GET | Nginx Basic Auth | Privacy-filtered student detail |
| `/school/api/admin/tables/{table}/students/{student_code}` | GET | Nginx Basic Auth + admin allowlist | Full student detail and audit event |
| `/school/api/admin/audit` | GET | Nginx Basic Auth + admin allowlist | Admin access audit events |

Paths above are the public URL shapes when Nginx strips or prefixes as in the examples. Backend listen paths omit the `/api` prefix for core routes.

---

## Performance

`lab-api` is intentionally small.

The application uses:

- Rust
- Tokio
- Axum
- Minimal runtime state
- No database for the core `/api/*` handlers
- Read-only School SQLite access configured by `SCHOOL_DB_PATH`
- No external application services

This makes it suitable for lightweight deployments where a larger application stack would be unnecessary.

---

## Limitations

The current implementation is intentionally simple.

- Catalan computation is limited to `n <= 34`.
- Fibonacci computation is limited to `n <= 186` by the `u128` boundary.
- GCD accepts `u64` path values, including zero.
- Prime search and prime-counting requests are bounded at `n <= 1,000,000`.
- The snapshot endpoint is Linux-oriented.
- System information is collected directly from Linux files such as `/proc/loadavg` and `/proc/meminfo`.
- Uptime is obtained through the system `uptime` command.
- Authentication is delegated to the reverse proxy (plus an admin allowlist for full School detail).
- School SQLite is read-only during normal API operation; replacement uses the validated one-shot importer.
- No general application-level user management system is implemented.

---

## Roadmap

Potential future improvements include:

- [ ] Additional numerical algorithms
- [ ] More system metrics
- [ ] Structured application logging
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
