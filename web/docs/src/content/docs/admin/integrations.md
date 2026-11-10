---
title: Integrations
description: What each integration needs, where to configure it, and where its credentials are stored.
sidebar:
  order: 8
---

Aphel connects to outside services through modules. Each integration is a module you enable, and its credentials are module settings or per-user connections. Settings that hold secrets are encrypted with `APP_ENCRYPTION_KEY`, so set that key before configuring any of them. See [Security](/_aphel/docs/admin/security/).

All of this needs the `settings.manage` permission, except mailboxes, which each user connects for themselves.

| Integration | Module | Configured in | Needs |
| --- | --- | --- | --- |
| Outgoing email (system messages) | kernel | Settings, Outgoing email | An SMTP server |
| Email mailboxes | `email` | My account, Email mailboxes (per user) | IMAP and SMTP credentials, or Google or Microsoft connected |
| Google | `google` | Settings, Modules | An OAuth client in Google Cloud |
| Microsoft | `microsoft` | Settings, Modules | An app registration in Microsoft Entra ID |
| Single sign-on | `sso` | Settings, Modules | An OpenID Connect provider |
| Telephony | `telephony` plus a provider module | Settings, Modules | A telephony provider module |
| Electronic signature | `esign` | Settings, Modules | A Clicksign account |
| AI assistant | `ai` | Settings, Modules | An Anthropic or OpenAI-compatible endpoint |
| CNPJ enrichment | `enrichment` | Settings, Modules | Nothing, it uses public APIs |
| Messaging channels | `messaging` | Messaging, Channels | WhatsApp Cloud API credentials, or an IMAP and SMTP mailbox |
| Outgoing and incoming webhooks | `webhooks` | Settings, Outgoing webhooks and Incoming webhooks | A URL to call or to be called |

The `integrations` module is the integration center: it lists providers, stores their configuration, runs a connection test and activates or deactivates them. The webhook provider is built in, and other modules contribute their own screens.

Note on language: the labels of some module settings are written in Portuguese in the current version, for example "Tipo de atividade das reuniões". The keys and behavior are the same.

## Outgoing email

System messages (invitations, password resets, notifications, reports, security notices) are sent over SMTP.

1. Open Settings, Outgoing email.
2. Fill the SMTP host, port (default 587), connection security (`starttls`, `tls` or `none`), user name, sender address and password. Host and sender are required together.
3. Use Test to check the connection. Tick the option to send a test message to yourself.

