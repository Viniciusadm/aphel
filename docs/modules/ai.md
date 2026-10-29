# `ai` module — AI assistant

Optional AI for Aphel: summaries of contacts, companies and deals, next-action suggestions, a writing assistant in the editors and natural-language queries that become a **validated filter**. It ships disabled (it does not use `enabled_by_default`) and does nothing until an administrator enables it and configures a provider.

- Backend: crate `aphel-mod-ai` (`back/modules/ai`, struct `AiModule`, in the default build via `back/aphel.modules.toml`; `deals` and `activities` optional).
- Web: `web/src/modules/ai` (**AI** tab on records, **AI assistant** button in the editors, **Ask AI** button on lists, `/ai` page, panel in the module settings).

## Providers

`AiProvider` interface (`complete(AiRequest) -> AiResponse`, with `system`, messages, `max_tokens`, optional temperature; the response carries the text and the input and output tokens). Timeout of 60 s per call (10 s to connect).

| Provider (`provider`) | Call | Authentication | Default URL |
| --- | --- | --- | --- |
| `anthropic` | `POST {base_url}/v1/messages` (`anthropic-version: 2023-06-01`), reads `content[].text` and `usage.input_tokens/output_tokens`; `stop_reason = refusal` becomes `ai_refused` | `x-api-key` (required) | `https://api.anthropic.com` |
| `openai_compatible` | `POST {base_url}/chat/completions` with the instruction as a `system` message, reads `choices[0].message.content` and `usage.prompt_tokens/completion_tokens`; `finish_reason = content_filter` becomes `ai_refused`; a leading `<think>…</think>` block (local reasoning models) is discarded | `Authorization: Bearer` if there is a key (required only for OpenAI's default URL) | `https://api.openai.com/v1` |

Local OpenAI-compatible servers: Ollama `http://localhost:11434/v1`, LM Studio `http://localhost:1234/v1` (no key). The URL is configured by the administrator and accepts `http`/`https`, including internal addresses: that is how a local model works. The temperature is only sent to the OpenAI-compatible adapter (current Anthropic models reject the parameter).

## Settings

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `provider` | Select | `anthropic` | `anthropic` or `openai_compatible` |
| `base_url` | Url | empty | Empty uses the provider's default URL |
| `model` | Text | `claude-sonnet-5-5` | E.g. `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`, `gpt-4o-mini`, `llama3.1` |
| `api_key` | Secret | — | Encrypted in the `SecretVault`; never comes back through the API, logs, events or audit (the API only shows `api_key_configured`) |
| `daily_limit` | Number (1–100000) | 200 | Calls to the provider per day for the installation (organization time zone) |
| `user_hourly_limit` | Number (1–10000) | 60 | Calls per user per hour (in-memory sliding window, per process) |
| `send_personal_data` | Boolean | on | When off, fields with `is_personal` and people's names (owner, contact, note author) are left out |
| `feature_summary`, `feature_next_actions`, `feature_compose`, `feature_query` | Boolean | on | Turns each feature on/off (403 `ai_feature_disabled`) |

## Which data goes to which service

The provider is only called when a user clicks an AI button. All data leaves **with the scope of whoever asked**: the record comes from the type's `RecordTypeHandler::load` (`ctx.registry().handler(record_type)`; read permissions, visibility and field permissions already applied, hidden fields removed), related lists come from the core API with the same scope, and fields are filtered by the user's `FieldSet::for_scope`.

| Feature | Receiving service | What is sent | What is never sent | What comes back |
| --- | --- | --- | --- | --- |
| Summary (`POST /summary`) | Configured provider (`provider` + `base_url` + `model`) | Fields of the contact/company/deal (system and custom), names of references (owner, company, contact, stage, tags), up to 10 recent notes (text up to 600 characters, author if personal data is on), up to 10 recent activities (title, type, status, priority, dates, description up to 400 characters); contact: up to 10 of its deals; company: up to 10 contacts (name, job title) and 10 deals; organization language and today's date. Context limited to 30 thousand characters | `is_sensitive` fields; fields hidden from the user's role; records outside visibility; internal ids; with `send_personal_data` off, `is_personal` fields and people's names | Summary text |
| Next actions (`POST /next-actions`) | Configured provider | Same as the summary | Same as the summary | Up to 5 suggestions `{ title, due_in_days, reason }` (validated JSON). Nothing is created: the UI creates the task (`POST /api/v1/activities`, type `task`, linked to the record) only when the user clicks |
| Writing (`POST /compose`) | Configured provider | Instruction, current editor draft (up to 20 thousand characters) and, if the user checks the option, the record fields (without notes or activities) | Same as the summary | Suggested text; the editor only changes when the user clicks **Replace text**/**Append** |
| Query (`POST /query`) | Configured provider | Question (up to 500 characters), record type, today's date and the list of **non-sensitive** filterable fields visible to the user (key, label, type, operators and up to 60 options) | Any record data | AST filter + explanation; the backend validates it and counts the results before responding |

Aphel **does not store** prompts or responses. The data sent is handled under the chosen provider's policy (for Anthropic and OpenAI, the data policy of the account owning the configured key; for a local model, it stays on your network).

## Natural-language query

The AI receives the filter grammar (groups `{ "op": "and"|"or", "conditions": [...] }`, conditions `{ "field", "operator", "value" }`), the fields with their operators and examples, and responds with `{ "filter", "explanation" }`. The backend:

1. Rejects anything that is not the AST: only the keys `op`/`conditions`/`field`/`operator`/`value` (no `related`), depth up to 4, up to 50 conditions, only fields from the list sent, only operators of the field's type and the correct value format (none, single, pair, list up to 100, days 1–3650).
2. Runs `ctx.query()` with `RecordQuery::new().filter(ast).window(0, 1)` in the user's scope: the query engine from session 04 validates against the field registry and permissions and returns only the total. Nothing other than this filter is executed; there is no SQL coming from the AI.
3. If the filter is invalid, it sends the error back to the AI **once** for correction (it counts as one more call). If it persists, 422 `ai_query_invalid`. If the AI says the question cannot become a filter, 422 `ai_query_unanswerable` with the reason.

The UI shows the filter in readable language (the same labels as the filter chips), the collapsible JSON and the total; **Apply filter** (or **Combine with current filter**) calls the list's `applyFilter`.

## Permissions

| Permission | Use |
| --- | --- |
| `ai.use` | Use summary, suggestions, writing and query; see `GET /status` |
| `ai.view_usage` | See `GET /status` and the call log (`GET /calls`) |

Creating a task from a suggestion also requires `activities.create`. Configuring the module requires `settings.manage` (the core modules screen, Settings → Integrations → Modules).

## Routes (`/api/v1/ai`)

| Route | Description |
| --- | --- |
| `GET /status` | `{ data: { configured, api_key_configured, provider, base_url, model, daily_limit, used_today, remaining_today, user_hourly_limit, send_personal_data, features } }` — never the key |
| `POST /summary` | `{ record_type: contact\|company\|deal, record_id }` → `{ data: { text, sources, provider, model } }` |
| `POST /next-actions` | Same body → `{ data: { suggestions: [{ title, due_in_days, reason }], sources, provider, model } }` |
| `POST /compose` | `{ context: note\|email\|message, instruction, draft?, record_type?, record_id? }` → `{ data: { text, sources, provider, model } }` |
| `POST /query` | `{ record_type, question }` → `{ data: { filter, total, explanation, provider, model } }` (any readable record type, including custom objects) |
| `GET /calls` | `feature`, `status` (`success`/`error`), `page`, `per_page` (up to 100) → `{ data: [calls], meta: { total, page, per_page, used_today, daily_limit, last_30_days: { calls, errors, input_tokens, output_tokens } } }` |

`sources` lists the records used as context (`{ record_type, record_id, label }`: the record, notes, activities, related deals and contacts).

### Error codes

| Code | HTTP | When |
| --- | --- | --- |
| `ai_not_configured` | 503 | No model, invalid URL or no key when the provider requires one |
| `ai_feature_disabled` | 403 | Feature turned off in the settings |
| `ai_limit_reached` | 429 | Installation daily limit reached |
| `ai_user_limit_reached` | 429 | Per-user hourly limit reached |
| `ai_provider_rate_limited` | 429 | The provider responded 429 |
| `ai_provider_auth_failed` | 503 | The provider rejected the key (401/403) |
| `ai_provider_error` | 503 | Network or HTTP failure from the provider (the message carries only the status, never the key or the body) |
| `ai_provider_timeout` | 503 | No response within 60 s |
| `ai_empty_response` | 503 | Response without text |
| `ai_invalid_response` | 503 | Next actions not in the expected JSON |
| `ai_refused` | 422 | The model refused the request |
| `ai_record_not_found` | 404 | Record does not exist or is not visible to the user |
| `ai_unsupported_record_type` | 422 | Summary/actions/writing outside `contact`, `company`, `deal` |
| `ai_invalid_instruction` | 422 | Invalid `/compose` body (`details.fields`) |
| `ai_query_invalid` | 422 | Invalid `/query` body or invalid filter after correction |
| `ai_query_no_fields` | 422 | No filterable field available to the user |
| `ai_query_unanswerable` | 422 | The question cannot become a filter (message with the AI's reason) |
| `ai_invalid_calls_query` | 422 | Invalid filter or pagination in `/calls` |

## Limits, log and data

- `ai_usage (day, calls)`: atomic per-day reservation in the organization's time zone (`UPDATE … SET calls = calls + 1 WHERE calls < limit`) before each call to the provider. A provider failure also counts.
- `ai_calls`: one row per call to the provider with `user_id`, `feature`, `provider`, `model`, `status`, `input_tokens`, `output_tokens`, `duration_ms`, `error_code`, `created_at`. **No prompt or response.**
- No domain events or audit per call: no call changes business data (the log is operational). The task created from a suggestion goes through the core activities API, with its own event and audit.
- Data export: the two tables (generic) plus the totals of the last 30 days. Data removal deletes the two tables.
- Migration `1` (`0001_ai.sql`).

## Limitations

- No streaming: the response arrives in full (up to 60 s).
- The per-user limit is in memory per process; the daily limit is in the database and applies to all instances.
- Notes are included as text; the read permission for notes and visibility (private note only for its author) are the core ones, but a note's free-form content does not go through the sensitive field filter.
- Small local models may get the next-actions and query JSON wrong; validation rejects it and the UI shows the error.
