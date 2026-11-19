---
title: Serving many companies
description: Run one Aphel process for many companies, each with its own database, users, modules and files, managed from the operator console.
sidebar:
  order: 14
---

An Aphel installation serves one company by default. The same binary can also serve many companies (tenants) from one process: each tenant keeps its own MySQL database, users, modules, plugins and files, and is reached through its own host names. Nothing in the data model changes; there is still no organization column in any table.

## Modes

| Mode | Configuration | Behaviour |
| --- | --- | --- |
| Single company | `DATABASE_URL` | The installation serves the database in `DATABASE_URL`. This is the default and nothing on this page applies. |
| Many companies | `CONTROL_DATABASE_URL` | Tenants come from a control database. Requests are routed by host name. The operator console answers on `CONSOLE_HOST`. |

`DATABASE_URL` and `CONTROL_DATABASE_URL` cannot be set together. Every tenant database lives on the MySQL server of `CONTROL_DATABASE_URL`, with the same credentials and the name `<control database>_<slug>`; the credentials need `CREATE` and `DROP` privileges.

```bash
CONTROL_DATABASE_URL=mysql://aphel:secret@db:3306/aphel_control
CONSOLE_HOST=console.example.com
# TENANT_DATABASE_MAX_CONNECTIONS=10
# TENANT_EXPORT_DIR=storage/tenant-exports
```

Point the DNS of every tenant host and of the console host to the server. The reverse proxy must keep the original `Host` header.

### Automatic HTTPS for tenant domains

Configure the proxy once for the installation, using `docker/Caddyfile.multi` as the Caddy template. Set `CONSOLE_HOST` in Caddy's service environment to the console domain. Keep Aphel bound to the loopback interface (`127.0.0.1:3001`), and expose ports 80 and 443 for certificate validation. If the VPS serves other applications, keep their existing explicit site blocks alongside the catch-all site.

```caddyfile
{
    on_demand_tls {
        ask http://127.0.0.1:3001/_aphel/tenancy/tls/authorize
    }
}

console.example.com {
    reverse_proxy 127.0.0.1:3001
}

https:// {
    tls {
        on_demand
    }
    reverse_proxy 127.0.0.1:3001
}
```

After that one-time proxy setup, operators only register a host on a tenant and point its DNS to the VPS. Caddy obtains HTTPS automatically during the first connection and renews the certificate. That first connection may take a few seconds. A wildcard DNS record can route many subdomains to the VPS; each host must still be registered to a tenant. This uses individual certificates and does not require a wildcard certificate or DNS provider credentials.

The authorization callback uses the same in-memory directory as host routing. Only exact hosts belonging to active tenants are allowed. Unknown hosts, suspended tenants and tenants being deleted are denied. Missing parameters return 400; authorization returns 204 or 403, with no tenant data. The callback works with a loopback Host header and never falls through to a tenant or the console login. Adding or removing hosts refreshes the directory; no proxy reload or privileged application access is needed. Certificates already cached by the proxy can remain valid after suspension or host removal, but Aphel still refuses application access according to its current directory.

Without this proxy integration, merely pointing DNS at the VPS is insufficient. Do not enable unrestricted on-demand certificate issuance.

## First steps

