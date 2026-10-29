# "CNPJ enrichment" module (`enrichment`)

Looks up public data of Brazilian companies by **CNPJ** (BrasilAPI and ReceitaWS) and fills the chosen fields of the **company**, recording the **source** (provider, CNPJ and lookup date) of each filled field. Code in `back/modules/enrichment` (crate `aphel-mod-enrichment`, struct `EnrichmentModule`) and `web/src/modules/enrichment`. It ships in the default build (`back/aphel.modules.toml`), disabled until an administrator enables it.

**It is never automatic without configuration**: the default path is manual (look up, compare, select and apply). Only with "Allow automatic enrichment" turned on does the `enrichment.enrich_company` automation action fill fields without a click.

## For users

1. **Settings → Integrations → Modules → CNPJ enrichment → Enable** and, under **Configure**:
   - **Data source**: BrasilAPI with ReceitaWS as fallback (default), BrasilAPI only or ReceitaWS only.
   - **CNPJ field** (optional): the company text field that holds the CNPJ. Without it, the CNPJ is typed during the lookup (and, if the field is configured, the typed CNPJ can also be saved to it).
   - **Mapping**: for each piece of data (legal name, trade name, main activity/CNAE, company size, registration status, opening date, full address, street, number, complement, district, city, state, postal code, phone, email) choose the company field that receives it; empty = do not fill. Defaults: address in the system address fields, phone in `phone`, email in `email`, activity in `industry`.
   - **Allow automatic enrichment** (off by default).
2. Grant the **Look up CNPJ and apply public data** permission (`enrichment.use`). Applying also requires `companies.update` (the core field permissions apply).
3. On the company, **Enrichment** tab: the CNPJ comes from the configured field (or type it) → **Look up**. The list shows, item by item, the current value × the new one, the target field and the source. Select what to apply → **Apply**. The "Fields filled by enrichment" section shows the source, the CNPJ and the date of each field.

## Settings

| Key | Type | Default |
| --- | --- | --- |
| `source` | Select (`brasilapi_then_receitaws`, `brasilapi`, `receitaws`) | `brasilapi_then_receitaws` |
| `cnpj_field` | Field (company, text) | |
| `automatic` | Boolean | `false` |
| `field_legal_name`, `field_trade_name`, `field_activity`, `field_size`, `field_registration_status`, `field_opened_on`, `field_address`, `field_address_street`, `field_address_number`, `field_address_complement`, `field_address_district`, `field_address_city`, `field_address_state`, `field_address_postal_code`, `field_phone`, `field_email` | Field (company) | see above |

There are no secrets: both sources are public and keyless.

## Permissions, events and automation

- `enrichment.use`: look up and apply (with `companies.read`/`companies.update`).
- Event `company.enriched` (`record = company`), declared and offered as the "Company enriched by CNPJ" trigger. Payload: `company_id`, `cnpj`, `source`, `fetched_at`, `fields` and `changes: [{ field, old, new }]` (fields hidden from the reader are masked in the timeline by the core). Audit in the same transaction with the changes and the source.
- Writing the fields goes through the company's `RecordTypeHandler::update` (`ctx.registry().handler("company")`), so the company also publishes `company.updated` with the core's audit, validation and permissions.
- Automation action **Enrich company by CNPJ** (`enrichment.enrich_company`, `company` records): requires `enrichment.use`, `companies.read` and `companies.update`; it only validates and runs with `automatic` turned on (`enrichment_automatic_disabled`). By default it fills only empty fields; `{ "overwrite": true }` replaces values.

## Routes (`/api/v1/enrichment`)

| Method | Path | What it does |
| --- | --- | --- |
| `GET` | `/companies/{company_id}` | CNPJ field and current value, source mode, `automatic` and the source of each field already filled |
| `POST` | `/lookups` | `{ company_id, cnpj? }` → 201 with the field-by-field proposal (`data_key`, `field_key`, `current`, `proposed`, `changed`). Writes nothing to the company |
| `POST` | `/lookups/{lookup_id}/apply` | `{ items: ["legal_name", ...], only_blank? }` → writes the chosen items, records the sources and publishes `company.enriched` |

The lookup is stored in `enrichment_lookups` for up to 24 h (valid for 60 min for applying), so that the recorded source is that of the actual lookup and not a value sent by the browser.

### Error codes

| Code | HTTP | When |
| --- | --- | --- |
| `enrichment_cnpj_missing` | 422 | No CNPJ typed and none in the configured field |
| `enrichment_invalid_cnpj` | 422 | Invalid check digits (alphanumeric CNPJs are also accepted) |
| `enrichment_invalid_request` | 422 | Invalid ids or too many items |
| `enrichment_cnpj_not_found` | 404 | The source does not know the CNPJ |
| `company_not_found` | 404 | Company does not exist or is not visible |
| `enrichment_lookup_not_found` | 404 | Lookup does not exist |
| `enrichment_lookup_expired` | 422 | Lookup older than 60 min |
| `enrichment_nothing_to_apply` | 422 | No item with a mapped field and a different value |
| `rate_limited` | 429 | More than 60 lookups/min on the installation or 15/min per user |
| `enrichment_source_rate_limited` | 429 | The source rate-limited the request (ReceitaWS accepts 3 lookups/min; Aphel enforces that limit in the process) |
| `enrichment_source_unavailable` | 503 | Source down or slow (15 s timeout) |
| `enrichment_automatic_disabled` | 422 | Automation action with `automatic` turned off |

## Data sent to third parties

Only the CNPJ, in `GET https://brasilapi.com.br/api/cnpj/v1/{cnpj}` and/or `GET https://receitaws.com.br/v1/cnpj/{cnpj}`. No other CRM data leaves.

## Tables

`enrichment_lookups` and `enrichment_field_sources` (`record_type`, `record_id`, `field_key`, `data_key`, `source`, `source_reference` = CNPJ, `value_snapshot`, `fetched_at`, `applied_by`, `applied_at`; one row per field, the most recent one). Data export/removal covers both.

## Limitations

- Public sources may be outdated, down or rate-limited; BrasilAPI may not yet know alphanumeric CNPJs.
- Data is written as text; list (select) fields are not part of the mapping. The opening date is written as `YYYY-MM-DD` (date or text field).
- The company update (core) and the source record are two transactions: if recording the source fails after the company was updated, the values remain written without the source.
