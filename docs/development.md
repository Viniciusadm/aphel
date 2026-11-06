# Development

Guide to running Aphel locally: Rust backend in `back/`, React frontend in `web/` and supporting services (MySQL and Mailpit) via Docker Compose.

## Prerequisites

| Tool | Version |
| --- | --- |
| Rust | The version pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) (edition 2024, with `rustfmt` and `clippy`); rustup installs it on the first `cargo` command. The minimum supported version for plugins stays `rust-version` in `back/Cargo.toml` |
| Node.js | 24 (with npm) |
| Docker | Recent Engine with the `docker compose` plugin |
| GNU Make | any recent version |

## Getting started

```bash
cp .env.example .env   # fill in APP_ENCRYPTION_KEY with: openssl rand -base64 32
make up
make dev-back
make dev-web
```

Open http://localhost:5173. On an empty database, the first access opens the setup wizard (`/setup`), a single flow that creates the organization (one installation = one company) and the administrator, saves the public address (the browser's origin, `http://localhost:5173` with Vite), applies the template recommended for the business and offers outgoing email, with backup restore on its first step. To get sample data after installation, run `cargo run -- demo seed` in `back/` (it refuses if there are already records, unless `--force` is used; `demo remove` deletes only the demo data).

- `make up` starts MySQL and Mailpit and waits for the MySQL healthcheck.
- `make dev-back` runs `cargo run -- serve` in `back/` (the `serve` subcommand is the default of the `aphel` binary). On the first run the web app is built, because `rust-embed` embeds `web/dist` at compile time.
- `make dev-web` runs Vite with a proxy from `/api` to the backend.
- `make down` tears down the environment without deleting the data. To reset only the configured database, stop the Aphel server and workers and run `cargo run -- db reset --confirm aphel` from `back/` (replace `aphel` with the exact database name in `DATABASE_URL`). This permanently deletes the company, users, records and database secrets, recreates the database and applies all migrations; the next access opens `/setup` again. It requires DROP and CREATE privileges and keeps files on disk or S3. To remove the infrastructure volumes instead: `docker compose down -v`.

The Rust workspace has the kernel (`back/kernel`), one crate per module (`back/modules/<key>`), local plugins (`back/plugins/<key>`), the binary (`back/app`) and `back/xtask`; `cargo run` in `back/` runs the binary (default member).

Aphel has a single `.env`, at the root of the repository. The backend reads variables from the environment and from a `.env` in the directory where it runs (`back/`) or in the directories above it, so it finds the root one. The defaults in `.env.example` already point to the MySQL from `make up`; the same file serves Docker Compose (`APP_PORT` and `MYSQL_*` variables, ignored by the binary). All variables are in [Configuration](configuration.md).

## Ports

