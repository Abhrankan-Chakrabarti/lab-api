# lab-api API contract

This document describes the current production-facing contract for `lab-api` as it is deployed today.

The service is intentionally small and intentionally narrow:

- the Rust application listens only on `127.0.0.1:8088`
- Nginx is the public entry point over HTTPS
- the backend is not directly exposed to the Internet
- the core `/api/*` service provides health, application metadata, read-only mathematical calculations, and an authenticated system snapshot; it does not use a database
- the separate `/school/*` surface contains a public static UI and a read-only SQLite API configured by `SCHOOL_DB_PATH`
- the public School UI and exact `/school/api/health` route are not authenticated; the remaining `/school/api/` routes are protected by Nginx HTTP Basic Authentication

## Service architecture

```text
Internet
   ↓
HTTPS
   ↓
Nginx
   ├── /api/*     → 127.0.0.1:8088  (lab-api core)
   └── /school/*  → 127.0.0.1:8088  (School module)
          ↓
      lab-api
          ↓
      systemd
```

## Public URL vs local backend URL

### Public URL

```text
https://abhrankan.duckdns.org/api/health
https://abhrankan.duckdns.org/api/v1/info
https://abhrankan.duckdns.org/api/v1/math/catalan/10
https://abhrankan.duckdns.org/api/v1/math/fibonacci/10
https://abhrankan.duckdns.org/api/v1/math/gcd/84/30
https://abhrankan.duckdns.org/api/v1/math/is-prime/97
https://abhrankan.duckdns.org/api/v1/math/next-prime/100
https://abhrankan.duckdns.org/api/v1/math/previous-prime/100
https://abhrankan.duckdns.org/api/v1/math/prime-gap/1000
https://abhrankan.duckdns.org/api/v1/math/prime-pi/1000
https://abhrankan.duckdns.org/api/v1/math/pi/1000
https://abhrankan.duckdns.org/api/v1/math/factor/360
https://abhrankan.duckdns.org/api/v1/math/totient/36
https://abhrankan.duckdns.org/api/v1/math/mobius/30
https://abhrankan.duckdns.org/api/v1/math/divisor-count/360
https://abhrankan.duckdns.org/api/v1/math/divisor-sum/360
https://abhrankan.duckdns.org/api/v1/snapshot
https://abhrankan.duckdns.org/school/
https://abhrankan.duckdns.org/school/api/health
https://abhrankan.duckdns.org/school/api/tables
https://abhrankan.duckdns.org/school/api/tables/II_A/students/1001
https://abhrankan.duckdns.org/school/api/admin/tables/II_A/students/1001
https://abhrankan.duckdns.org/school/api/admin/audit
```

These are consumed through Nginx, which terminates TLS and forwards traffic to the backend.

### Local backend URL

```text
http://127.0.0.1:8088/health
http://127.0.0.1:8088/v1/info
http://127.0.0.1:8088/v1/math/catalan/10
http://127.0.0.1:8088/v1/math/fibonacci/10
http://127.0.0.1:8088/v1/math/gcd/84/30
http://127.0.0.1:8088/v1/math/is-prime/97
http://127.0.0.1:8088/v1/math/next-prime/100
http://127.0.0.1:8088/v1/math/previous-prime/100
http://127.0.0.1:8088/v1/math/prime-gap/1000
http://127.0.0.1:8088/v1/math/prime-pi/1000
http://127.0.0.1:8088/v1/math/pi/1000
http://127.0.0.1:8088/v1/math/factor/360
http://127.0.0.1:8088/v1/math/totient/36
http://127.0.0.1:8088/v1/math/mobius/30
http://127.0.0.1:8088/v1/math/divisor-count/360
http://127.0.0.1:8088/v1/math/divisor-sum/360
http://127.0.0.1:8088/v1/snapshot
http://127.0.0.1:8088/school/
http://127.0.0.1:8088/school/api/health
http://127.0.0.1:8088/school/api/tables
http://127.0.0.1:8088/school/api/tables/II_A/students/1001
http://127.0.0.1:8088/school/api/admin/tables/II_A/students/1001
http://127.0.0.1:8088/school/api/admin/audit
```

The backend itself is only accessible from the local machine. It should not be exposed directly on a public interface or a public port.

## Endpoint summary

