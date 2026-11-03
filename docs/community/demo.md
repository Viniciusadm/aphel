# Demo environment

A public Aphel demo: fictional data, a visitor with limited permissions, emails turned off, a warning banner and an automatic daily reset.

## Running with Docker

```bash
cp .env.example .env      # at the root: adjust passwords and APP_ENCRYPTION_KEY
docker compose -f docker/demo/compose.yml --env-file .env up -d --build
```

The [`compose.yml`](../../docker/demo/compose.yml) uses the production image ([`docker/Dockerfile`](../../docker/Dockerfile)) in three services, all with `DEMO_MODE=true`: `app` (web server and migrations), `worker` (queue and scheduler) and `reset` (the reset schedule), plus MySQL. Details in [`docker/demo/README.md`](../../docker/demo/README.md).

The demo is the installation itself (one installation = one company). On the first start, open the demo address and complete the installer (it stays available with `DEMO_MODE=true` until the installation exists; afterwards it responds 404): the company and the administrator created there are the demo's. The `reset` service waits for the installation and then runs `aphel demo seed` (which responds `demo_not_installed` while there is no active administrator). Then, as the administrator, save the public `https://` address in Settings → General.

## What `DEMO_MODE=true` does

| Behavior | Where |
| --- | --- |
| No email goes out: the sending email becomes a sink that only logs the subject (`demo mode: email discarded`), even with SMTP configured in the UI, and the integration's test email is refused | `serve` and `worker` |
| The `messaging` module's email channel refuses sends (permanent error `demo_restricted`; reading via IMAP continues) | `messaging` module |
| `GET /api/v1/demo` (public) reports `enabled`, the visitor's credentials and the reset time; the web shows the demo banner on every screen, including login, with the credentials | `DemoBanner` |
| Refuses (403 `demo_restricted`) password recovery, password change, signing out of all sessions, ending other sessions, 2FA and API tokens | API middleware |
| `aphel demo seed` creates or resets the visitor | CLI |

Variables: `DEMO_MODE`, `DEMO_USER_EMAIL` (default `demo@aphel.demo`), `DEMO_USER_PASSWORD` (default `aphel-demo`, public by definition) and `DEMO_RESET_TIME` (HH:MM in UTC, default `03:00`).

## Demo visitor

`aphel demo seed` creates the user `DEMO_USER_EMAIL` (or, if it already exists, resets the password, turns off 2FA, ends sessions and deletes the tokens of that user), with its own role `demo_visitor` ("Visitante da demonstração"): read, create, edit, delete and archive records (people, organizations, deals, activities, notes, products and goals) with organization scope, **without** export, settings, users and roles, audit, privacy, automations, lead distribution, system or organization export. Outside `DEMO_MODE`, the visitor is only created if `--visitor-email` is given.

## Daily reset

The `reset` service runs [`docker/demo/daily-reset.sh`](../../docker/demo/daily-reset.sh): it waits for the database (`aphel migrate`), seeds the demo if it does not exist yet and, every day at `DEMO_RESET_TIME` (UTC), deletes and seeds again:

```bash
aphel demo reset --full
```

`reset` deletes the demo data and seeds again. With `--full`, before seeding it deletes all business records, the files in storage and the installation's users, except the administrators (including whatever the visitors created). Without `--full`, it deletes only the data tracked in `demo_records`, along with those records' attachments and the files left without any other reference (row in `files` and object in storage). `aphel demo remove` deletes the tracked data without seeding again. A failure to delete an object from storage becomes a warning in the log and does not interrupt the reset.

**Why a container with a loop and not the worker's scheduler:** the reset deletes and recreates the installation's data and users, something that should not run inside the worker serving that same installation; the shell loop does not depend on cron in the image (the production image is `debian:bookworm-slim` without cron), survives restarts (it recomputes the next time) and retries once on failure. To scale, just replace the `reset` service with a host cron that runs the same command with `docker compose run --rm`.

## Limitations

- `reset` without `--full` does not delete files uploaded by visitors that are not attached to tracked records (`--full` deletes them all).
- Other message channels (WhatsApp from `messaging`) and modules that call external services follow their own configuration.
