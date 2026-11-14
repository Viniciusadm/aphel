---
title: Architecture
description: How Aphel is built, for people who operate or extend it, from crates and layers to the request path, the record pipeline, events, jobs and security.
sidebar:
  order: 4
---

This page describes how Aphel is built. It assumes you have read [Concepts](/_aphel/docs/start/concepts/). You do not need it to use Aphel, but it helps when you operate an installation, debug a failure or design a plugin.

## Overview

Aphel is a **modular monolith**. One Rust binary contains the HTTP server, the worker, the command line and the embedded web application. There are no microservices. The same binary starts in different roles:

| Command | Role |
| --- | --- |
| `aphel serve` | HTTP server: API and web application. This is the default when no subcommand is given. |
| `aphel worker` | Job worker and scheduler. |
| `aphel serve --with-worker` | Both in one process, for simple installations. |
| `aphel migrate` | Apply pending database migrations and exit. |

```text
  browser / API client
          |
          v
  +----------------+        +----------------+
  | aphel serve    |        | aphel worker   |
  | HTTP + web app |        | jobs+scheduler |
  +-------+--------+        +-------+--------+
          |                         |
          +-----------+-------------+
                      v
        +----------------------------+     +-------------------+
        | MySQL 8                    |     | file storage      |
        | records, outbox, job queue |     | disk or S3        |
        +----------------------------+     +-------------------+
```

The two processes share nothing except the database and file storage. You can run several of each. The state that must be consistent between processes (the job queue, the outbox, rate limit counters, the enabled module set, the entity registry version) lives in MySQL.

## Source layout

| Crate or folder | What it is |
| --- | --- |
| `back/kernel` (`aphel-kernel`) | The platform: HTTP and security, identity and access, the entity, query and pipeline engine, events and audit, jobs, files, search plumbing, the installer, generic settings and the module system. It also contains the core migrations. |
| `back/contract` (`aphel-contract`) | Contract primitives with no behavior and no dependencies: operation, event, hook and extension-point keys, reference kinds and the catalog. |
| `back/contracts/<name>` | Contract packages. A contract describes what a capability offers (for example `crm`, `deals`, `tasks`) so that modules and plugins depend on contracts and never on each other. |
| `back/modules/<key>` (`aphel-mod-<key>`) | Aphel's modules. Each has its code and its migrations. |
| `back/plugins/<key>` | Examples of compiled modules kept outside the default build. |
| `back/sdk` (`aphel-sdk`) | The curated API for native modules. New module code imports from it instead of the kernel. |
| `back/app` (`aphel`) | The binary: command line, the generated module registry and the embedded web application. |
| `back/blueprint` (`aphel-blueprint`) | Reads and resolves blueprint files. |
| `back/xtask` | Maintenance tasks: generating the module registries and design token files, and the architecture checks. |
| `blueprints/<key>` | Blueprint definitions and their configuration packages. |
| `web/` | The React and TypeScript application. |
| `sdk/` | SDKs and tools for runtime plugins: the Rust and JavaScript plugin SDKs, the `aphel-plugin` command line and the `@aphel/ui` package. |

Which modules go into a build is decided by `back/aphel.modules.toml`. The `modules sync` task generates the module dependencies of the binary, the module list in `back/app/src/modules.rs` and the web module list, and the generated files are committed. A build with only the kernel is possible.

## Layering

Backend code follows a hexagonal (ports and adapters) layout:

```text
  api  ---------->  application  -------->  domain
  (axum routes,     (use cases and          (entities, value
   extractors)       ports = traits)         objects, rules)
                          ^
                          |
                  infrastructure
                  (MySQL, SMTP, object storage,
                   Argon2, adapters of the ports)
```

- The **domain** is pure: no I/O.
- **Use cases** in the application layer reach the outside only through traits (ports). They receive an access scope and check permissions explicitly.
- **Infrastructure** implements the ports with sqlx and MySQL, lettre, the `object_store` crate and so on.
- The **API layer** extracts the request, calls a use case and renders the response envelope.

Modules follow the same direction of dependencies. Every route has an OpenAPI annotation, and the document is served at `/api/openapi.json`.

## The request path

