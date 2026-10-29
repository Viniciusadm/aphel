# Aphel — demo environment

A dedicated Compose setup for a public demo, on top of the production image (`docker/Dockerfile`). Full guide: [`docs/community/demo.md`](../../docs/community/demo.md).

```bash
cp .env.example .env
# edit at the root: MySQL passwords and APP_ENCRYPTION_KEY (aphel secrets generate-key)
docker compose -f docker/demo/compose.yml --env-file .env up -d --build
```

On the first start, open `http://<host>:APP_PORT` and complete the installer: the company and administrator created there are the demo's. The `reset` service waits for the installation and seeds the fictitious data.

| Service | Role |
| --- | --- |
| `app` | `aphel serve` with `DEMO_MODE=true`, runs the migrations and publishes the `APP_PORT` port |
| `worker` | `aphel worker` (queue, scheduler, notifications); emails are discarded |
| `reset` | [`daily-reset.sh`](daily-reset.sh): seeds the demo as soon as the installation exists and, every day at `DEMO_RESET_TIME` (UTC), runs `aphel demo reset --full` (deletes business data, files and non-administrator users and seeds again) |
| `mysql` | Dedicated database (`aphel_demo`) |

The public address, community catalog and other adjustments are in Settings (sign in as the demo administrator); visitors do not have access to settings.

Useful commands (at the repository root; the `.env` is the one at the root):

```bash
docker compose -f docker/demo/compose.yml --env-file .env logs -f reset  # next reset and its result
docker compose -f docker/demo/compose.yml --env-file .env exec app aphel demo reset --full
docker compose -f docker/demo/compose.yml --env-file .env exec app aphel demo seed --force
```

Put the demo behind an HTTPS proxy and save the `https://` address in Settings → General → Public address (the session cookie gets `Secure` from it).
