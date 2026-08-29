# lab-api API contract

This document describes the current production-facing contract for `lab-api` as it is deployed today.

The service is intentionally small and intentionally narrow:

- the Rust application listens only on `127.0.0.1:8088`
- Nginx is the public entry point over HTTPS
- the backend is not directly exposed to the Internet
- the public API is limited to health, Catalan number calculation, and authenticated system snapshot data

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
https://example.com/api/v1/catalan/10
https://example.com/api/v1/snapshot
```

These are consumed through Nginx, which terminates TLS and forwards traffic to the backend.

### Local backend URL

```text
http://127.0.0.1:8088/health
http://127.0.0.1:8088/v1/catalan/10
http://127.0.0.1:8088/v1/snapshot
```

The backend itself is only accessible from the local machine. It should not be exposed directly on a public interface or a public port.

## Endpoint summary

| Method | Endpoint | Auth | Purpose |
| --- | --- | --- | --- |
| GET | /health | No | Service health |
| GET | /v1/catalan/:n | No | Catalan number, with `0 ≤ n ≤ 34` |
| GET | /v1/snapshot | Basic Auth | Host/system snapshot |

## HTTP status codes

The API uses the standard HTTP responses implied by the handler behavior:

| Status | Meaning |
| --- | --- |
| 200 OK | Successful request |
| 400 Bad Request | Invalid Catalan input such as `n > 34` |
| 401 Unauthorized | Missing or invalid HTTP Basic credentials |
| 404 Not Found | Route not defined |
| 500 Internal Server Error | Unexpected backend failure |

The most important contract checks are:

- `GET /health` succeeds with `200`
- `GET /v1/catalan/:n` succeeds with `200` when `0 ≤ n ≤ 34`
- `GET /v1/catalan/:n` fails with `400` when `n > 34`
- `GET /v1/snapshot` fails with `401` without valid Basic Auth

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
- `GET /v1/catalan/:n` is unauthenticated
- `GET /v1/snapshot` requires HTTP Basic Auth

### Auth implementation

The credential check is handled by Nginx, not the Rust application.

This is the desired deployment pattern because:

- the backend remains local-only
- auth is enforced before proxying
- the app does not need to store or validate user credentials
- the public HTTP layer is responsible for access control

### Example Nginx auth block

```nginx
location /api/ {
    auth_basic "lab-api";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://127.0.0.1:8088/;
}
```

This keeps the application simple while allowing Nginx to handle TLS and access control at the public edge.

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

If you use a prefix rewrite or a path mapping in Nginx, the exact external path layout may vary slightly, but the underlying route semantics remain the same: public HTTPS at the edge, private local backend behind it.

## Security model

The current security model is intentionally conservative:

- only localhost binding: `127.0.0.1:8088`
- no public TCP exposure for the Rust process
- HTTPS termination at Nginx
- Basic Auth for the system snapshot endpoint
- no database
- no application-layer user management
- no token system, no OAuth, no session handling

This is a small service with a minimal security boundary. The trust boundary is:

```text
Internet -> HTTPS + Nginx -> local lab-api process -> Linux host
```

The system snapshot endpoint is the only route with protected access. The health and Catalan endpoints are intentionally public and informational.

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
- The current contract is intentionally deliberate: a health check, a numeric calculation endpoint, and an authenticated snapshot endpoint.
- If the service is extended later, the contract should be updated in this document first.

This is the current canonical API contract for the service.