1. Start `aphel serve` and `aphel worker` (or `aphel serve --with-worker`) with `CONTROL_DATABASE_URL`. They create the control tables on boot when `RUN_MIGRATIONS` is on.
2. Create the first operator: `aphel console create-operator --email ops@example.com`. The password is asked twice without echo.
3. Open `https://<CONSOLE_HOST>` and sign in. Create the companies there, or with `aphel tenants create`.
4. Run `aphel tenancy-check` once on the new installation (see [Isolation check](#isolation-check)).

## How requests are routed

| Host of the request | Answer |
| --- | --- |
| `CONSOLE_HOST` | The operator console (`/api/console/v1` and its screens) |
| A host of an active tenant | That tenant's Aphel, exactly as a single-company installation |
| A host of a suspended, provisioning or deleting tenant | `503 tenant_unavailable` (a plain page outside `/api`) |
| Any other host | `404 tenant_not_found` |

Host names are compared in lowercase, without port. A tenant can have several hosts; the primary one is used for its public address when it is created, and the public address stays editable in the tenant's own settings.

## The console

The console is served only on `CONSOLE_HOST`, with its own cookie (`aphel_console_session`); tenant sessions never reach it and console sessions never reach a tenant.

- **Companies**: list with state, hosts, database size and last write; create a configured company (name, identifier, host, first administrator, template and modules) or a completely blank instance (host only, with an automatically generated internal identifier); suspend and resume; add and remove hosts and choose the primary one; health (queued and failed jobs, workers, pending database updates, plugins with errors, recent failures); delete, optionally writing a full copy into `TENANT_EXPORT_DIR` first.
- **Plugins and modules**: on a company's page, the *Plugins and modules screens* switch turns the company's own plugin and module management on or off. When off, its administrators do not see the Plugins and Modules screens and the plugin and module install, enable, disable, settings, export and purge calls answer `403 plugin_management_disabled`; plugins already enabled keep running. Operators can always manage the company's plugins from the console (install an `.aphelplugin` after reviewing what it asks for, enable, disable, configure, remove with or without its data) and turn its modules on or off. Plugins installed from the console act on behalf of the company's first administrator.
- **Operators**: invite with a link valid for 7 days (shown once, there is no email in the console), issue a new link, deactivate. Operators change their own password from the account menu.
- **History**: every action of the console and of the tenant commands, with operator and IP.

Sign-in uses Argon2 passwords, locks an operator for 15 minutes after 5 wrong passwords and accepts 10 attempts per minute per address.

A completely blank instance creates only its isolated database and the platform schema required to serve the installer. The console uses its host as the initial label. No organization, administrator, optional modules or blueprint are installed. Open its host to complete the normal installer before using the company. Whoever completes that installer creates the first administrator, so share the address only with the person responsible for setup until installation is complete.

The console creation API uses `mode: "blank"` with `host`, or `mode: "configured"` with all company and administrator fields.

## Command line

| Command | What it does |
| --- | --- |
| `aphel tenants list` | Tenants with state, database, hosts, last migration and last error |
| `aphel tenants create ...` | Create, migrate, install and activate a tenant (see [CLI](/_aphel/docs/admin/cli/#tenants)) |
| `aphel tenants suspend <slug>` / `resume <slug>` | Stop or restart serving a tenant and running its jobs |
| `aphel tenants delete <slug> --confirm <slug> --export <zip>` | Export, then delete the files, database and registration |
| `aphel migrate` | Migrate the control database and every active or suspended tenant |
| `aphel <command> --tenant <slug>` | Run `doctor`, `modules`, `mounts`, `secrets rotate`, `search` or `org` on one tenant |
| `aphel console create-operator --email ...` | Create a console operator |
| `aphel tenancy-check` | Run the isolation check |

Commands that act on one company refuse to run without `--tenant`. `db reset` and `upgrade` only work on a single-company installation.

## Jobs and migrations

One worker process runs the jobs and schedules of every active tenant. Each tenant keeps its own queue, schedules and events in its database and has its own polling loop; `WORKER_CONCURRENCY` is shared by the whole process and handed out in turn to the tenants with pending jobs, so a busy tenant does not hold every slot. The worker picks up new and resumed tenants within a few seconds and stops the loops of suspended ones after their running jobs finish.

On boot (`RUN_MIGRATIONS`) and with `aphel migrate`, the control database is migrated first, then every active or suspended tenant one at a time. A tenant that fails keeps the error (`aphel tenants list`, console) and the others go on.

## Files

Stored files of a tenant live under `<slug>/` in the storage that tenant configured (local disk or S3-compatible bucket), so tenants sharing a disk or a bucket never share a path. Deleting a tenant removes everything under its prefix.

## Isolation check

`aphel tenancy-check` creates two throwaway tenants on the control server, signs in to each through the real host routing and fails when one of them reaches the other's database, stored files, cached state, enabled modules, sessions, records or events, when a suspended tenant still answers, or when state answers outside a tenant. Both tenants are deleted at the end, also when a check fails. Run it after installing and after upgrading.

```text
ok   database per tenant: aphel_control_isocheck_1a2b3c4d_a and aphel_control_isocheck_1a2b3c4d_b
ok   sessions: a session of A is refused on B
...
Tenants are isolated.
```