The routes in this table are **backend** paths. Public clients prefix core routes with `/api`. School routes already include their `/school` prefix.

| Method | Endpoint | Auth (public edge) | Purpose |
| --- | --- | --- | --- |
| GET | `/health` | None | Service health |
| GET | `/v1/info` | None | Non-sensitive application metadata and endpoint discovery |
| GET | `/v1/math/catalan/:n` | None | Catalan number, `0 ≤ n ≤ 34` |
| GET | `/v1/math/fibonacci/:n` | None | Fibonacci number, `0 ≤ n ≤ 186` |
| GET | `/v1/math/gcd/:a/:b` | None | Greatest common divisor of two `u64` values |
| GET | `/v1/math/is-prime/:n` | None | Primality test, `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/next-prime/:n` | None | Next prime greater than `n`, `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/previous-prime/:n` | None | Previous prime less than `n`, `2 < n ≤ 1,000,000` |
| GET | `/v1/math/prime-gap/:n` | None | Surrounding prime gap, `2 < n ≤ 1,000,000` |
| GET | `/v1/math/prime-pi/:n` | None | Prime-counting function π(n), `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/pi/:n` | None | Prime-counting function π(n), compatibility alias |
| GET | `/v1/math/factor/:n` | None | Prime factorisation of `n`, `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/totient/:n` | None | Euler's totient φ(n), `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/mobius/:n` | None | Möbius function μ(n), `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/divisor-count/:n` | None | Divisor-count function τ(n), `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/math/divisor-sum/:n` | None | Divisor-sum function σ(n), `0 ≤ n ≤ 1,000,000` |
| GET | `/v1/catalan/:n` | None | Deprecated Catalan compatibility alias; use `/v1/math/catalan/:n`, `0 ≤ n ≤ 34` |
| GET | `/v1/snapshot` | Nginx Basic Auth | Host/system snapshot |
| GET | `/school/` | None | Public School portal UI and static assets |
| GET | `/school/api/health` | None | School API health |
| GET | `/school/api/tables` | Nginx Basic Auth | Available School tables |
| GET | `/school/api/tables/{table}` | Nginx Basic Auth | Paginated, searchable safe student rows |
| GET | `/school/api/tables/{table}/schema` | Nginx Basic Auth | Discovered table schema |
| GET | `/school/api/tables/{table}/students/{student_id}` | Nginx Basic Auth | Privacy-filtered student detail |
| GET | `/school/api/admin/tables/{table}/students/{student_id}` | Nginx Basic Auth + admin allowlist | Full student detail and audit event |
| GET | `/school/api/admin/audit` | Nginx Basic Auth + admin allowlist | Paginated admin student-detail audit events |

`{table}` is a discovered class table name (for example `II_A`, `LPP`). `{student_id}` is the table’s primary-key value (for example `Student Code` or `Roll No`), not a nested path segment named `students`.

## School API

The core `/api/*` service does not use a database. The separate School API under `/school/` reads SQLite from `SCHOOL_DB_PATH` and is read-only during normal runtime. The `/school/` UI is served from the backend's `static/school` directory.

In production, the public `/school/` location serves the UI and static assets without authentication. The exact `/school/api/health` location is also public for health checks. Nginx Basic Authentication applies to `/school/api/` for table, schema, student, admin, and audit requests. The Rust process itself does not enforce Basic Authentication; it assumes that Nginx provides this edge protection. Admin detail and audit add an application allowlist on top of that.

The Nginx exact health location takes precedence over the broader protected `/school/api/` prefix:

```nginx
location = /school/api/health {
    proxy_pass http://127.0.0.1:8088;
}

location /school/api/ {
    auth_basic "School Database";
    auth_basic_user_file /etc/nginx/school.htpasswd;
    proxy_pass http://127.0.0.1:8088;
    proxy_set_header X-Authenticated-User $remote_user;
}

location /school/ {
    proxy_pass http://127.0.0.1:8088;
}
```

The `/school/api/` proxy preserves the `/school/api/...` path when forwarding to Axum. The `/school/` location is for the portal UI and static assets, not for bypassing API authorization.

### Table listing and schema

```http
GET /school/api/tables
GET /school/api/tables/{table}?limit=25&offset=0&search=Abhrankan
GET /school/api/tables/{table}/schema
```

