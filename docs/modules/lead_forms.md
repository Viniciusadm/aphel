# "Lead forms" module (`lead_forms`)

Public forms, hosted by Aphel or embedded in the customer's website, that create (or reuse) a **contact** and optionally a **deal** on each submission. It is a regular module (former extension, session 35): `back/modules/lead_forms` (crate `aphel-mod-lead_forms`, struct `LeadFormsModule`, depends on `people` and, optionally, on `leads`, `deals`, `activities` and `lead_distribution`) and `web/src/modules/lead_forms`. Authenticated routes under `/api/v1/lead-forms` and public ones under `/api/v1/public/lead-forms/`.

## For users

1. **Settings → Integrations → Modules → Lead forms → Enable.**
2. Grant the **Manage lead forms** permission (`lead_forms.manage`) to the roles that will configure forms (administrator already has it).
3. **Forms** menu (`/lead-forms`) → **New form**:
   - **Fields**: label, identifier, type (text, email, phone, long text, option list, checkbox), required and order. Each field can **save to the contact** in a system or custom field. "Full name" splits first and last name. A required field must go to the name (or full name).
   - **Create records as**: records are created with this user's permissions (by default, whoever created the form), who must be able to create people (and deals, if configured).
   - **Contact**: reuse the contact when the email already exists, source and tag.
   - **Deal** (optional): pipeline, stage, title from a template (`{{identificador}}` inserts an answer, `{{form}}` the form name) and amount.
   - **After submission**: button text, success message or redirect URL.
   - **Allowed websites** (optional): restricts who can show the form in an iframe and submit through the script.
4. **Embed** shows the ready-made codes (iframe, script and direct link) and the preview of the public page. **Submissions** shows the history, including spam and failures, with links to the contact and the deal.

## Routes

Authenticated (`/api/v1/lead-forms`, permission `lead_forms.manage`):

| Method | Path | What it does |
| --- | --- | --- |
| `GET` | `/forms` | List with counters (`accepted_count`, `spam_count`, `failed_count`, `last_submission_at`) |
| `POST` | `/forms` | Creates (generates an opaque `public_id`, `f_` + 32 random hex characters); event `lead_form.created` |
| `GET` | `/forms/{id}` | Detail |
| `PUT` | `/forms/{id}` | Replaces the configuration (keeps the `public_id`); event `lead_form.updated` |
| `DELETE` | `/forms/{id}` | Deletes the form and its history (people and deals remain); event `lead_form.deleted` |
| `POST` | `/forms/{id}/rotate-public-id` | New `public_id`; the old codes stop working immediately |
| `GET` | `/forms/{id}/submissions?status=&page=&per_page=` | History (`accepted`, `spam`, `failed`) |
| `GET` | `/options` | Writable contact fields, active pipelines/stages, contact tags and users (lists the user cannot read come back empty) |

Public (`/api/v1/public/lead-forms`, no authentication and no `X-Requested-With`):

| Method | Path | What it does |
| --- | --- | --- |
| `GET` | `/forms/{public_id}` | JSON definition for rendering (labels, types, options, limits, honeypot field, submission path). Never exposes the mapping, pipeline or user |
| `POST` | `/forms/{public_id}/submissions` | Submission as `application/json` or `application/x-www-form-urlencoded` |
| `GET` | `/forms/{public_id}/page` | Self-contained HTML page for an iframe or direct link; works without JavaScript |
| `GET` | `/forms/{public_id}/embed.js` | Script that renders the form in `<div data-aphel-form="<public_id>">` and submits via `fetch` |

Iframe code:

```html
<iframe src="https://<your-aphel>/api/v1/public/lead-forms/forms/<public_id>/page" title="Contato" style="width:100%;min-height:560px;border:0" loading="lazy"></iframe>
```

Script code:

```html
<div data-aphel-form="<public_id>"></div>
<script src="https://<your-aphel>/api/v1/public/lead-forms/forms/<public_id>/embed.js" async></script>
```

Direct submission (custom integrations):

```bash
curl -X POST "https://<your-aphel>/api/v1/public/lead-forms/forms/$PUBLIC_ID/submissions" \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"name":"Ana Souza","email":"ana@acme.com.br","_aphel_hp":""}'
```

Response: `{ "data": { "status": "accepted", "success_message": ..., "redirect_url": ... } }`. An HTML form `POST` (urlencoded `Content-Type` and `Accept` with `text/html`) receives the page itself with the success message, the per-field errors or a `303` to the redirect URL.

## Protections of the public routes

