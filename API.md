# lab-api API contract

This document describes the current production-facing contract for `lab-api` as it is deployed today.

The service is intentionally small and intentionally narrow:

- the Rust application listens only on `127.0.0.1:8088`
- Nginx is the public entry point over HTTPS
- the backend is not directly exposed to the Internet
- the core `/api/*` service provides health, application metadata, read-only mathematical calculations, and an authenticated system snapshot; it does not use a database
- the separate `/school/*` surface is a read-only SQLite API configured by `SCHOOL_DB_PATH`

## Service architecture

```text
Internet
   ↓
HTTPS
   ↓
Nginx
   ↓
/api/*
   ↓
127.0.0.1:8088
   ↓
lab-api
   ↓
systemd
```

## Public URL vs local backend URL

### Public URL

```text
https://example.com/api/health
https://example.com/api/v1/info
https://example.com/api/v1/catalan/10
https://example.com/api/v1/math/catalan/10
https://example.com/api/v1/math/fibonacci/10
https://example.com/api/v1/math/gcd/84/30
https://example.com/api/v1/snapshot
https://example.com/school/api/tables/students/students/12345
https://example.com/school/api/admin/tables/students/students/12345
```

These are consumed through Nginx, which terminates TLS and forwards traffic to the backend.

### Local backend URL

```text
http://127.0.0.1:8088/health
http://127.0.0.1:8088/v1/info
http://127.0.0.1:8088/v1/catalan/10
http://127.0.0.1:8088/v1/math/catalan/10
http://127.0.0.1:8088/v1/math/fibonacci/10
http://127.0.0.1:8088/v1/math/gcd/84/30
http://127.0.0.1:8088/v1/snapshot
http://127.0.0.1:8088/school/api/tables/students/students/12345
http://127.0.0.1:8088/school/api/admin/tables/students/students/12345
```

The backend itself is only accessible from the local machine. It should not be exposed directly on a public interface or a public port.

## Endpoint summary

The routes in this table are backend paths. Public clients prefix core routes with `/api`; School routes are published under `/school`.

| Method | Endpoint | Auth | Purpose |
| --- | --- | --- | --- |
| GET | /health | No | Service health |
| GET | /v1/info | No | Non-sensitive application metadata and endpoint discovery |
| GET | /v1/math/catalan/:n | No | Catalan number, with `0 ≤ n ≤ 34` |
| GET | /v1/math/fibonacci/:n | No | Fibonacci number, with `0 ≤ n ≤ 186` |
| GET | /v1/math/gcd/:a/:b | No | Greatest common divisor of two `u64` values |
| GET | /v1/catalan/:n | No | Catalan number, compatibility alias, with `0 ≤ n ≤ 34` |
| GET | /v1/snapshot | Basic Auth | Host/system snapshot |
| GET | /school/api/tables/{table}/students/{student_code} | No | Privacy-filtered student detail |
| GET | /school/api/admin/tables/{table}/students/{student_code} | Basic Auth + admin allowlist | Full student detail |

## School API

The core `/api/*` service does not use a database. The separate School API under `/school/` reads SQLite from `SCHOOL_DB_PATH` and is read-only during normal runtime. Normal student details remain privacy-filtered.

### Safe student detail

```http
GET /school/api/tables/{table}/students/{student_code}
```

This endpoint excludes explicitly sensitive contact, financial, and identity fields.

### Admin full student detail

```http
GET /school/api/admin/tables/{table}/students/{student_code}
```

The admin endpoint returns every column discovered from the selected table schema. It requires both:

1. Nginx Basic Authentication for the `/school/` location.
2. The authenticated username in the backend `X-Authenticated-User` header and in `LAB_API_ADMIN_USERS`.

Configure the backend with a comma-separated allowlist, for example:

```text
LAB_API_ADMIN_USERS=abhrankan,teacher1,principal
```

Missing or non-admin usernames receive:

```json
{
  "error": "admin access required"
}
```

The backend listens only on `127.0.0.1:8088`, so the authenticated identity header is intended to be supplied by the local Nginx reverse proxy rather than by an Internet client. Nginx should set it inside the authenticated `/school/` location:

```nginx
proxy_set_header X-Authenticated-User $remote_user;
```

Do not expose the backend directly on a public interface.

## HTTP status codes

The API uses the standard HTTP responses implied by the handler behavior:

| Status | Meaning |
| --- | --- |
| 200 OK | Successful request |
| 400 Bad Request | Invalid Catalan input (`n > 34`) or Fibonacci input (`n > 186`) |
| 401 Unauthorized | Missing or invalid HTTP Basic credentials |
| 404 Not Found | Route not defined |
| 500 Internal Server Error | Unexpected backend failure |

The most important contract checks are:

- `GET /health` succeeds with `200`
- `GET /v1/info` succeeds with `200` and returns non-sensitive metadata
- `GET /v1/math/catalan/:n` succeeds with `200` when `0 ≤ n ≤ 34`
- `GET /v1/math/fibonacci/:n` succeeds with `200` when `0 ≤ n ≤ 186`
- `GET /v1/math/gcd/:a/:b` succeeds with `200` for valid `u64` path values
- `GET /v1/catalan/:n` remains available as a compatibility alias
- `GET /v1/catalan/:n` succeeds with `200` when `0 ≤ n ≤ 34`
- `GET /v1/catalan/:n` fails with `400` when `n > 34`
- `GET /v1/math/fibonacci/:n` fails with `400` when `n > 186`
- `GET /v1/snapshot` fails with `401` without valid Basic Auth

## Math endpoints

The math endpoints are public, read-only GET routes. They use `u128` arithmetic where applicable and return numeric values as strings to preserve the exact result in JSON clients.

### Catalan

```http
GET /v1/math/catalan/:n
```

Supports `0 ≤ n ≤ 34`. The existing `GET /v1/catalan/:n` route is retained as a compatibility alias with the same response and limit.

### Fibonacci

```http
GET /v1/math/fibonacci/:n
```

Supports `0 ≤ n ≤ 186`. `F(186)` is the largest Fibonacci value representable by the implementation's `u128` boundary; `n = 187` returns `400 Bad Request`.

Example response:

```json
{
   "n": 10,
   "value": "55"
}
```

### Greatest common divisor

```http
GET /v1/math/gcd/:a/:b
```

Computes the GCD of two `u64` path values using the Euclidean algorithm. Zero is valid, including `gcd(0, 0) = 0`.

```bash
curl -sS 'https://example.com/api/v1/math/gcd/84/30'
curl -sS 'https://example.com/api/v1/math/fibonacci/10'
```

```json
{
   "a": 84,
   "b": 30,
   "gcd": 6
}
```

## Application information endpoint

### Route

```http
GET /v1/info
```

This public endpoint describes the running application and its public route surface.
It does not require authentication and must not expose hostnames, filesystem paths,
credentials, secret environment variables, or system snapshot data. The `environment`
field is only a non-secret deployment label.

### Request examples

```bash
curl -sS 'https://example.com/api/v1/info'
curl -sS 'http://127.0.0.1:8088/v1/info'
```

### Response

```json
{
   "service": "lab-api",
   "api_version": "v1",
   "app_version": "0.7.1",
   "endpoints": [
      "GET /health",
      "GET /v1/info",
      "GET /v1/math/catalan/:n",
      "GET /v1/math/fibonacci/:n",
      "GET /v1/math/gcd/:a/:b",
      "GET /v1/catalan/:n",
      "GET /v1/snapshot"
   ],
   "build_profile": "release",
   "environment": "production"
}
```

### Status

- `200 OK` — Metadata returned
- `500 Internal Server Error` — Unexpected backend failure

`app_version` is taken from the package version at build time. `build_profile` identifies
whether the binary was compiled with debug assertions. `environment` is an optional
`LAB_API_ENV` deployment label, defaults to `unknown`, and must remain free of secrets.

## Health endpoint

### Request

```bash
curl -sS https://example.com/api/health
```

### Response

```json
{
  "ok": true,
  "service": "lab-api"
}
```

### Local backend example

```bash
curl -sS http://127.0.0.1:8088/health
```

### Status

- `200 OK`
- No authentication required

This endpoint is intended to remain publicly readable for simple service checks and monitoring.

## Catalan number endpoint

### Route

```http
GET /v1/catalan/:n
```

### Request examples

```bash
curl -sS 'https://example.com/api/v1/catalan/0'
curl -sS 'https://example.com/api/v1/catalan/10'
curl -sS 'https://example.com/api/v1/catalan/34'
```

### Success response

```json
{
  "n": 10,
  "value": "16796"
}
```

### Maximum supported value

The implementation computes Catalan numbers in `u128` and therefore intentionally limits input to:

```text
0 ≤ n ≤ 34
```

This is a deliberate product choice, not a general-purpose arbitrary-precision implementation.

The calculation uses the standard recurrence:

```text
C(0) = 1
C(n) = C(n-1) * 2 * (2n - 1) / (n + 1)
```

The underlying integer type is `u128`, so values larger than the range supported by that type would overflow. The application rejects exceeding values early with a request error instead of allowing undefined arithmetic behavior.

### Out-of-range error

```bash
curl -sS -i 'https://example.com/api/v1/catalan/35'
```

```http
HTTP/1.1 400 Bad Request
Content-Type: application/json

{
  "error": "n must be <= 34 for this demo (u128 limit)"
}
```

### Status

- `200 OK` when `0 ≤ n ≤ 34`
- `400 Bad Request` when `n > 34`
- no authentication required

## Snapshot endpoint

### Route

```http
GET /v1/snapshot
```

This endpoint provides a minimal system summary from the host running `lab-api`.

### Example response

```json
{
  "hostname": "ip-172-31-77-184.ec2.internal",
  "uptime": "up 2 days, 2 hours, 52 minutes",
  "loadavg": "0.13 0.15 0.13",
  "mem_available_kb": 580200
}
```

### Data collected

- hostname
- system uptime
- 1, 5, and 15 minute load averages
- available memory in KiB

### Request example

```bash
curl -sS -u 'username:password' https://example.com/api/v1/snapshot
```

