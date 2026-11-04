# Integrations

The integrations hub (Integrations → Integrations, route `/integrations/integrations`, permission `settings.manage`) brings together the providers that connect the organization to other services. Each provider has at most one instance in the installation, with its own configuration, status and a short error history.

## How it works

| Piece | Where |
| --- | --- |
| Provider contract (`IntegrationProvider`) and registry (`IntegrationRegistry`) (kernel) | `back/kernel/src/application/integrations/provider.rs` |
| Configuration schema and validation, model (`Integration`, `IntegrationStatus`, `secret_key`) and the organization's mailer resolver (kernel) | `back/kernel/src/application/integrations/{schema,model,mailer}.rs`; SMTP provider in `back/kernel/src/infrastructure/integrations/smtp.rs` |
| Use cases (list, configure, test, activate, deactivate, remove) | `back/modules/integrations/src/service.rs` (`integrations` module, required because the kernel's mailer reads the `smtp` integration) |
| Built-in registry `IntegrationRegistry::builtin()` and MySQL repository | `back/modules/integrations/{registry,store,ports}.rs` |
| API | `back/modules/integrations/src/http.rs` (`/api/v1/integrations`) |
| Screen | `web/src/features/settings/integrations/`, texts in `web/src/i18n/pt-BR/integrations.json` |

### Instance

Table `integrations` (migration `modules/integrations/migrations/0001_create_integrations.sql`): `provider_key` (unique), `config` (JSON with **non-secret** values only), `status`, `last_error`/`last_error_at`, `recent_errors` (the last 5, with `source` = `test`, `activation` or `delivery`), `last_tested_at`, `last_test_ok`, `created_at`, `updated_at`.

Status:

| Status | Meaning |
| --- | --- |
| `not_configured` | API only: the provider has no instance yet |
| `inactive` | Configured, but turned off |
| `active` | Turned on |
| `error` | Turned on, but the last test or delivery failed. It stays in use; it goes back to `active` on the next successful test or delivery |
| `available` | API only: shortcut provider (with no configuration of its own), such as the generic webhook |

`updated_at` only changes with user actions (configure, activate, deactivate) and is the version used in the `If-Match` of the `PUT`. Tests and delivery failures do not change the version.

### Configuration schema

Each provider declares a list of fields (`ConfigField`): `key`, `type` (`text`, `number`, `boolean`, `select`, `secret`, `email`, `url`), `required`, `options` (for `select`), `default`, `min`/`max` (for `number`). The API returns only keys; the screen translates them through `providers.<provider>.name|description` and `providers.<provider>.fields.<field>.label|help|options.<option>` in `integrations.json` (without a translation, it falls back to the backend's English name or the key itself).

On `PUT /integrations/{provider_key}` the backend validates everything against the schema (unknown fields, required fields, types, ranges, options) and then calls `IntegrationProvider::validate` for specific rules. The non-secret configuration is replaced as a whole.

### Secrets

`secret` fields never go into `config`: they are written to the vault (`SecretVault`) with `SecretKey::new("integration", <instance id>, <field>)`, in the same transaction as the rest. The API only reports `secrets: { "<field>": true|false }`. Sending the field empty (or omitting it) keeps the current value; `clear_secrets: ["<field>"]` deletes an optional secret. Secrets do not appear in events, audit entries or logs (the audit only records `"[redacted]"`). Removing the integration deletes the instance and all its secrets (`remove_owner`). Without `APP_ENCRYPTION_KEY`, saving a secret responds 503 `configuration_error`.

### Actions and events

| Route | Effect | Internal event |
| --- | --- | --- |
| `GET /integrations` | All providers in the registry + instance state | — |
| `GET /integrations/{key}` | One provider | — |
| `PUT /integrations/{key}` | Saves the configuration (optional `If-Match`) | `integration.configured` |
| `POST /integrations/{key}/test` | Tests with the saved configuration and stores the result; `send_test_email: true` also sends a test email to the current user (SMTP) | `integration.tested` |
| `POST /integrations/{key}/activate` | Requires a complete configuration; providers with `requires_successful_test_to_activate` (SMTP) test first and refuse with `integration_test_failed` | `integration.activated` |
| `POST /integrations/{key}/deactivate` | Turns off, keeping the configuration | `integration.deactivated` |
| `DELETE /integrations/{key}` | Deletes configuration and secrets | `integration.removed` |

Every mutation writes an event and an audit entry in the same transaction (record `integration`). The events are internal (they are not in the public catalog). Error codes: `integration_provider_not_found`, `integration_not_configurable`, `integration_not_configured`, `integration_test_failed`, `integration_activation_failed` (see [`api/errors.md`](api/errors.md)).

## Outgoing email (`smtp`)

It is **the** sending server for all system emails; there is no SMTP in `.env`. It is configured in **Settings → Outgoing email** (`/settings/outgoing-email`, the same integration shown here) or in the installer's optional step. In a development build the form comes prefilled with Mailpit (`127.0.0.1:1025`, no TLS). See [Configuration](configuration.md#organization-settings).

Fields: `host` (required), `port` (required, default 587), `security` (`none`, `starttls`, `tls`; default `starttls`), `username`, `password` (secret), `from_email` (required) and `from_name`.

- **Test:** opens the connection, negotiates TLS and authenticates (lettre's `test_connection`, 15 s timeout); with `send_test_email`, sends a test email to the user who asked.
- **Activation:** runs the test first; if it fails, the activation is refused and the error goes into `recent_errors`.
- **Resolution on every send:** the `SmtpMailerProvider` (`kernel/infrastructure/mail/provider.rs`) is the `Mailer` of `serve` and the worker. On every send it reads the `smtp` instance with status `active` or `error`, reveals the password from the vault and builds an `SmtpMailer`, cached until the instance changes (`updated_at`/status), so a change in the UI takes effect without restarting. Without an active integration, sending fails with `mail_not_configured` (recorded on the job, without bringing anything down); invitations and password resets done by an administrator return the link to copy. With `DEMO_MODE`, emails are discarded.
- **Where it applies:** all system emails: account verification, password reset, invitations, notifications (`notification.email` job), scheduled reports and booking.
- **Delivery failures:** the error goes into `recent_errors` (`source: delivery`) and the integration moves to `error`, but keeps being used; the notification job is retried by the queue. The first successful send after that sets the status back to `active`.

Caution: the Aphel server opens connections to the host and port configured in the UI. If that is a risk, restrict the server's outbound network access.

## Generic webhook (`webhook`)

Shortcut provider, with no configuration or activation of its own: the card leads to `/integrations/webhooks`, where outgoing webhook endpoints are managed. In the API it shows up with `kind: "shortcut"`, `path: "/integrations/webhooks"` and `status: "available"`; configuring, testing, activating or removing it responds `integration_not_configurable`.

## How to add a provider

1. Implement `IntegrationProvider` (via `async_trait`): a stable `key()` (it becomes the `provider_key` and the i18n key), `name()`/`description()` in English (fallback), `icon()` (the name of an icon known to the screen; today `mail`, `webhook`, and anything else falls back to a generic icon), `config_schema()`, `test_connection()` and, if needed, `validate()`, `requires_successful_test_to_activate()`, `on_activate()`/`on_deactivate()`. Providers without configuration use `kind() -> ProviderKind::Shortcut { path }`.
   - A pure provider (with no infrastructure dependency) can live in `kernel/application/integrations`; one that uses an HTTP client, SMTP, etc. lives in `kernel/infrastructure/integrations`.
   - `test_connection` receives the resolved configuration (`ResolvedConfig`: values + revealed secrets) and returns `Err(message)` on failure; the message is stored and shown to the user, so never include secrets in it.
2. Register it in `IntegrationRegistry::builtin()` (`modules/integrations/registry.rs`).
3. Add the texts in `web/src/i18n/pt-BR/integrations.json` (`providers.<key>`), and the icon in `ICONS` of `IntegrationCard.tsx` if it is new.
4. To use the integration elsewhere in the system, read the active instance through a dedicated port (like `OrganizationMailerResolver`) instead of coupling the consuming use case to the `integrations` table.
5. Regenerate the contract (`make api-types`) if you change DTOs.
