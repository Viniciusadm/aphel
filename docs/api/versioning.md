# API versioning and compatibility

- The public API lives at `/api/v1`. The `info.version` field of the OpenAPI document (`/api/openapi.json`) is the version of Aphel that generated it (the same as in `Cargo.toml`).
- Only **compatible** changes go into `/api/v1`:
  - new resources, routes, optional parameters and headers;
  - new properties in responses (clients must ignore unknown properties);
  - new enumeration values in responses, new scopes and new error codes;
  - new accepted values in filters, `sort` and `include`.
- The following are **breaking** changes and can only ship in `/api/v2`:
  - removing or renaming a route, property, parameter, scope or error code;
  - changing the type or format of a property (e.g. a number that becomes a string);
  - making something that was optional required, or restricting accepted values;
  - changing the meaning of an HTTP status or an error code.
- When there is a `/api/v2`, `v1` stays up for at least 12 months, marked as `deprecated` in the OpenAPI and with the `Deprecation` header in responses, and the shutdown date is announced in `CHANGELOG.md`.
- The `back/openapi.json` committed in the repository is the published contract: CI generates the document, publishes it as an artifact and fails if the committed file diverges. Every contract change shows up in the PR diff and in the version's section of `CHANGELOG.md`.