All three routes require Nginx Basic Authentication. The table route returns
safe/display columns only and supports optional `limit`, `offset`, and `search`
query parameters. The default page size is 25 and the backend caps page sizes
at 100. Schema discovery returns the table's columns and primary-key metadata.

### Safe student detail

```http
GET /school/api/tables/{table}/students/{student_id}
```

Example:

```http
GET /school/api/tables/II_A/students/1001
```

This endpoint excludes explicitly sensitive contact, financial, and identity fields.

### Admin full student detail

```http
GET /school/api/admin/tables/{table}/students/{student_id}
```

Example:

```http
GET /school/api/admin/tables/II_A/students/1001
```

The admin endpoint returns every column discovered from the selected table schema. It requires both:

1. Nginx Basic Authentication for the protected `/school/api/` location.
2. The authenticated username in the backend `X-Authenticated-User` header and in `LAB_API_ADMIN_USERS`.

Configure the backend with a comma-separated allowlist, for example:

```text
LAB_API_ADMIN_USERS=abhrankan,teacher1,principal
```

Missing authentication at Nginx yields **401 Unauthorized**.  
Authenticated but non-admin (or missing `X-Authenticated-User`) yields **403 Forbidden**:

```json
{
  "error": "admin access required"
}
```

The backend listens only on `127.0.0.1:8088`, so the authenticated identity header is intended to be supplied by the local Nginx reverse proxy rather than by an Internet client. Nginx should set it inside the authenticated `/school/api/` location:

```nginx
location /school/api/ {
    auth_basic "School Database";
    auth_basic_user_file /etc/nginx/school.htpasswd;

    proxy_pass http://127.0.0.1:8088;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Authenticated-User $remote_user;
}
```

Do not expose the backend directly on a public interface. The public health route does not provide table or student data.

### Admin audit

```http
GET /school/api/admin/audit?limit=50&offset=0
```

This route requires both School API Basic Authentication and an authenticated
username listed in `LAB_API_ADMIN_USERS`. It returns recent admin student-detail
access events. `limit` defaults to 50 and is capped at 100; `offset` defaults
to 0. Admin student-detail responses are returned only after the corresponding
audit event is recorded.

## HTTP status codes

| Status | Meaning |
| --- | --- |
| 200 OK | Successful request |
| 400 Bad Request | Invalid mathematical input, including values above the documented prime limits |
| 401 Unauthorized | Missing or invalid HTTP Basic credentials at Nginx for protected School/API routes or the snapshot |
| 403 Forbidden | Authenticated School user is not on the admin allowlist (admin detail only) |
| 404 Not Found | Route not defined, or unknown table/student |
| 500 Internal Server Error | Unexpected backend failure |

The most important contract checks are:

- `GET /health` succeeds with `200`
- `GET /v1/info` succeeds with `200` and returns non-sensitive metadata
- `GET /v1/math/catalan/:n` succeeds with `200` when `0 ≤ n ≤ 34`
- `GET /v1/math/fibonacci/:n` succeeds with `200` when `0 ≤ n ≤ 186`
- `GET /v1/math/gcd/:a/:b` succeeds with `200` for valid `u64` path values
- `GET /v1/math/is-prime/:n` succeeds with `200` when `0 ≤ n ≤ 1,000,000`
- `GET /v1/math/next-prime/:n` succeeds with `200` when `0 ≤ n ≤ 1,000,000`
- `GET /v1/math/previous-prime/:n` succeeds with `200` when `2 < n ≤ 1,000,000`
- `GET /v1/math/prime-gap/:n` succeeds with `200` when `2 < n ≤ 1,000,000`
- `GET /v1/math/prime-pi/:n` and `/v1/math/pi/:n` succeed with `200` when `0 ≤ n ≤ 1,000,000`
- `GET /v1/catalan/:n` remains available as a deprecated compatibility alias; new clients should use `/v1/math/catalan/:n`
- `GET /v1/catalan/:n` fails with `400` when `n > 34`
- `GET /v1/math/fibonacci/:n` fails with `400` when `n > 186`
- `GET /v1/snapshot` fails with `401` without valid Basic Auth
- `GET /school/` succeeds without Basic Auth and serves the public portal
- `GET /school/api/health` succeeds without Basic Auth
- `GET /school/api/tables`, table, schema, student, admin, and audit routes fail with `401` without valid Basic Auth at the public edge
- `GET /school/api/admin/...` fails with `403` for non-admin users

