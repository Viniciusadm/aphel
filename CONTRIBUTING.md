# How to contribute to Aphel

Thank you for your interest! This guide explains how to set up the environment, validate and submit changes. By participating, you agree to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Before you start

- **Bugs:** search the issues to see whether the problem has already been reported. If not, open one with the "Report a bug" template, with steps to reproduce and the version.
- **Features:** open an issue with the "Suggest a feature" template before writing a large amount of code, to align on the idea first.
- **Security:** do not open a public issue. Follow [SECURITY.md](SECURITY.md).
- Small changes (typos, documentation, obvious fixes) can go straight to a pull request.

## Environment

Requirements: rustup (the Rust version is pinned in `rust-toolchain.toml` and installed automatically), Node.js 24, Docker with `docker compose` and GNU Make.

```bash
cp .env.example .env   # fill in APP_ENCRYPTION_KEY with: openssl rand -base64 32
make up
make dev-back
make dev-web
```

- `make up` starts MySQL 8.4 and Mailpit (emails at http://127.0.0.1:8025).
- `make dev-back` runs the backend at http://127.0.0.1:3001; `make dev-web` runs Vite at http://localhost:5173.
- On an empty database, the first access opens the installation assistant.

Ports in use and other details: the Development section of the [README](README.md#development).

## Checks

```bash
make lint
make fmt
```

| Command | What it runs |
| --- | --- |
| `make lint` | `cargo fmt --check`, `cargo clippy -D warnings`, oxlint and TypeScript type checking |
| `make fmt` | Formats the Rust code |
| `make api-types` | Regenerates the frontend API types (`web/src/api/schema.d.ts`) |

CI runs the same checks on every pull request; a PR is only accepted when everything is green.

If you changed an API route or DTO, also regenerate the committed document: `cargo run -q -- openapi > openapi.json` in `back/` (CI fails if it is out of date) and then `make api-types`.

## Branches and commits

- Create a branch from the main branch following the `<type>/<short-name>` pattern, for example `feat/tag-filter`, `fix/kanban-order` or `chore/update-deps`.
- Commit messages in the `<type>: <description>` format, in English, in the imperative mood:
  - `feat:` new feature or visible behavior change.
  - `fix:` bug fix.
  - `chore:` maintenance, dependencies, build, CI, documentation, refactoring without behavior change.
- Examples: `feat: export organizations to XLSX`, `fix: make the Kanban total respect the owner filter`.
- Prefer small, cohesive commits. Do not mix a large refactoring with a bug fix.

## Pull requests

1. Keep the PR focused on one subject. Smaller PRs are reviewed faster.
2. Fill in the PR template: what changes, why, how to test, and screenshots when there is a visual change.
3. Update the affected documentation (`docs/`, `back/README.md`, `README.md`).
4. Run `make lint` before opening it.
5. Database changes go in as a new migration in `back/migrations/` (never edit a migration that has already been published).
6. Respond to review comments with new commits; squashing, if needed, is done on merge.

## Code rules

The project follows simple rules, which the review checks:

- **Everything in English in the code:** names of variables, functions, types and files. Texts the user sees always go through translation (i18n) and must exist in both supported languages, Brazilian Portuguese and English: `web/src/i18n/<locale>/<namespace>.json` and `web/src/modules/<key>/i18n/<locale>.json`. No hardcoded text in components.
- **No explanatory comments and no docstrings.** The code should explain itself through good names and good structure. Only `TODO`/`FIXME` and directives required by some tool are allowed.
- **Backend layers** (details in [`back/README.md`](back/README.md)):
  - `domain` is pure: business rules without axum, sqlx or serde.
  - `application` has the use cases, which only talk to the outside world through traits (ports).
  - `infrastructure` implements those traits (MySQL, SMTP, storage).
  - `api` is the HTTP layer (routes, DTOs, extractors).
- **Isolation between organizations:** every repository of organization data receives the organization scope (`TenantScope`) and filters by `organization_id`. There is no "unscoped" query for customer data.
- **Authorization in the use case:** the use case checks the permission explicitly (`scope.require(...)` or `scope.record_filter(...)`). Hiding a button on the frontend is not authorization. New permissions go into the catalog in `domain/access/permission.rs`.
- **Events and audit:** every business change writes the domain event and the audit in the same transaction as the write.
- **API:** REST at `/api/v1`, every route documented with `utoipa`; errors always in the `{ "error": { "code", "message", "details" } }` envelope; paginated lists (`items`, `total`, `page`, `per_page`). Non-GET requests require the `X-Requested-With: aphel` header.
- **Lists and custom fields** go through the query engine (`RecordQuery`) and the `CustomFieldWriter`; never build SQL with text coming from the user. Guide: [architecture](web/docs/src/content/docs/start/architecture.md).
- **Frontend:** each feature in `web/src/features/<feature>/`; API calls only through `web/src/api` (generated types); base components in `web/src/components/ui`; record screens reuse the blocks from `web/src/features/records`.

## Contribution license

Aphel is distributed under the [GNU AGPL v3.0 or later](LICENSE) (`AGPL-3.0-or-later`). By submitting a contribution, you agree that it will be distributed under the same license and declare that you have the right to submit it.