### Local backend example

```bash
curl -sS -u 'username:password' http://127.0.0.1:8088/v1/snapshot
```

### Authentication behavior

The snapshot endpoint is protected by Nginx HTTP Basic Authentication.

The Rust application itself does not enforce credentials for this route. Authentication is enforced at the public entry point before traffic reaches the backend.

If the client omits credentials or provides invalid credentials, the response is:

```http
HTTP/1.1 401 Unauthorized
```

This is intentional. The endpoint exposes host-level information and is therefore treated as sensitive.

## Authentication model

### Basic Auth behavior

- `GET /health` is unauthenticated
- `GET /v1/info` is unauthenticated
- `GET /v1/math/catalan/:n` is unauthenticated
- `GET /v1/math/fibonacci/:n` is unauthenticated
- `GET /v1/math/gcd/:a/:b` is unauthenticated
- `GET /v1/catalan/:n` is unauthenticated
- `GET /v1/snapshot` requires HTTP Basic Auth

### Auth implementation

For `/v1/snapshot`, Nginx enforces HTTP Basic Authentication before proxying; the core Rust handler does not validate those credentials. The separate School admin endpoint also checks the forwarded `X-Authenticated-User` against `LAB_API_ADMIN_USERS` after Nginx authentication.

This keeps the backend local-only while placing public Basic Auth at the HTTPS edge and School-specific authorization in the School handler.

### Example Nginx auth block

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

location /api/ {
    proxy_pass http://127.0.0.1:8088/;

   proxy_http_version 1.1;
   proxy_set_header Host $host;
   proxy_set_header X-Real-IP $remote_addr;
   proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
   proxy_set_header X-Forwarded-Proto $scheme;
}
```

Only the exact snapshot route requires Basic Auth. Health, info, math, and the Catalan compatibility alias remain public.

## Nginx routing

The public interface is intentionally routed through Nginx. The pattern is:

```text
https://example.com/api/... -> Nginx -> http://127.0.0.1:8088/...
```

This allows the backend to remain local-only while still exposing a controlled HTTPS API surface.

For example:

```text
https://example.com/api/health
```

routes to:

```text
http://127.0.0.1:8088/health
```

and:

```text
https://example.com/api/v1/catalan/10
```

routes to:

```text
http://127.0.0.1:8088/v1/catalan/10
```

The information route follows the same mapping:

```text
https://example.com/api/v1/info -> http://127.0.0.1:8088/v1/info
```

If you use a prefix rewrite or a path mapping in Nginx, the exact external path layout may vary slightly, but the underlying route semantics remain the same: public HTTPS at the edge, private local backend behind it.

## Security model

The current security model is intentionally conservative:

- only localhost binding: `127.0.0.1:8088`
- no public TCP exposure for the Rust process
- HTTPS termination at Nginx
- Basic Auth for the system snapshot endpoint
- no database for the core `/api/*` service
- the separate `/school/*` API reads SQLite configured by `SCHOOL_DB_PATH` and is read-only during normal runtime
- no application-layer user management
- no token system, no OAuth, no session handling

This is a small service with a minimal security boundary. The trust boundary is:

```text
Internet -> HTTPS + Nginx -> local lab-api process -> Linux host
```

The system snapshot endpoint is the only protected route in the core `/api/*` service. Health, info, all math routes, and the Catalan alias are public. The School admin detail endpoint has its own Basic Auth and allowlist requirements described above.

## systemd deployment

The service is designed to run as a background systemd service on the host.

A typical unit file is shaped like this:

```ini
[Unit]
Description=lab-api
After=network.target

[Service]
Type=simple
User=lab-api
WorkingDirectory=/srv/lab-api
ExecStart=/usr/local/bin/lab-api
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
```

This keeps the service under traditional Linux supervision while Nginx remains responsible for Internet-facing access.

The important operational point is that the Rust process listens on localhost only; systemd does not expose it directly to the Internet.

## Example curl commands

### Health check

```bash
curl -sS https://example.com/api/health
```

### Catalan number

```bash
curl -sS 'https://example.com/api/v1/catalan/10'
curl -sS 'https://example.com/api/v1/math/fibonacci/10'
curl -sS 'https://example.com/api/v1/math/gcd/84/30'
```

### Snapshot with auth

```bash
curl -sS -u 'username:password' https://example.com/api/v1/snapshot
```

### Snapshot without auth

```bash
curl -sS -i https://example.com/api/v1/snapshot
```

This produces:

```http
HTTP/1.1 401 Unauthorized
```

### Local debugging

```bash
curl -sS http://127.0.0.1:8088/health
curl -sS 'http://127.0.0.1:8088/v1/catalan/34'
curl -sS -u 'username:password' http://127.0.0.1:8088/v1/snapshot
```

## Operational notes

- This API is intentionally small and stable.
- No additional endpoints should be added without a matching documentation update.
- The current contract is intentionally deliberate: health, public application metadata, math routes, an authenticated snapshot, and the separate read-only School API under `/school/`.
- If the service is extended later, the contract should be updated in this document first.

This is the current canonical API contract for the service.