- **Resolution by form**: the `public_id` resolves the form; a nonexistent or inactive form or a disabled module all respond the same, 404 `lead_form_not_found`, without revealing which case it is.
- **Rate limit** (the kernel's `SlidingWindowLimiter`, in memory per process): 120 reads/min per IP; 5 submissions/min per IP and form; 300 submissions/h per form. Above that, 429 `rate_limited`.
- **Honeypot field** `_aphel_hp`: hidden from the visitor; if it comes filled in, the response is a success, nothing is created and the submission is kept in the history as `spam`.
- **Size and validation**: body up to 64 KB (`lead_form_payload_too_large`); text up to 500 characters, long text up to 5000; email, phone (8 to 15 digits), list option and required fields are validated; answers outside the form are ignored.
- **Allowed websites**: with the list filled in, an `Origin` outside it (including `null`) gets 403 `lead_form_origin_not_allowed` and the page is served with `Content-Security-Policy: frame-ancestors 'self' <sites>`; without a list, any website can embed it. Aphel itself (the origin of the saved public address or the request's `Host`) is always accepted, so that the hosted page works.
- **Page**: everything escaped; CSP `default-src 'none'; style-src 'unsafe-inline'`; `X-Content-Type-Options: nosniff`; `Cache-Control: no-store`. The script builds the form only with `textContent`/`setAttribute`.

## What an accepted submission does

1. Loads the scope of the acting user (`build_access_scope`). If they are no longer a member, are deactivated or cannot create people (or deals, when configured), the submission fails with 503 `lead_form_owner_unavailable` and is kept in the history as `failed`.
2. With "reuse contact" and a field saving to `email`, it looks for a contact visible to that user with the same email (`ctx.query()`, the record query engine). Otherwise it creates the contact through the `RecordTypeHandler::create` of `contact` (`ctx.registry().handler("contact")`), with the configured source if the `source` field was not mapped. If the CRM rejects the contact, the submission is `failed` and the visitor gets 422 `lead_form_invalid_submission` (fields identified by their identifier) or 503 `lead_form_submission_failed`.
3. Applies the tag (`tags::service::tag_record`, from the `tags` module) and creates the deal linked to the contact through the `deal` handler (`{ title, pipeline_id, stage_id, contact_id, amount, source }`). Failures here do not lose the lead: the submission stays `accepted` with `error_code` `lead_form_tag_failed` or `lead_form_deal_failed`.
4. In the same transaction: writes `lead_forms_submissions`, publishes **`lead_form.submitted`** (with `record` = contact; payload `form_id`, `form_name`, `submission_id`, `contact_id`, `contact_created`, `deal_id`) and writes the audit.

Contact, tag and deal go through the core use cases: they generate their own events (`contact.created`, `deal.created`, `record.tagged`), audit, automations and outgoing webhooks.

`lead_form.submitted` is an automation trigger ("Lead form submitted", record = contact).

## Data

| Table | Content |
| --- | --- |
| `lead_forms_forms` | Name, `public_id` (unique), active, acting user, `definition` (JSON with fields, mapping, deal, tag, source, messages, allowed websites), creator |
| `lead_forms_submissions` | Status (`accepted`, `spam`, `failed`), created contact/deal, answers to the form fields (each value truncated at 1000 characters), IP, `Origin`, error code and message |

Both are included in the module's data export and removal. The history stores personal data submitted by the visitor: delete old forms or remove the module's data according to the organization's retention policy.

## Module-specific error codes

| Code | HTTP | When |
| --- | --- | --- |
| `lead_form_not_found` | 404 | Nonexistent or inactive form, or module disabled |
| `lead_form_invalid` | 422 | Invalid configuration (`details.fields` with `name`, `fields.<i>.<field>`, `deal.*`, `acting_user_id`, `tag_id`, `redirect_url`, `allowed_origins.<i>`...) |
| `lead_form_limit_reached` | 409 | 200 forms in the installation |
| `lead_form_invalid_query` | 422 | Invalid filter or pagination in the history |
| `lead_form_origin_not_allowed` | 403 | `Origin` outside the allowed websites |
| `lead_form_invalid_payload` | 422 | Body that is neither a JSON object nor a urlencoded form |
| `lead_form_payload_too_large` | 422 | Body above 64 KB |
| `lead_form_invalid_submission` | 422 | Invalid answers (`details.fields[].field` = field identifier; messages in pt-BR to show to the visitor) |
| `lead_form_owner_unavailable` | 503 | The acting user lost access or permission |
| `lead_form_submission_failed` | 503 | The CRM rejected the contact for another reason |
| `rate_limited` | 429 | Read or submission limit |

In the history, `error_code` can also be `lead_form_tag_failed` or `lead_form_deal_failed` (submission accepted with a pending issue) or the code returned by the core when creating the contact.

## Known limitations

- The rate limit is in memory and per process; behind a proxy, the IP seen is the proxy's (the host does not read `X-Forwarded-For`).
- Email deduplication respects the acting user's visibility: if they only see their own people, a contact of another owner with the same email is not found.
- The list value saved into a contact select field must be the field option's key (use "value | label" in the options).
- There is no concurrent version control (`If-Match`) on `PUT`: the last one to save wins.
