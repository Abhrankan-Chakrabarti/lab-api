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