```text
  request
    |
    v  CORS
    v  API token rate limit        (only when Authorization: Bearer is present)
    v  request policies            (contributed by modules, may reject a method and path)
    v  CSRF header check           (non-GET without X-Requested-With: aphel)
    v  idempotency                 (POST with an Idempotency-Key header)
    v  payload size limit
    v  route handler
         |  AuthContext extractor: session or API token -> user, role, teams, permissions
         v
       use case
         |  scope.require(permission) or scope.record_filter(permission)
         v
       begin transaction
         |  record pipeline (hooks) + write
         |  publish domain event  -> outbox
         |  record audit entry
       commit
         v
       response: { "data": ... } or { "error": { "code", "message", "details" } }
```

Key points:

- **Authentication.** The `AuthContext` extractor resolves the session cookie (`aphel_session`) or the bearer token into an access scope. It rejects invited or deactivated users and sessions that are still waiting for the second factor. Routes about the account itself accept sessions only, not tokens.
- **Authorization.** Handlers do not check permissions. The use case calls `scope.require(...)` for an administrative permission, or `scope.record_filter(...)` to obtain the own, team or organization filter that is added to the query. There is no `Require<P>` extractor.
- **Transaction.** Every business mutation opens a transaction, writes, publishes a domain event into the outbox and records an audit entry in that same transaction. Either all of it happens or none of it.
- **Errors.** Everything goes through one error type and comes out in the same envelope, with a stable `code`. Internal errors are logged and answered with a generic message.
- **Versioning.** Updates can send `If-Match`. The version is checked again inside the transaction after locking the row, and a mismatch answers 409 `version_conflict`.

Aphel's own infrastructure lives under a few fixed prefixes: `/api` for the REST API and `/_aphel` for the built web assets, the shared plugin modules, the service worker, health checks, metrics and the static documentation (`/_aphel/docs`). If the web application has been built into the binary, the internal app is served at its base path, `/` by default or a single segment such as `/app` (see [Configuration](/_aphel/docs/admin/configuration/)), and it always answers at `/_aphel/app/` too. With the app outside `/`, the other paths are left free and `/` redirects to the app.

## The record pipeline

All record writes go through one pipeline, whichever way they arrive: the generic API, a module's own API, an import, a bulk action, an automation or a plugin. Hooks are how modules and plugins join it.

| Stage | Hook | Kind | What a hook can do |
| --- | --- | --- | --- |
| 1. Authorization | none | | The use case checks the permission and the record scope. |
| 2. Saving | `RecordSaving` | filter | Change the values and custom fields before they are written. |
| 3. Write | none | | The kernel writes the fixed columns and the custom field values. |
| 4. Validate | `RecordValidating` | veto | Return an error to reject the write. The transaction rolls back. |
| 5. Saved | `RecordSaved` | action | React inside the same transaction, after the write. |
| 6. Event and audit | none | | The domain event goes to the outbox and the audit entry is recorded. |
| 7. Commit | none | | The transaction commits. |
| 8. Committed | `RecordCommitted` | action | React after the commit. It runs in its own transaction, and a failure is logged without failing the request. |

Other record hooks:

| Hook | Kind | When |
| --- | --- | --- |
| `RecordArchiving` | veto | Before a record is archived. |
| `RecordDeleting` | veto | Before a record is deleted. Used by relationships with `restrict`. |
| `RecordDeleted` | action | After a record is deleted, in the same transaction. |
| `RecordRestored` | action | After a record is restored from the trash, after validation and before `RecordSaved`. |
| `RecordPurged` | action | After a record is permanently deleted, before the kernel removes its custom values, links, attachments and shares. |
| `RecordQueryFilter` | filter | Adds `AND` conditions and sorting to every query of a record type. It can only narrow results. |
| `RecordSerializing` | filter | Changes records on their way out of the API, after field permissions. |
| `RecordSchemaFilter` | filter | Changes the field schema returned to forms, lists and filters. |
| `SearchPlanFilter` | filter | Adds restrictions to global search for one record type. |

Rules that follow from this design:

- Hooks run inside the request. Keep them fast and do not call slow external services from them. Do external work from an event or a job.
- Hooks in a transaction cannot start a second write path. Runtime plugins that try to write records from an in-transaction hook receive 409 `plugin_write_in_hook`. They write from events instead.
- Listings are built only by the query engine from a filter tree validated against the entity registry. User text is never turned into SQL.
- Custom field values are written only by the custom field writer, so validation, layout rules and uniqueness apply everywhere.

The complete list of hooks, including domain hooks of modules, is in the [Hooks catalog](/_aphel/docs/reference/hooks-catalog/). Plugin authors should read [Hooks](/_aphel/docs/plugins/hooks/).