## Math endpoints

The math endpoints are public, read-only GET routes. They use `u128` arithmetic where applicable and return numeric values as strings to preserve the exact result in JSON clients.

### Catalan

```http
GET /v1/math/catalan/:n
```

Supports `0 ≤ n ≤ 34`. The existing `GET /v1/catalan/:n` route is retained as a deprecated compatibility alias with the same response and limit.

### Fibonacci

```http
GET /v1/math/fibonacci/:n
```

Supports `0 ≤ n ≤ 186`. `F(186)` is the largest Fibonacci value representable by the implementation’s `u128` boundary; `n = 187` returns `400 Bad Request`.

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
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/gcd/84/30'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/fibonacci/10'
```

```json
{
  "a": 84,
  "b": 30,
  "gcd": 6
}
```

### Prime-number endpoints

```http
GET /v1/math/is-prime/:n
GET /v1/math/next-prime/:n
GET /v1/math/previous-prime/:n
GET /v1/math/prime-gap/:n
GET /v1/math/prime-pi/:n
GET /v1/math/pi/:n
GET /v1/math/factor/:n
GET /v1/math/totient/:n
GET /v1/math/mobius/:n
GET /v1/math/divisor-count/:n
GET /v1/math/divisor-sum/:n
```

These public, read-only endpoints support values up to `1,000,000`. The
`/v1/math/pi/:n` route is a compatibility alias for `/v1/math/prime-pi/:n`.
Prime-counting uses a bounded sieve so requests above the limit return
`400 Bad Request` rather than allocating unbounded memory. Prime searches and
prime gaps and factorisation use the same upper bound. Factorisation uses trial
division for this demonstration; it is not a general factoring service.

Examples:

```bash
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/is-prime/97'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/next-prime/100'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/previous-prime/100'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/prime-gap/1000'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/prime-pi/1000'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/factor/360'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/totient/36'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/mobius/30'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/divisor-count/360'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/divisor-sum/360'
```

```json
{
  "n": 97,
  "prime": true
}
```

`next-prime` and prime-counting return `{ "n": ..., "value": "..." }`.
`previous-prime` returns the greatest prime strictly less than `n` using the
same `{ "n": ..., "value": "..." }` response shape. It returns `400 Bad
Request` for `n ≤ 2`.
`prime-gap` returns the previous prime, next prime, and gap:

```json
{
  "n": 1000,
  "previous_prime": 997,
  "next_prime": 1009,
  "gap": 12
}
```

### Factorisation

`/v1/math/factor/:n` returns the prime factorisation of a value in
`0 ≤ n ≤ 1,000,000` as distinct prime factors and their exponents, ordered by
ascending prime. Values below `2` return an empty `factors` array; `1` has no
prime factors.

```bash
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/factor/360'
```

```json
{
  "n": 360,
  "factors": [
    {
      "prime": 2,
      "power": 3
    },
    {
      "prime": 3,
      "power": 2
    },
    {
      "prime": 5,
      "power": 1
    }
  ]
}
```

For example, factoring `1` is successful and returns an empty factor list:

```http
GET /v1/math/factor/1
```

```json
{
  "n": 1,
  "factors": []
}
```

### Totient and Möbius functions

The totient endpoint returns the count of positive integers up to `n` that
are coprime to `n`. The Möbius endpoint returns `0` when a squared prime
factor divides `n`, otherwise `-1` or `1` according to the parity of the
number of distinct prime factors. Both use bounded trial division through
`1,000,000`.

```http
GET /v1/math/totient/:n
GET /v1/math/mobius/:n
```

Examples:

```json
{
  "n": 36,
  "value": "12"
}
```

```json
{
  "n": 30,
  "value": -1
}
```

For `n = 0`, totient and Möbius both return `0`; `φ(1) = 1` and `μ(1) = 1`.

The divisor-count and divisor-sum endpoints use the same factorisation:

```http
GET /v1/math/divisor-count/:n
GET /v1/math/divisor-sum/:n
```

For `n = 360`, `τ(360) = 24` and `σ(360) = 1170`.
For `n = 0`, both endpoints return `0`; `τ(1) = 1` and `σ(1) = 1`.

```json
{ "n": 360, "value": "24" }
{ "n": 360, "value": "1170" }
```

## Application information endpoint

### Route

```http
GET /v1/info
```

This public endpoint describes the running application and its public route surface. It does not require authentication and must not expose hostnames, filesystem paths, credentials, secret environment variables, or system snapshot data. The `environment` field is only a non-secret deployment label.

### Request examples

```bash
curl -sS 'https://abhrankan.duckdns.org/api/v1/info'
curl -sS 'http://127.0.0.1:8088/v1/info'
```

### Response

```json
{
  "service": "lab-api",
  "api_version": "v1",
  "app_version": "0.9.1",
  "endpoints": [
    "GET /health",
    "GET /v1/info",
    "GET /v1/math/catalan/:n",
    "GET /v1/math/fibonacci/:n",
    "GET /v1/math/gcd/:a/:b",
    "GET /v1/math/is-prime/:n",
    "GET /v1/math/next-prime/:n",
    "GET /v1/math/previous-prime/:n",
    "GET /v1/math/prime-pi/:n",
    "GET /v1/math/pi/:n",
    "GET /v1/math/prime-gap/:n",
    "GET /v1/math/factor/:n",
    "GET /v1/math/totient/:n",
    "GET /v1/math/mobius/:n",
    "GET /v1/math/divisor-count/:n",
    "GET /v1/math/divisor-sum/:n",
    "GET /v1/catalan/:n",
    "GET /v1/snapshot",
    "GET /school/",
    "GET /school/api/health",
    "GET /school/api/tables",
    "GET /school/api/tables/:table",
    "GET /school/api/tables/:table/schema",
    "GET /school/api/tables/:table/students/:student_code",
    "GET /school/api/admin/tables/:table/students/:student_code",
    "GET /school/api/admin/audit"
  ],
  "build_profile": "release",
  "environment": "production"
}
```

### Status

- `200 OK` — Metadata returned
- `500 Internal Server Error` — Unexpected backend failure

`app_version` is taken from the package version at build time. `build_profile` identifies whether the binary was compiled with debug assertions. `environment` is an optional `LAB_API_ENV` deployment label, defaults to `unknown`, and must remain free of secrets.

## Health endpoint

### Request

```bash
curl -sS https://abhrankan.duckdns.org/api/health
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
GET /v1/math/catalan/:n
```

### Request examples

```bash
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/catalan/0'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/catalan/10'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/catalan/34'
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

