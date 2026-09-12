# lab-api v0.5.1

Release date: 2026-09-12

## Added

- Added a read-only School Database API backed by SQLite.
- Added dynamic school table discovery with `GET /school/api/tables`.
- Added dynamic schema discovery with `GET /school/api/tables/{table}/schema`.
- Added paginated table access with `GET /school/api/tables/{table}`.
- Added student-table search with bounded, literal substring matching.
- Added clickable student detail access with `GET /school/api/tables/{table}/students/{student_code}`.
- Added a School Database health endpoint at `GET /school/api/health`.
- Added a web-based School Database portal at `/school/`.
- Added a validated School database importer through `lab-api import <candidate.db>`.
- Added SQLite integrity and schema validation before database activation.
- Added importer tests covering successful activation, malformed candidates, missing candidates, invalid schemas, and preservation of the existing database after failed imports.

## Security

- School database access is read-only during normal API operation.
- The SQLite database is opened using read-only SQLite flags.
- Student responses expose only an explicitly allowed set of safe columns.
- Dynamic table and column identifiers are validated and safely quoted.
- Search values use parameterized SQL queries.
- SQL `LIKE` wildcards are escaped so user search input is treated literally.
- Pagination limits and search lengths are bounded.
- Candidate databases are copied to a staging file and validated before activation.
- Invalid or incomplete candidates are rejected without replacing the active database.
- Unix activation uses atomic rename-based replacement.
- Windows activation uses rollback-safe replacement.
- The School portal is designed to run behind Nginx HTTPS and HTTP Basic Authentication.
- The database is stored outside the application repository and is not directly exposed by Nginx.

## Configuration

- The School database path is configured through the `SCHOOL_DB_PATH` environment variable.
- The production database is located at `/var/lib/lab-api/School.db`.
- The Rust service remains bound to `127.0.0.1:8088`.
- Nginx continues to provide the public HTTPS and authentication boundary.
- The importer uses the same `SCHOOL_DB_PATH` destination as normal server mode.
- Import mode is one-shot and does not start the HTTP server.

## Compatibility

- Existing health, mathematical, metadata, and authenticated snapshot endpoints remain unchanged.
- Preserved `GET /v1/catalan/:n` as a compatibility alias.
- The existing localhost/systemd/Nginx deployment shape remains unchanged.
- No additional application runtime, database server, container, or external service is required.
- The normal service process continues to operate with read-only database access.

## Validation

- `cargo fmt` passes.
- Debug test suite: **39/39 tests passed**.
- Clippy passes with `-D warnings`.
- The validated database importer was tested successfully with a real School database.
- Production deployment and end-to-end operation were verified.
- Failed-import behavior was tested to ensure the active database remains protected.

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
/school/api/tables/{table}/students/{student_code}

School Database Importer
lab-api import <candidate.db>
```

The School Database remains intentionally read-only during runtime. Database replacement is supported only through the separate, validated one-shot importer. Editing student records, HTTP-based database uploads, and more advanced student-management operations remain deferred to a future release.

---

# lab-api v0.5.0

Release date: 2026-09-08

## Added

- Added real pagination totals to `GET /school/api/tables/{table}` through the `total` response field.
- Added debounced search to the School Database portal.
- Added clickable student rows in the School Database portal.
- Added student detail access through `GET /school/api/tables/{table}/students/{student_code}`.
- Added safe student-detail responses restricted to explicitly allowed display columns.
- Added deterministic student-table ordering for consistent pagination.
- Added comprehensive tests for pagination totals, search behavior, student details, safe-column filtering, and API responses.

## Security

- Student detail responses expose only explicitly allowed safe columns.
- Student lookup is restricted to the selected table and `Student Code`.
- Existing read-only database access and SQL-safety protections remain unchanged.
- Search input remains bounded and is treated as literal text.
- Existing Nginx HTTPS and HTTP Basic Authentication boundaries remain unchanged.

## Compatibility

- Existing health, mathematical, metadata, snapshot, and School Database endpoints remain available.
- Existing table and schema discovery behavior remains unchanged.
- The existing localhost/systemd/Nginx deployment shape remains unchanged.
- No additional application runtime, database server, container, or external service is required.

## Validation

- Added unit tests for student-detail lookup and safe response filtering.
- Added API tests for successful student-detail responses and missing students.
- Verified pagination totals independently of the current page and search results.
- Verified deterministic ordering and non-overlapping pagination pages.

## Freeze scope

The School Database portal now supports:

```text
/school/
/school/api/health
/school/api/tables
/school/api/tables/{table}
/school/api/tables/{table}/schema
/school/api/tables/{table}/students/{student_code}
```

The School Database remains intentionally read-only. Database importing, validation, and replacement are introduced in v0.5.1 through the separate one-shot importer.

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