## Events, the outbox and delivery guarantees

```text
  use case ---> events().publish(tx, event)  ---> domain_events table (dispatched_at = NULL)
                                                        |
                    every 5 seconds, worker             v
                  outbox.dispatch job  ---> one event.consume job per interested consumer
                                                        |
                                                        v
                                           consumer.consume(event)  (own retries)
```

- A domain event carries an id, the event type, the actor, an optional record reference, a JSON payload, causation information and a timestamp. Actors are a user, the system, an API token, an automation, a plugin or a named service.
- The `outbox.dispatch` job runs every 5 seconds. It locks undispatched events with `FOR UPDATE SKIP LOCKED`, reads up to 100 per batch, creates one `event.consume` job per consumer that accepts the event type, and marks the event as dispatched in the same transaction. If a batch is full, it schedules a follow-up.
- A consumer has a stable unique name and declares which event types it accepts. Consumers include notifications, the search index, automations, webhooks and plugins. A plugin never receives events it caused.

**What you can rely on:**

| Guarantee | Detail |
| --- | --- |
| Atomic publication | An event exists if and only if the change that caused it committed. |
| No lost events | Undispatched events stay in the table until a worker dispatches them. |
| At-least-once delivery | A consumer can see the same event more than once, for example after a crash. Consumers must be idempotent. |
| Isolated failures | Each consumer has its own job and retries independently. A failing consumer does not block the others. |
| Deduplicated scheduling | The consumer job has the unique key `event:<event id>:<consumer>`, so it is created once. |
| No global ordering | Events are dispatched roughly in order of occurrence, but consumers run in parallel across workers. |
| Latency | Consumers react within seconds, not milliseconds, and only while a worker is running. |

## Jobs, the worker and the scheduler

Jobs live in a MySQL table. Workers claim them with `FOR UPDATE SKIP LOCKED`, so any number of workers can share a queue.

| Setting | Default | Where |
| --- | --- | --- |
| Attempts per job | 5 | Per job, set when it is enqueued. |
| Retry delay | 10 seconds, doubling each attempt, at most 1 hour | Built in. |
| Job timeout | 300 seconds | `WORKER_JOB_TIMEOUT_SECS` |
| Parallel jobs per worker | 4 | `WORKER_CONCURRENCY` |
| Queue polling | 1000 ms | `WORKER_POLL_INTERVAL_MS` |
| Scheduler tick | 5 seconds | `SCHEDULER_INTERVAL_SECS` |

- A job can be enqueued inside the transaction that needs it, so it exists only if the change committed.
- A job can carry a unique key. Enqueuing a second job with a key that is still active is ignored.
- If a worker dies while holding a job, the job is released after the lock expires (the job timeout plus a 60 second grace period) and tried again. If it has no attempts left, it is marked as failed.
- Failed jobs stay visible to administrators with `system.manage`, who can retry or delete them.
- The scheduler creates recurring jobs from cron expressions with a seconds field. Modules register them in code, and runtime plugins declare them in the manifest.

See [Operations](/_aphel/docs/admin/operations/).

## File storage

Files go through one storage abstraction on top of the `object_store` crate. The default is local disk at `./storage` (a volume in the Docker image). Administrators can switch to S3-compatible storage in **Settings**, under **File storage**, and test it before saving. The secret access key goes to the secret vault. Files are stored with keys of the form `<year>/<month>/<id>` and are never served directly: downloads always go through an authenticated endpoint that checks access to the record the file is attached to. Attachments reference records through a record reference. The upload limit is an organization setting and defaults to 25 MB.

## Email

Outgoing email uses SMTP through `lettre`. The SMTP settings are organization configuration, edited under **Settings**, **System email**, with the password in the secret vault. `SMTP_*` environment variables act as a fallback. The mailer is resolved on every send, so changes apply without a restart. Without SMTP, sending fails with `mail_not_configured` without bringing anything down, and screens that would send an invitation or password reset show the link instead. Notification emails are sent by a job. Links in emails always use the saved public address, never the `Host` header of a request.

The email module can also connect personal mailboxes over IMAP and SMTP. See [Email and sequences](/_aphel/docs/using/email-and-sequences/).

## Search

