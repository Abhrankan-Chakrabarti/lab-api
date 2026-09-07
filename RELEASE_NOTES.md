# lab-api v0.3.0

Release date: 2026-09-08

## Added

- Added `GET /v1/math/catalan/:n`.
- Added `GET /v1/math/fibonacci/:n` with `0 ≤ n ≤ 186`.
- Added `GET /v1/math/gcd/:a/:b`.
- Added unit and real Axum router tests for math values, boundaries, metadata, aliases, and unknown routes.

## Compatibility

- Preserved `GET /v1/catalan/:n` as a compatibility alias.
- Kept `/v1/info` public and `/v1/snapshot` behind the existing Nginx Basic Auth boundary.
- Kept the dependency footprint small and the existing localhost/systemd/Nginx deployment shape unchanged.

## Freeze scope

The service remains a small read-only API for health, metadata, mathematical calculations, and an authenticated system snapshot.

---

# lab-api v0.2.0

Release date: 2026-08-30

## Added

- Added public `GET /v1/info` application metadata endpoint.
- Exposed service name, API version, application version, available endpoints, build profile, and deployment environment label.
- Added API information documentation for public and local URLs.

## Changed

- Bumped the application version from `0.1.0` to `0.2.0`.
- Updated the README and canonical API contract for the new endpoint.

## Security

- `/v1/info` is intentionally unauthenticated because it exposes non-sensitive application metadata only.
- `/v1/snapshot` remains protected by Nginx HTTP Basic Authentication.
- The Rust service remains bound to `127.0.0.1:8088`; Nginx continues to provide the public HTTPS boundary.

## Compatibility

- Existing health, Catalan, and authenticated snapshot endpoints are unchanged.
- No database, external observability service, or new deployment component is required.

## Freeze scope

This release completes the small observability layer for `lab-api`. The service remains intentionally limited to four routes:

```text
/health
/v1/info
/v1/catalan/:n
/v1/snapshot
```

Further API expansion is deferred so the project can remain a focused systems, Rust, mathematics, and deployment demonstration.