---
title: Security model
description: How plugin backends are sandboxed, who they act as in each entry point, what a web bundle can do and what an administrator consents to.
sidebar:
  order: 16
---

Installing a plugin means running third-party code inside your Aphel. The security model has two halves with very different guarantees: the **backend** (`backend.wasm`) runs in a strict sandbox with limited authority; the **web bundle** runs in the browser with no isolation at all. Understand both before installing or publishing a plugin.

## The backend sandbox

Backends are WebAssembly components executed by Wasmtime.

| Property | Value |
| --- | --- |
| Instance lifetime | A fresh instance for every call. Nothing survives between calls, so one call can never read another call's memory. |
| WASI | Present with an empty configuration: no preopened directories, no environment variables, no arguments, no sockets (TCP and UDP disabled), no DNS lookups. |
| Memory | 64 MB per instance. Allocating more traps the instance. |
| Time | Epoch-based limits: hook 2 s, route 10 s, event 30 s, schedule 60 s, automation action 30 s, operation and contribution 30 s, widget 5 s, privacy request 60 s. The limit is enforced by interrupting the running code. |
| Host access | Only the functions of the `host` interface, described in [Host API](/docs/plugins/host-api/). A plugin cannot run SQL, read files, spawn processes or open connections. |
| Failure | A trap, a timeout or an out-of-memory condition fails the call with `503 plugin_failed`. The detail goes to the server log, not to the caller. Host errors of kind internal are reported as `internal error`. |
| Code integrity | The component is checked when installed (`plugin_backend_invalid`) and compiled code is cached by the SHA-256 of `backend.wasm`. |

The sandbox restricts what code can do, not what it can see through the host API. Everything a plugin reads or writes goes through the host API under the permissions described below.

## Authority: actor ∩ capabilities

A plugin declares **capabilities**: the permissions in `requires`, `optional_requires`, its `[[permissions]]` and the permissions of its own entities. Every call runs with the intersection of those capabilities and the permissions of the party the call acts for. The record scope (`own`, `team`, `organization`) of the actor's permission is preserved. Field permissions apply too: values the actor cannot see are removed before the plugin gets them.

Who the party is depends on the entry point.

| Entry point | Acts as | Notes |
| --- | --- | --- |
| Hook, caused by a signed-in user or API token | The caller | Writes are refused in hooks anyway; reads see what the caller can see. |
| Hook, caused by a background process without a user | The plugin installer | |
| Private route | The caller | The route's `permission` is checked against the caller's real permissions first. Records written are attributed to the caller. |
| Public route | The plugin installer, as the plugin | The actor kind is `anonymous` in the request, but the permissions are the installer's, limited to capabilities. |
| Event, schedule, privacy request | The plugin installer, as the plugin | `503 plugin_owner_inactive` when the installer is no longer an active member. |
| Automation action | The automation (the permissions of the user who published its version) | |
| Widget | The viewer | Read-only. |
| Operation provided by the plugin, contribution | The caller of the operation, after the operation's own `permission` is checked | |
| Deferred effect | The authority captured when the effect was deferred, re-resolved when it runs, limited to capabilities, acting as the plugin | |

"Installer" means the user who last installed the plugin. The plugin can never do more than that user can, and never more than its declared capabilities. Reinstalling the plugin as a different user changes the installer.

Consequences:

- A plugin that does not declare `people.person.read` cannot read people, even if the user is an administrator.
- A plugin whose installer was demoted loses power for events, schedules and public routes at once.
- A plugin acting as the plugin (events, schedules, public routes, deferred effects) appears in events and audit logs as the actor `plugin` with the plugin key.
- Executions carry a chain of plugin keys. A plugin never receives events it caused, and a loop through the same plugin is stopped by `plugin_execution_cycle` or by read-only re-entry of hooks.

## What the host API does not protect

Be aware of these properties when you decide whether to trust a plugin:

