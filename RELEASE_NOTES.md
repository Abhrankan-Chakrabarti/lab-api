# lab-api v0.4.0

Release date: 2026-09-08

## Added

- Added a read-only School Database API backed by SQLite.
- Added dynamic school table discovery with `GET /school/api/tables`.
- Added dynamic schema discovery with `GET /school/api/tables/{table}/schema`.
- Added paginated table access with `GET /school/api/tables/{table}`.
- Added student-table search with bounded, literal substring matching.
- Added a School Database health endpoint at `GET /school/api/health`.
- Added a web-based School Database portal at `/school/`.
- Added comprehensive database-layer tests covering validation, pagination, search, schema discovery, table discovery, ordering, and sensitive-column filtering.

## Security

- School database access is read-only.
- The SQLite database is opened using read-only SQLite flags.
- Student responses expose only an explicitly allowed set of safe columns.
- Dynamic table and column identifiers are validated and safely quoted.
- Search values use parameterized SQL queries.
- SQL `LIKE` wildcards are escaped so user search input is treated literally.
- Pagination limits and search lengths are bounded.
- The School portal is designed to run behind Nginx HTTPS and HTTP Basic Authentication.
- The database is stored outside the application repository and is not directly exposed by Nginx.

## Configuration

- The School database path is configured through the `SCHOOL_DB_PATH` environment variable.
- The production database is located at `/var/lib/lab-api/School.db`.
- The Rust service remains bound to `127.0.0.1:8088`.
- Nginx continues to provide the public HTTPS and authentication boundary.

## Compatibility

- Existing health, mathematical, metadata, and authenticated snapshot endpoints remain unchanged.
- Preserved `GET /v1/catalan/:n` as a compatibility alias.
- The existing localhost/systemd/Nginx deployment shape remains unchanged.
- No additional application runtime, database server, container, or external service is required.

## Validation

- `cargo fmt` passes.
- Debug test suite: 22/22 tests passed.
- Release test suite: 22/22 tests passed.
- Debug build passes.
- Release build passes.

## Freeze scope

The project now provides two focused capabilities:

```text
Core API
/health
/v1/info
/v1/catalan/:n
/v1/math/catalan/:n
/v1/math/fibonacci/:n
/v1/math/gcd/:a/:b
/v1/snapshot

School Database
/school/
/school/api/health
/school/api/tables
/school/api/tables/{table}
/school/api/tables/{table}/schema
```

The School Database remains intentionally read-only in this release. Editing, authentication beyond the existing Nginx boundary, and more advanced student-management operations are deferred to a future release.

---

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