The password goes to the secret vault. You can also provide the values with the `SMTP_*` environment variables, which act as a fallback. See [Configuration](/_aphel/docs/admin/configuration/#which-setting-wins). Until it is configured, links for invitations and password resets are shown on screen so you can copy them.

## Email mailboxes

Each user connects their own mailbox to read and send email from records. Open My account, Email mailboxes. The available providers are the core `imap` provider and the providers of enabled modules (Gmail through `google`, Outlook through `microsoft`).

| Provider | What the user provides |
| --- | --- |
| IMAP and SMTP | Email address, IMAP host, port and security, SMTP host, port and security, user name (and an SMTP user name if different) and password, plus an optional sent folder. The server connects to these hosts, so they must be reachable from it. |
| Gmail | Consent on Google's screen (read and send access to Gmail). Requires the Google module to be configured. |
| Outlook | Consent on Microsoft's screen. Requires the Microsoft module to be configured. |

Synchronization runs as a scheduled job on the worker. Tuning variables (`EMAIL_SYNC_*`) are in [Configuration](/_aphel/docs/admin/configuration/). By default the server refuses to connect to private-network addresses. Set `EMAIL_ALLOW_PRIVATE_NETWORKS` to allow an internal mail server. Credentials are never returned by the API, only whether they are configured.

## Google

The `google` module provides sign-in with Google, meeting sync with Google Calendar and Gmail mailboxes. It depends on the `sso` module.

1. In Google Cloud, create an OAuth client for a web application.
2. Register the redirect address `<public address>/api/v1/public/google/callback`, using the public address from Settings, General. The module's status (`GET /api/v1/google/status`) returns the exact address.
3. In Settings, Modules, configure Google: client ID, client secret, the email domains that sign in with Google, and optionally the option that refuses password sign-in for those domains. For the calendar, choose the activity type used for synced meetings and whether to import only events with a CRM contact or all timed events.
4. Enable the module.

Without domains, the Google sign-in button does not appear. Requiring single sign-on needs at least one domain.

## Microsoft

The `microsoft` module provides sign-in with a Microsoft account, Outlook calendar sync and Outlook mailboxes. It depends on `sso`.

Register an app in Microsoft Entra ID with the redirect address `<public address>/api/v1/public/microsoft/callback`. In Settings, Modules, set the client ID and secret, the email domains, and the **tenant**: a directory (tenant) ID to accept only your company, or `organizations` (the default) to accept any work or school account. The calendar options are the same as for Google.

## Single sign-on (OIDC)

The `sso` module connects any OpenID Connect provider (Okta, Keycloak, Auth0, Entra ID and others).

1. Create an application at your provider and register the redirect address `<public address>/api/v1/public/sso/callback`.
2. In Settings, Modules, configure Single sign-on: the issuer address or discovery document URL (an `https://` address), the client ID and secret, a label for the button, the email domains that use it, and optionally whether password sign-in is refused for those domains.
3. Enable the module.

Users whose email domain matches are offered the sign-in button. Sign-in attempts through single sign-on are rate limited to 30 per 5 minutes. See [Security](/_aphel/docs/admin/security/).

## Telephony

The `telephony` module logs calls as tasks and keeps call outcomes. Click-to-call providers come from separate provider modules, which implement a common interface. With no provider, the web app dials with a `tel:` link. A provider reports calls to `POST /api/v1/telephony/events`, and the first event of a call logs it as the acting user. No telephony provider is built into the current modules.

## Electronic signature

The `esign` module sends PDFs attached to deals to **Clicksign** and tracks their status through webhooks. It has no legal validity of its own: validity comes from the provider.

1. In Clicksign, generate an access token in the API settings.
2. In Settings, Modules, configure Electronic signature: the environment (`sandbox` or `production`, default sandbox), the access token, the HMAC secret of the webhook and whether to attach the signed PDF to the deal.
3. In Clicksign, register the webhook address `<public address>/api/v1/public/esign/webhook` with the same HMAC secret. Requests are verified with the `Content-Hmac` header.

## AI assistant

The `ai` module adds summaries, next-step suggestions, a writing assistant and natural-language queries.

| Setting | Meaning |
| --- | --- |
| Provider | `Anthropic` or an OpenAI-compatible service (OpenAI, Ollama, LM Studio). Default Anthropic. |
| Provider URL | Optional. Empty uses the provider's public address. For a local Ollama use `http://localhost:11434/v1`. |
| Model | Required. Default `claude-sonnet-5-5`. |
| API key | Secret. Optional for local OpenAI-compatible servers. |
| Daily limit | Calls to the provider per day, 1 to 100000. Default 200. |
| Limit per user per hour | 1 to 10000. Default 60. |
| Send personal data | On by default. When off, fields marked as personal data and names of people are not sent. Sensitive fields are never sent. |
| Features | Each of the four features (summary, next actions, composing, query) can be switched off. |

Calls are logged and visible to users with the `ai.view_usage` permission (`ai.use` allows using the assistant). Data leaves your installation when this module is enabled. Decide what to allow in line with your privacy policy.

## CNPJ enrichment

The `enrichment` module looks up public company data by CNPJ and fills the fields you choose, recording the source of each value. It uses BrasilAPI, with ReceitaWS as a fallback, so the server needs outbound internet access. The source can be BrasilAPI with ReceitaWS as backup (default), BrasilAPI only, or ReceitaWS only (limited to 3 queries per minute). By default it runs only when a user asks. The automation action that fills fields without a click works only if you turn on automatic enrichment.

## Messaging channels

The `messaging` module is a shared inbox for **WhatsApp Cloud API** and email (IMAP and SMTP), with conversations on the contact timeline. Channels are managed with the `messaging.manage_channels` permission.

For WhatsApp, a channel needs the phone number ID and three secrets: the access token, the app secret and a verify token. The channel shows the webhook address to register at Meta. WhatsApp accepts free-form replies only within 24 hours of the customer's last message, then only approved templates. The module settings include whether to create a contact for an unknown sender and which user acts for inbound messages.

## Webhooks

The `webhooks` module has two directions. See [Webhooks and events](/_aphel/docs/reference/webhooks-and-events/) for payloads and signatures.

- **Outgoing** webhooks call your URL when events happen. Delivery behavior (https required, timeout, failures before disabling, history retention) is set in Settings, Outgoing webhooks, Delivery. Targets on private networks are blocked unless `WEBHOOKS_ALLOW_PRIVATE_NETWORKS` is set.
- **Incoming** webhooks give you a URL with a token (`/api/v1/hooks/{token}`) that outside systems call. A signing secret can be set and the token regenerated.

## Check an integration

The System screen lists recent failures of automations, outgoing webhooks and integrations. See [Operations](/_aphel/docs/admin/operations/). Plugins can add their own integrations, see [Plugins overview](/_aphel/docs/plugins/overview/).
