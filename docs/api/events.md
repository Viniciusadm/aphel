# Event catalog

Stable list of Aphel's public events, delivered by outgoing webhooks and queryable at `GET /api/v1/events/catalog`. A new event type is only ever added; renaming or removing a type is a breaking change (see [`versioning.md`](versioning.md)).

Internal events (configuration, import, export, bulk actions, tokens, etc.) are still written to the outbox, but they are not part of this contract and are not delivered by webhook.

## Delivered body

```json
{
  "id": "0192f0c4-8c1e-7d2a-9f3b-1c2d3e4f5a6b",
  "type": "deal.stage_changed",
  "actor": { "type": "user", "id": "0192f0a0-0000-7000-8000-000000000002" },
  "occurred_at": "2026-10-21T14:03:12.481Z",
  "data": { "id": "0192f0b1-...", "title": "Contrato anual", "stage": { "id": "...", "name": "Proposta" }, "...": "..." },
  "previous": { "stage": { "id": "...", "name": "Qualificação" } }
}
```

| Property | Content |
| --- | --- |
| `id` | Event id (UUID v7). Use it to discard repeated deliveries: delivery is **at least once** |
| `type` | Event type, from the table below |
| `actor` | Who caused it: `type` = `user`, `api_token`, `system` or `automation`; `id` is null for `system` |
| `occurred_at` | When the event happened (UTC, RFC 3339) |
| `data` | The resource in the public API format, the same as the `data` of `GET /api/v1/<resource>/{id}` (schema in the catalog's `data_schema`). It reflects the record's state when the event is processed, right after it happens. In deletion events it only carries `{ "id" }` |
| `previous` | Only in change events (`has_previous` in the catalog): an object `{ <field>: <previous value> }` (see below) |
| `context` | Optional: event data that is not in the resource (e.g. `pricing` and `components` of `product_bundle.updated`) |

### Format of `previous`

`previous` is always a flat object in which each key is a field and each value is that field's **previous** value; the current value is in `data`. The rules apply to all events:

- In `<type>.updated` events (and in generic change events, such as `deal.member_*`) the keys are the same as in the audit log: the system field name (`owner_id`, `email`), `custom_fields.<key>` for custom fields and `<group>.<field>` for nested data (`member.role`). Only the fields that changed appear.
- In specific change events (`deal.stage_changed`, `deal.amount_changed`, `exchange_rate.rebased`…) the keys are fixed and documented in the table below, and they **always** appear: if there was no previous value, the value is `null`.
- Sensitive fields come as `"[redacted]"`; fields hidden from the webhook owner's role are removed, and if nothing is left `previous` is omitted.

Records that stopped being visible before processing (e.g. private notes, records deleted right afterwards) produce no delivery for events other than deletion.

## Events

| Type | Resource (`data`) | `previous` |
| --- | --- | --- |
| `contact.created` | Contact | — |
| `contact.updated` | Contact | `{ <field>: <old value> }` |
| `contact.deleted` | `{ id }` | — |
| `contact.archived` | Contact | — |
| `contact.unarchived` | Contact | — |
| `contact.merged` | Contact that absorbed the other one | — |
| `lead.qualified` | Contact qualified in the lead inbox | — |
| `lead.disqualified` | Disqualified contact (reason in `GET /lifecycle/history/contact/{id}` and in the report) | — |
| `lead.converted` | Converted contact (the created deal arrives in `deal.created`) | — |
| `company.created` | Company | — |
| `company.updated` | Company | `{ <field>: <old value> }` |
| `company.deleted` | `{ id }` | — |
| `company.archived` | Company | — |
| `company.unarchived` | Company | — |
| `company.merged` | Company that absorbed the other one | — |
| `deal.created` | Deal | — |
| `deal.updated` | Deal | `{ <field>: <old value> }` (stage, pipeline and status have their own events; amount/currency, owner and expected close date too, and they are not repeated here when their own event goes out for the same change) |
| `deal.stage_changed` | Deal | `{ stage: { id, name } }` |
| `deal.pipeline_changed` | Deal | `{ pipeline: { id, name }, stage: { id, name } }` |
| `deal.amount_changed` | Deal | `{ amount }` (the internal payload carries `changes` with `amount`, `currency`, `amount_base` and `exchange_rate`) |
| `deal.owner_changed` | Deal | `{ owner: { id, name } \| null }` (the internal payload carries `changes` with `owner_id`) |
| `deal.expected_close_date_changed` | Deal | `{ expected_close_date }` (the internal payload carries `changes` with `expected_close_date`) |
| `deal.won` | Deal | — |
| `deal.lost` | Deal | — |
| `deal.reopened` | Deal | — |
| `deal.deleted` | `{ id }` | — |
| `deal.archived` | Deal | — |
| `deal.unarchived` | Deal | — |
| `deal.renewal_created` | Renewal deal (`renewal_of_deal_id` points to the original) | — |
| `deal.checklist_item_checked` | Deal whose stage checklist item was checked | — |
| `deal.checklist_item_unchecked` | Deal whose stage checklist item was unchecked | — |
| `deal.member_added` | Deal | `{ "member.user_id": null, "member.role": null, "member.credit_percent": null }` (the team is at `GET /deals/{id}/members`) |
| `deal.member_updated` | Deal | `{ "member.role" \| "member.credit_percent": <old value> }` |
| `deal.member_removed` | Deal | `{ "member.user_id", "member.role", "member.credit_percent" }` of the removed member |
| `activity.created` | Activity | — |
| `activity.updated` | Activity | `{ <field>: <old value> }` |
| `activity.completed` | Activity | — |
| `activity.reopened` | Activity | — |
| `activity.deleted` | `{ id }` | — |
| `call.logged` | Call activity (details in `call`) | — |
| `call.updated` | Call activity | `{ <field>: <old value> }` (direction, outcome, duration, phone, recording) |
| `quote.created` | Quote (version) | — |
| `quote.updated` | Quote | `{ <field>: <old value> }` (line items in `items`) |
| `quote.deleted` | `{ id }` | — |
| `quote.version_created` | New quote version (`previous_id` in the internal payload) | — |
| `quote.submitted` | Quote submitted for approval | — |
| `quote.approved` | Approved quote | — |
| `quote.returned_to_draft` | Quote that went back to draft (approval rejected or cancelled) | — |
| `quote.sent` | Quote sent to the customer by email with the public link | — |
| `quote.viewed` | Quote opened by the customer through the public link (first time) | — |
| `quote.accepted` | Quote accepted by the customer through the public link or recorded manually by a user (`manual: true`, `recorded_by`, `note`) | — |
| `quote.rejected` | Quote declined by the customer through the public link | — |
| `quote.expired` | Sent quote that passed its validity date | — |
| `quote.deal_synced` | Accepted quote whose line items and amount were applied to the deal | — |
| `quote.deal_sync_failed` | Accepted quote whose application to the deal failed (reason in the internal payload) | — |
| `product.created` | Catalog product | — |
| `product.updated` | Catalog product | `{ <field>: <old value> }` |
| `product.deleted` | `{ id }` | — |
| `note.created` | Note | — |
| `note.updated` | Note | — |
| `note.deleted` | `{ id }` | — |
| `meeting.booked` | Contact who booked through the booking link | — |
| `meeting.rescheduled` | Contact of the rescheduled meeting | — |
| `meeting.canceled` | Contact of the canceled meeting | — |
| `sequence.enrolled` | Contact enrolled in the sequence | — |
| `sequence.completed` | Contact who completed all steps of the sequence | — |
| `sequence.exited` | Contact who left the sequence (reply, meeting, deal won/lost, bounce, objection or removal) | — |
| `approval.requested` | Deal whose approval was requested (stage change or on demand). Requests for quotes and other types are not delivered by webhook (the catalog ties each event type to a single resource) | — |
| `approval.approved` | Deal whose approval request was approved at every step | — |
| `approval.rejected` | Deal whose approval request was rejected | — |
| `approval.cancelled` | Deal whose approval request was cancelled | — |
| `score.band_changed` | Contact whose score crossed into another band (cold, warm, hot) of a scoring model. Deals and custom objects only produce the internal event and the automation trigger | `{ score_band: <old band> }` |
| `score.company_band_changed` | Company whose score crossed into another band (cold, warm, hot) of a scoring model | `{ score_band: <old band> }` |
| `proposal.generated` | Deal whose PDF proposal was generated and attached (`proposals` module) | — |
| `esign.queued` | Deal whose signature request was queued for delivery | — |
| `esign.cancel_queued` | Deal whose signature cancellation was queued | — |
| `esign.failed` | Deal whose signature delivery failed or could not be confirmed | — |
| `esign.sent` | Deal whose document was sent for electronic signature (`esign` module) | — |
| `esign.signed` | Deal whose document was signed by a signer | — |
| `esign.refused` | Deal whose document was refused by a signer | — |
| `esign.cancelled` | Deal whose signature request was cancelled | — |
| `esign.completed` | Deal whose document was signed by all signers | — |
| `esign.expired` | Deal whose signature request expired | — |
| `esign.signed_file_attached` | Deal to which the signed file was attached | — |
| `company.enriched` | Company whose fields were filled by the public CNPJ lookup (`enrichment` module) | — |
| `lead_form.submitted` | Contact created or updated by the submission of a capture form (`forms` module) | — |
| `exchange_rate.created` | Exchange rate (`ExchangeRateResponse`, the same as `GET /currencies/rates/{id}`), entered by hand or fetched from the configured provider (`source` = `integration`) | — |
| `exchange_rate.updated` | Exchange rate | `{ <field>: <old value> }` (`rate`, `effective_date`, `source`) |
| `exchange_rate.deleted` | `{ id }` | — |
| `exchange_rate.rebased` | `{ previous_base, base, converted, created, removed }` (change of the organization's base currency: the stored exchange rates were converted to the new base) | `{ base: <previous base currency> }` |
| `currency.enabled` | `{ currency, base }` (currency enabled in the organization) | — |
| `currency.disabled` | `{ currency, base }` (currency disabled in the organization) | — |
| `organization.updated` | `{ id, changed_fields, currency, previous_currency? }` (company data changed; `previous_currency` only when the base currency changed) | `{ currency: <previous base currency or null> }` |
| `price_list.created` | Price list (`PriceListResponse`, the same as `GET /price-lists/{id}`) | — |
| `price_list.updated` | Price list | `{ <field>: <old value> }` |
| `price_list.deleted` | `{ id }` | — |
| `price_list.items_replaced` | Price list whose prices were replaced (items at `GET /price-lists/{id}/items`) | — |
| `product_bundle.updated` | Product whose bundle was saved (`context`: `pricing`, `components`; components at `GET /products/{id}/bundle`) | — |
| `product_bundle.removed` | Product that is no longer a bundle | — |
| `custom.<key>.created` | Custom object record | — |
| `custom.<key>.updated` | Custom object record | `{ <field>: <old value> }` |
| `custom.<key>.deleted` | `{ id }` | — |
| `custom.<key>.archived` | Custom object record | — |
| `custom.<key>.unarchived` | Custom object record | — |

`data` schemas: `ContactResponse`, `CompanyResponse`, `DealResponse`, `ActivityResponse`, `NoteResponse`, `QuoteDetailResponse`, `ProductResponse`, `PriceListResponse`, `ExchangeRateResponse` and `CustomRecordResponse` in `/api/openapi.json`.

### Events without a record

`currency.enabled`, `currency.disabled`, `exchange_rate.rebased` and `organization.updated` do not have a record with an id: their resource in the catalog is payload-only (`PublicResource::payload(..)`, null `data_schema` in `GET /api/v1/events/catalog`) and `data` is the event payload without internal keys (`changes`, `requested_by`) or keys that look like secrets (`secret`, `password`, `token`, `credential`, `api_key`). These events do not go through per-role field redaction.

The `currency.provider_updated` event (change of the exchange rate provider) stays internal: the provider configuration may contain an address with a credential. The current base currency and the enabled currencies are at `GET /currencies`. A base currency change reaches integrations through `organization.updated` (with `currency` and `previous_currency`) and through `exchange_rate.rebased` (exchange rates converted to the new base).

The `custom.<key>.*` events exist for each active custom object of the organization (`custom.contract.created` for the `contract` object) and appear with their concrete name in `GET /api/v1/events/catalog`. The record's fields are in `data.custom_fields`; each object's schema is described at runtime by `GET /api/v1/objects/{key}/schema`.

A single action can produce several events (e.g. moving a deal to the won stage produces `deal.stage_changed` and `deal.won`), each with its own `id`. Delivery order between events is not guaranteed; use `occurred_at`.

Configuration events with no use for integrations stay internal (they are not in the catalog or in webhooks; they serve the audit log, the timeline and Aphel's own consumers): `lead_form.created|updated|deleted`, `proposal_template.*`, `discount_limits.updated` (discount limit per role: an internal approval rule), `validation_rule.created|updated|deleted|reordered` (validation rules: the effect reaches integrations as a 422 `validation_rule_failed` error on write), `approval_policy.*`, `approval.step_advanced|decision_recorded|overdue` and those of workflow and stages (`pipeline.*`, `pipeline_stage.*`, stage guide and checklist; checking checklist items on a deal is public as `deal.checklist_item_checked|unchecked`). To track these configurations, export the configuration package.

## In the code

The catalog is `public_event_catalog()` in `back/kernel/src/application/public_events/catalog.rs`. The body is built by `PublicEventRenderer` (implementation `ApiPublicEventRenderer` in `back/modules/webhooks/src/outgoing/renderer.rs`): records whose type is in the `RecordTypeRegistry` are read through the `RecordTypeHandler`; the other resources (`note`, `exchange_rate`, `price_list`) through a `PublicResourceLoader` contributed by the owning module (`app.contribute::<dyn PublicResourceLoader>(..)`); payload resources use `payload_resource_data`. `previous` comes from `previous_values` according to the definition's `PreviousValues` (`Changes`, `FromTo`, `OldNew`, `Pipeline` or `Fields(&[(field, payload_key)])`). Making an event public means contributing it from the owning module with `app.public_event(..)` and adding a row to this table. Custom object events come from `custom_public_events()` (pattern `custom.<key>.<action>`), resolved by `find_public_event` from the type prefix.

## Plugin events and outgoing webhooks

Enabled plugins expose their `[[emits]]` declarations in `/api/v1/events/catalog` as `plugin.<key>.<name>`. These events support exact and wildcard outgoing webhook subscriptions. The webhook `data` is exactly the JSON object passed to `ctx.emit`, including nested values and sensitive-looking keys. Plugin authors are responsible for selecting public content; associated records are not loaded or added to the payload. Disabling or uninstalling a plugin removes its declarations after runtime refresh; existing queued deliveries retain their retry behavior.