- Directory functions (`list-users`, `list-roles`, `list-teams`) and `organization-json` do not check any permission. A plugin with a backend can read active users' names, e-mail addresses, roles and teams.
- A plugin with `emails.send` in `requires` can send e-mail through a deferred `email` effect, to any address that is not on the opt-out list, at up to 120 messages per minute.
- A plugin can emit events that webhooks and automations consume, and send notifications to any active user (the recipient must be able to read the linked record).
- A plugin trusted with a secret can send it to any host in its `allowed_hosts` or return it from a route.
- Deferred commands are stored as JSON until they complete; a secret placed in one is stored in clear for that time.

## Outbound HTTP

Backends call the network only with `http-fetch` or a deferred `http` effect.

- The destination host must be listed in the manifest's `[http] allowed_hosts`. The administrator sees the list in the consent dialog. Matching is by host name; `*.example.com` covers subdomains but not the apex domain.
- The request goes through the same outbound policy as webhooks: absolute `http` or `https` URLs, no credentials or fragments in the URL, HTTPS when the installation requires it, and no private or loopback addresses or local names (`localhost`, `.local`, `.internal`) unless the operator explicitly allowed private networks. Addresses are checked when the name is resolved, which protects against DNS tricks.
- Redirects are not followed, so an allowed host cannot send the plugin to a host that is not allowed. Proxies are not used.
- Requests and responses are limited to 5 MiB, connecting to 5 seconds, and the whole request to the time left in the call.

## Secrets

Secrets (`secret` settings) live in the secrets vault, encrypted with `APP_ENCRYPTION_KEY`, scoped to the plugin. They are never returned by the API, and logs and audit entries show `[redacted]`. Only the plugin's own backend can read them, with `secret(key)`. See [Settings and secrets](/docs/plugins/settings-and-secrets/).

## The web bundle has no isolation

A `web/` bundle is JavaScript that the Aphel web app imports into the page. It runs with the same origin, the same session cookie and the same access to the REST API as the signed-in user. Nothing in the runtime limits it: it can read what the user can read, change what the user can change, read the DOM and local storage, and call any endpoint. The permissions of the backend are not a limit for the bundle.

For that reason:

- A package that declares `[web]` can be installed, upgraded or enabled only after an explicit acknowledgement for the exact manifest version (`acknowledge_web_version=<version>`; the interface shows a checkbox). Every new version needs a new acknowledgement. A dry run does not need one.
- Treat web bundles like any other code you run in your browser with your credentials: install only from authors you trust and review the bundle.
- A public web application (`web.public`) is served at `/p/<key>/` with a restrictive content security policy: scripts, fonts, connections and frames are limited to the same origin, and the page cannot be framed. See [Public web apps](/docs/plugins/public-web-apps/).

Plugin web files are served to anyone, even before sign-in: bundles are code, not data. Do not put secrets in them.

## What an administrator consents to

The consent dialog (and the dry-run response) lists everything the plugin will do, so that the decision is informed:

- the record types it creates and extends, and unique fields;
- the permissions it uses (`requires`, `optional_requires`) and creates, and the roles it offers;
- its hooks, consumed events, routes (private and public), schedules, actions, triggers, notifications, widgets and operations;
- the hosts it may call and the secrets it stores;
- the built-in modules it would disable, and the overlaps with other plugins that need acknowledgement;
- whether it ships a web bundle, which needs its own acknowledgement.

Installing requires the `system.manage` permission. The audit log records who installed, enabled, disabled, configured or removed each plugin, with the client address; installs record the version change.

## Guidance for plugin authors

- Declare the least: each `requires` entry, allowed host and public route widens what administrators must trust.
- Authenticate public routes yourself. Compare a shared secret in constant time, validate the body and answer with minimal information.
- Keep secrets in `secret` settings. Never log them or return them.
- Treat record values as untrusted input even when they come from your own entity.
- Prefer deferred effects with stable keys: they make a retry harmless and keep the audit trail attached to the right actor.
- Fail with explicit plugin errors for expected conditions so callers get clear messages instead of `plugin_failed`.