Global search is generic over the entity registry. A record type is picked up automatically if it publishes events for its records and marks fields as searchable. The index lives in MySQL tables, including a full-text index with the `ngram` parser, and is kept current by an event consumer. `aphel search reindex` rebuilds it, optionally for one record type. See [Search and collaboration](/_aphel/docs/using/search-and-collaboration/).

## Caching and the entity registry

The entity registry describes every record type and field. It is built from two sources: record types registered in code by modules, and custom objects and fields stored in the database, which include those created by administrators and the managed entities declared by runtime plugins.

- Each process keeps the registry in memory.
- Whenever an administrator changes objects, fields or relationships, a version counter in the organization settings is incremented.
- A process compares that counter with its cached copy at most once per second and rebuilds the registry when it differs. This is how a change made through one process reaches all the others, including the worker, which resolves the current registry on each job.
- The set of enabled modules and the organization settings are cached the same way and refreshed every 5 seconds.
- Runtime plugin state is rechecked at most every 2 seconds in every process.

You therefore never need to restart anything after changing fields, modules, plugins or settings, but expect up to a few seconds before every process agrees.

## How modules and plugins register contributions

Every module implements one trait: a manifest plus a `register` function that receives an application builder. The builder is the only contribution point. Through it a module registers:

- routes (authenticated, public and root-level);
- hooks (`on` for actions and vetoes, `filter` for filters);
- services, and decorators that wrap a service;
- record types and extensions of record types, permissions and their default grants;
- public events, notification kinds, OpenAPI tags and dashboard widgets;
- automation triggers and actions;
- jobs, event consumers, schedules and workers;
- typed contributions that other modules define, such as timeline renderers or configuration sections.

When the application starts, the builder sorts modules by their dependencies and freezes the result. Each contribution remembers its module key and is ignored while that module is disabled. This includes routes, hooks, system fields, services and jobs, so disabling a module takes effect without a restart. Modules declare migrations in their manifest. They are applied in load order and tracked per module.

Modules extend each other without importing each other: through a hook, a service, or a typed contribution defined in a contract. Cycles between crates are forbidden.

Runtime plugins reach the same extension points through a narrower door. Their manifest declares entities, fields, hooks, events, routes, schedules, settings, automation actions, widgets and more, and the host registers them on the plugin's behalf. Modules can expose operations and forward domain hooks to plugins. See [Plugins overview](/_aphel/docs/plugins/overview/) and [Host API](/_aphel/docs/plugins/host-api/).

### The runtime plugin host

- Plugins run as WebAssembly components in `wasmtime`, in an empty sandbox with no file or network access of their own.
- Each call has a time limit: 2 seconds for hooks, 5 for widgets, 10 for routes, 30 for events and actions, and 60 for schedules. Each instance is limited to 64 MB of memory.
- Compiled code is cached by the SHA-256 of the backend file, in memory and in the database, so other processes and restarts do not recompile.
- Every call runs with the caller's permissions intersected with the capabilities the plugin declared.
- If a plugin backend stops loading, it fails closed: record types it hooks answer 503 `plugin_failed`.
- A plugin accesses data only through host calls (records, key-value storage, settings, secrets, outbound HTTP, the clock and logging), never through SQL.

See [Lifecycle](/_aphel/docs/plugins/lifecycle/) and [Security model](/_aphel/docs/plugins/security-model/).

## How the web application is assembled

The web application is a React 19 single-page application, built with Vite and embedded into the binary. It is assembled from web modules in the same way the backend is assembled from backend modules.

- Each web module registers its routes, navigation entries, settings pages, record tabs, widgets, commands, field input types and renderers with a central registry. There are no central lists: the router, sidebar, command palette, keyboard shortcuts and settings read from the registry.
- Extension is by slots (named places in a screen where other modules add content), filters (functions that rewrite a list such as table columns or navigation items) and overrides (replace a component).
- The list of web modules is generated from the same module file as the backend, so a module is either in both builds or in neither.
- At runtime, everything is filtered by the set of enabled modules from `/api/v1/modules`, so disabled modules disappear from navigation and their routes are blocked.
- Screens use generic record building blocks (list, form, detail, timeline), so a new record type gets complete screens from its registry description.
- API calls go through one client with types generated from the OpenAPI document.
- Texts come from i18n catalogs in Brazilian Portuguese and English. Visual values come from design tokens, which are also what plugins use through the UI library. See [Design tokens](/_aphel/docs/reference/design-tokens/).