### Out-of-range error

```bash
curl -sS -i 'https://abhrankan.duckdns.org/api/v1/math/catalan/35'
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
curl -sS -u 'username:password' https://abhrankan.duckdns.org/api/v1/snapshot
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

### Core `/api/*` (public edge)

- `GET /health` — unauthenticated
- `GET /v1/info` — unauthenticated
- `GET /v1/math/catalan/:n` — unauthenticated
- `GET /v1/math/fibonacci/:n` — unauthenticated
- `GET /v1/math/gcd/:a/:b` — unauthenticated
- `GET /v1/math/is-prime/:n` — unauthenticated
- `GET /v1/math/next-prime/:n` — unauthenticated
- `GET /v1/math/previous-prime/:n` — unauthenticated
- `GET /v1/math/divisor-count/:n` — unauthenticated
- `GET /v1/math/divisor-sum/:n` — unauthenticated
- `GET /v1/math/prime-gap/:n` — unauthenticated
- `GET /v1/math/prime-pi/:n` — unauthenticated
- `GET /v1/math/pi/:n` — unauthenticated
- `GET /v1/catalan/:n` — unauthenticated
- `GET /v1/snapshot` — Nginx HTTP Basic Auth

### School `/school/*` (public edge)

- `GET /school/` and static assets — unauthenticated
- `GET /school/api/health` — unauthenticated
- `GET /school/api/tables`, table, schema, and privacy-filtered student routes — Nginx HTTP Basic Auth
- `GET /school/api/admin/tables/...` and `GET /school/api/admin/audit` — Nginx HTTP Basic Auth **and** username in `LAB_API_ADMIN_USERS` via `X-Authenticated-User`

### Auth implementation

For `/v1/snapshot`, Nginx enforces HTTP Basic Authentication before proxying; the core Rust handler does not validate those credentials.

For School, Nginx authenticates only the protected `/school/api/` API location and forwards `$remote_user` as `X-Authenticated-User`. The admin detail and audit handlers additionally check that value against `LAB_API_ADMIN_USERS`. The public UI and health route do not receive this authentication gate.

Do not expose port `8088` publicly: without Nginx, School routes would not have the edge Basic Auth gate.

### Example Nginx blocks

```nginx
# Public core API (no auth)
location /api/ {
    proxy_pass http://127.0.0.1:8088/;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}

# Snapshot only
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

# Public School health
location = /school/api/health {
    proxy_pass http://127.0.0.1:8088;
}

# Protected School API
location /school/api/ {
    auth_basic "School Database";
    auth_basic_user_file /etc/nginx/school.htpasswd;
    proxy_pass http://127.0.0.1:8088;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Authenticated-User $remote_user;
}

# Public School UI and static assets
location /school/ {
    proxy_pass http://127.0.0.1:8088;
}
```

Exact matches for `/api/v1/snapshot` take precedence over the `/api/` prefix. Health, info, math, and the Catalan alias remain public.

## Nginx routing

```text
https://abhrankan.duckdns.org/api/...     →  http://127.0.0.1:8088/...
https://abhrankan.duckdns.org/school/...  →  http://127.0.0.1:8088/school/...
```

Examples:

```text
https://abhrankan.duckdns.org/api/health
  → http://127.0.0.1:8088/health

https://abhrankan.duckdns.org/api/v1/math/catalan/10
  → http://127.0.0.1:8088/v1/math/catalan/10

https://abhrankan.duckdns.org/api/v1/info
  → http://127.0.0.1:8088/v1/info

https://abhrankan.duckdns.org/school/api/tables/II_A/students/1001
  → http://127.0.0.1:8088/school/api/tables/II_A/students/1001
```

## Security model

- only localhost binding: `127.0.0.1:8088`
- no public TCP exposure for the Rust process
- HTTPS termination at Nginx
- Basic Auth for `/api/v1/snapshot`
- public `/school/` UI and `/school/api/health`
- Basic Auth for the protected `/school/api/` routes
- admin allowlist for full School student detail and audit access
- no database for the core `/api/*` service
- School SQLite via `SCHOOL_DB_PATH`, read-only at runtime
- no token system, OAuth, or general session framework

Trust boundary:

```text
Internet → HTTPS + Nginx → local lab-api → Linux host
                │
                ├── public: health, info, math
                ├── Basic Auth: snapshot
                └── Basic Auth: /school/*
                       └── allowlist: admin full detail
```

## systemd deployment

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
# Example:
# Environment=SCHOOL_DB_PATH=/var/lib/lab-api/School.db
# Environment=LAB_API_ADMIN_USERS=abhrankan
# Environment=LAB_API_ENV=production

[Install]
WantedBy=multi-user.target
```

The Rust process listens on localhost only; systemd does not expose it to the Internet.

## Example curl commands

### Health and math

```bash
curl -sS https://abhrankan.duckdns.org/api/health
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/catalan/10'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/fibonacci/10'
curl -sS 'https://abhrankan.duckdns.org/api/v1/math/gcd/84/30'
```

### Snapshot

```bash
curl -sS -i https://abhrankan.duckdns.org/api/v1/snapshot
# → 401 without credentials

curl -sS -u 'username:password' https://abhrankan.duckdns.org/api/v1/snapshot
```

### School (requires Basic Auth at the edge)

```bash
curl -sS -i https://abhrankan.duckdns.org/school/api/tables/II_A/students/1001
# → 401 without credentials

curl -sS -u 'username:password' \
  'https://abhrankan.duckdns.org/school/api/tables/II_A/students/1001'

curl -sS -u 'adminuser:password' \
  'https://abhrankan.duckdns.org/school/api/admin/tables/II_A/students/1001'
```

### Local debugging

```bash
curl -sS http://127.0.0.1:8088/health
curl -sS 'http://127.0.0.1:8088/v1/math/catalan/34'
curl -sS 'http://127.0.0.1:8088/v1/snapshot'
```

Local calls bypass Nginx auth; use only on the host.

## Operational notes

- This API is intentionally small and stable.
- No additional endpoints should be added without a matching documentation update.
- The current contract covers: health, public metadata, math routes, authenticated snapshot, and the separate read-only School API under `/school/`.
- If the service is extended later, update this document in the same change.

This is the current canonical API contract for the service.