| Port | Service |
| --- | --- |
| 3001 | Backend (`HTTP_HOST`/`HTTP_PORT`, default `127.0.0.1:3001`) |
| 5173 | Vite (frontend in development) |
| 3306 | MySQL 8.4 |
| 1025 | Mailpit SMTP |
| 8025 | Mailpit UI (http://127.0.0.1:8025) |

If a port is already taken on the machine, compose lets you override the host side: `MYSQL_PORT`, `MAILPIT_PORT` and `MAILPIT_UI_PORT`, and, in the `s3` profile, `OBJECT_STORE_PORT` and `OBJECT_STORE_BUCKET` (e.g. `MYSQL_PORT=3307 make up`). In that case also adjust `DATABASE_URL` or the corresponding setting in the interface.

## Email

The backend sends emails (account verification, password reset, invitation) through the SMTP configured in the interface (Settings → Outgoing email, or in the installer's optional step). In development builds the form comes prefilled with the compose Mailpit (`127.0.0.1:1025`, no TLS); open http://127.0.0.1:8025 to see the messages. Without SMTP configured, sending fails with `mail_not_configured` and the invitation shows the link to copy. Email links use the public address saved in Settings → General (the installer saves the browser's origin, `http://localhost:5173` with Vite).

## Database

Compose creates the `aphel` database and the `aphel`/`aphel` user. The `docker/mysql/init/01-grant-aphel.sql` script grants `ALL PRIVILEGES ON *.*` to the user. The script only runs when the volume is created; if the volume is old, recreate it with `docker compose down -v && make up`.

```
DATABASE_URL=mysql://aphel:aphel@127.0.0.1:3306/aphel
```

Migrations run when `aphel serve` boots (turn this off with `RUN_MIGRATIONS=false`) or manually with `cargo run -- migrate` in `back/`.

### Per-module migrations

Core migrations live in `back/kernel/migrations/` (`sqlx::migrate!`). Each module's migrations live in its crate, `back/modules/<key>/migrations/` (or `back/plugins/<key>/migrations/`), numbered **within the module** (`0001_*.sql`, `0002_*.sql`…) and declared in the `Manifest` with `Migration::new(n, description, include_str!("../migrations/000n_*.sql"))`. They run after the core ones, in module load order (dependencies first), and are recorded in `module_migrations`. An applied migration never changes: fix it with a new one. A module that needs to undo schema on `aphel modules uninstall <key> --purge` declares `Manifest::uninstall_sql`.

## Build modules

The binary compiles the modules listed in `back/aphel.modules.toml`. After changing that file, run `make modules` (`cargo run -p xtask -- modules sync`), which regenerates `back/app/Cargo.toml`, `back/app/src/modules.rs`, `web/src/modules.generated.ts`, `web/modules.aliases.json` and `web/tsconfig.modules.json` (committed). `make dev-back`, `make dev-web` and `make build` already call `make modules`; `make lint` checks with `--check`. Full guide: [Modules and plugins](modules/README.md).

## Tests

The project has no test suites and nothing runs per commit. The one automated check beyond compilation is the probe:

```bash
make probe
```

`aphel probe` needs no database or network and prints failures only (it exits non-zero on any). It checks access-scope filters, hook ordering, the record-validation priorities of the build, entity registry integrity (permissions, title fields, soft references), operation uniqueness, the event actor envelope, organization experience catalog validation and personal experience policy projection/validation. Run it after changes to those areas. Everything else is validated with `make lint` (module registries, `cargo fmt --check`, clippy, oxlint and typecheck) and by hand.

## Quality

```bash
make lint
make fmt
```

`make lint` runs `cargo run -p xtask -- modules sync --check`, `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `npm run lint` (oxlint) and `npm run typecheck`. CI (`.github/workflows/ci.yml`) runs the same checks, with MySQL 8.4 as a service.

## API types

```bash
make api-types
```

Generates `web/src/api/schema.d.ts` from `/api/openapi.json`, so the backend must be running (`make dev-back`). The interactive documentation is at http://127.0.0.1:3001/api/docs.

## Build and production image

```bash
make build
```

Runs `make modules` and `./build.sh`: `npm ci` and the web build, then `cargo build --release -p aphel`. The binary ends up in `back/target/release/aphel`, with the frontend embedded.

The Docker image (`docker/Dockerfile`) does the same in stages (first `make modules`, then web and binary) and produces a `debian-slim` image with a non-root user, running `aphel serve` on `0.0.0.0:3001`:

```bash
docker build -f docker/Dockerfile -t aphel .
```

`docker-compose.prod.yml` starts app, worker and MySQL, with a volume for `/app/storage`. Variables come from the root `.env` (see `.env.example`); `DATABASE_URL` is built from `MYSQL_USER`, `MYSQL_PASSWORD` and `MYSQL_DATABASE`, so avoid URL special characters in the password or adjust the file.

```bash
docker compose -f docker-compose.prod.yml up -d --build
```

The production walkthrough (reverse proxy, systemd, first access) is in [Installation](installation.md); backup in [Backup and restore](backup.md) and upgrading in [Upgrading](upgrading.md).

## Makefile commands

| Command | What it does |
| --- | --- |
| `make up` | Starts MySQL and Mailpit |
| `make down` | Tears down the environment |
| `make modules` | Generates the module registries from `back/aphel.modules.toml` |
| `make dev-back` | Backend at http://127.0.0.1:3001 |
| `make dev-web` | Vite with a proxy for `/api` |
| `make lint` | module registries, fmt check, clippy, oxlint, typecheck |
| `make probe` | Phase-gate invariant probe (no database, failures only) |
| `make fmt` | `cargo fmt` |
| `make api-types` | Generates the API types |
| `make build` | Release build (web + binary) |
