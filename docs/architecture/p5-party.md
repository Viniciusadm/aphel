# P5 party contract: first implementation slice

P5.1 introduces `aphel-contract-party`. It depends only on the contract primitives and serde, with no kernel, database, plugin, runtime or CRM dependency. `party.contact_points` describes contact fields on any entity. It does not prescribe a person/company record type or storage table.

`ContactPoint` identifies an entity field and its channel (`email`, `phone`, `web`). `ContactPoints` stores those mappings once; summary columns are derived in stable, unique order for consumers.

The native SDK exposes `PartyTraits` for publishing and reading the mapping. SQL-facing summary consumers receive only valid SQL identifiers. Generic phone/address field helpers and their existing bilingual labels have moved from the transitional CRM contract to `aphel_sdk::party`; field keys, Brazilian phone defaults and address labels are preserved.

## Connected providers and consumers

- `people` declares email, mobile and phone mappings and publishes the party contract descriptor.
- `organizations` declares domain and email mappings and publishes the same contract descriptor as another provider.
- `duplicates`, `trash` and `privacy` read the shared SDK projection instead of the CRM trait constant.

The host catalog lists each provider. No provider depends on the other through the new contract. Existing direct dependencies and SQL between the native people/organizations plugins are migration debt for P5.2, not resolved by this slice.

## D4 review checklist per plugin

These are static review results and manual acceptance cases. No database instance, automated test, probe or smoke run is used as evidence.

### Contacts (future people)

- [x] The mapping preserves `email`, `mobile`, `phone` order and phone-channel classification.
- [x] Shared field keys, labels, default phone country and privacy flags are preserved.
- [ ] Manually confirm create/edit, company links, archive/restore and access scopes after the P5.2 entity migration.

### Companies (future organizations)

- [x] The mapping preserves `domain`, `email` order and distinguishes web from email.
- [x] Company-specific hierarchy and relationships remain outside the party contract.
- [ ] Manually confirm company hierarchy, linked people, archive/restore and visibility after P5.2.

### Duplicates

- [x] Summary columns come from party metadata; the existing label-alias fallback remains.
- [ ] Manually confirm comparison, merge and summaries for both providers after the identity migration.

### Trash

- [x] Subtitle columns and the label-alias fallback remain in the same order.
- [ ] Manually confirm list, restore and purge for people/organizations after P5.2.

### Privacy

- [x] Subject subtitles use the shared party projection; matching, permissions and redaction are unchanged in this slice.
- [ ] Manually confirm subject lookup, export and erasure after the P5.2 migration.

## P5.2 implementation

The native plugins are now `people` and `organizations`, with canonical entities `people.person` and `organizations.organization`. Their owned schemas use `people`, `people_organizations` and `organizations`. The build selection, profiles, permissions, native consumers, web modules, routes and translated entity keys use the new identities directly.

The kernel's installation/profile code is named `organization_profile`, distinguishing the instance's settings from records owned by the organizations plugin. There is no identity rewrite job or compatibility alias.

P5.2 intentionally leaves the existing contacts/organization linking behavior in the people plugin while later P5 slices replace its direct dependency and cross-plugin SQL. The zero-dependency, SQL and foreign-key gates apply to the completed P5 phase, not this identity step.

## Remaining scope

P5.3 extracts workflow from pipelines, followed by tasks/calendar and messaging. Native dependency dissolution, owner-provided sources, schema cleanup and P5's zero-dependency/SQL/foreign-key gates remain pending.