Runtime plugins add screens too. Before the router is built, the application loads the web bundles of enabled plugins. Plugin bundles import React, the router, TanStack Query and the Aphel web SDK from shared entries served by the host, through an import map, so there is exactly one copy of each. See [Web bundle](/_aphel/docs/plugins/web-bundle/) and [UI library](/_aphel/docs/plugins/ui-library/).

## Real-time, PWA and offline

- **Updates.** The interface mostly refreshes by polling, for example the unread notification count and the progress of exports and bulk actions. Plugins can subscribe to a server-sent events stream of changes to their own data.
- **Push.** Browsers that support it can receive web push notifications through the service worker. This is configured by each user.
- **Installable app.** Aphel ships a web manifest and a service worker, so supported browsers can install it as an app. The service worker caches the application shell.
- **Offline.** When the network is unavailable, creating certain items, such as notes, activities, contacts and calls, is queued on the device in IndexedDB and sent when the connection returns. Each queued request carries an `Idempotency-Key`, so a retry never creates a duplicate. Items the server rejects are shown for the user to fix, retry or discard.

## Security model overview

| Area | How it works |
| --- | --- |
| Passwords | Hashed with Argon2. Between 8 and 128 characters. |
| Sessions | An opaque random token in the `aphel_session` cookie (`HttpOnly`, `SameSite=Lax`, `Path=/`, plus `Secure` when the public address is HTTPS). Only a hash of the token is stored, so sessions can be revoked. Sessions last 30 days. Users can list and end their sessions, and a password reset revokes all of them. |
| CSRF | Every request that changes data must send `X-Requested-With: aphel`, unless it uses a bearer token or targets a public route (`/api/v1/public/*`, `/api/v1/hooks/*`). Otherwise it is refused with 403 `csrf_header_missing`. |
| API tokens | Format `aphel_<prefix>_<secret>`. Only a hash is kept and the secret is shown once. Permissions are the token's scopes intersected with the creator's current permissions. Tokens are rate limited per token, by default 600 requests per 60 seconds, configurable by administrators. Responses carry `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset`. |
| Two-factor authentication | TOTP with recovery codes, for sessions only. After a correct password, a user with 2FA gets a partial session that is valid for 10 minutes and accepts only the challenge. Five failed attempts revoke it. An organization policy of `none`, `admins` or `everyone` can require it, and users without it are blocked with 403 `two_factor_setup_required`. |
| Rate limits | Sign-in: 10 attempts per 5 minutes. Password reset: 5 per 15 minutes. Two-factor challenge: 10 per 15 minutes. Single sign-on: 30 per 5 minutes. Password change: 5 per 15 minutes. Counters are shared through the database, so they hold across processes. Exceeding a limit answers 429 `rate_limited`, and repeated sign-in failures can answer 429 `login_locked`. |
| Secrets | Passwords of integrations, signing secrets and keys are encrypted with AES-256-GCM using `APP_ENCRYPTION_KEY`, and never returned by the API, written to logs or recorded in the audit log. Keys can be rotated with `aphel secrets rotate`. |
| Audit | Every business mutation writes an audit entry with a diff, with sensitive fields replaced by `[redacted]`. Account security events (sign-in, 2FA, password, sessions, tokens, permissions) are recorded separately. |
| Outbound requests | Webhooks and plugin HTTP calls apply SSRF protection and do not reach private networks unless `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` is set. |
| Privacy | Fields can be marked personal or sensitive. A privacy module locates data subjects, handles requests (export, anonymize, delete) and applies retention policies. |

See [Security](/_aphel/docs/admin/security/) for the administrator's view, and [Authentication](/_aphel/docs/reference/authentication/) and [Errors](/_aphel/docs/reference/errors/) for the API contract.

## Health and observability

- `GET /_aphel/health` answers `ok` when the process is up.
- `GET /_aphel/health/ready` checks the database, storage, worker heartbeat, scheduler and queue delay, and answers 503 if one fails. `aphel healthcheck` exits 0 when the local server is healthy. With `--ready` it checks `/_aphel/health/ready`, and with `--worker` it checks the heartbeat of the worker on the same host. The Docker images use it for their health checks.
- `GET /_aphel/metrics` serves Prometheus metrics only when `METRICS_TOKEN` is set, and requires it as a bearer token.
- `aphel doctor` prints a diagnosis without secrets that is safe to attach to an issue.
- Logs go to standard error and follow `RUST_LOG`.
