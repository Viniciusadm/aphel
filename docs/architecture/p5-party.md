# P5 party contract: first implementation slice

P5.1 introduces `aphel-contract-party`. It depends only on the contract primitives and serde, with no kernel, database, plugin, runtime or CRM dependency. `party.contact_points` describes contact fields on any entity. It does not prescribe a person/company record type or storage table.

`ContactPoint` identifies an entity field and its channel (`email`, `phone`, `web`). `ContactPoints::new` derives the ordered, unique `columns` projection used by existing summary consumers. Serialized metadata carries both the channel mappings and that projection. Deserialization accepts the old columns-only format as well as points-only metadata; typed points are authoritative when present. Legacy metadata without channel information does not guess a channel.

The native SDK exposes `PartyTraits` for publishing and reading the mapping. SQL-facing summary consumers receive only valid SQL identifiers. Generic phone/address field helpers and their existing bilingual labels have moved from the transitional CRM contract to `aphel_sdk::party`; field keys, Brazilian phone defaults and address labels are preserved.

## Connected providers and consumers

- `contacts` declares email, mobile and phone mappings and publishes the party contract descriptor.
- `companies` declares domain and email mappings and publishes the same contract descriptor as another provider.
- `duplicates`, `trash` and `privacy` read the shared SDK projection instead of the CRM trait constant.

The host catalog lists each provider. No provider depends on the other through the new contract. Existing direct dependencies and SQL between the native contacts/companies plugins are migration debt for P5.2, not resolved by this slice.

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

## P5.2 migration boundary

The next slice maps native module identities from `contacts`/`companies` to `people`/`organizations`, and entity identities from `contacts.contact`/`companies.company` to `people.person`/`organizations.organization`. Compatibility work must account for installed module state, migration history, role grants, stored entity references and configuration JSON before changing those identities. Existing API paths can remain compatibility routes while canonical entities change.

The current contacts crate also imports the company's `EmailEngagement` model, row mapper and not-found error and declares a direct module dependency. Its migrations contain company foreign keys, and its store reads company tables. Replacing only the crate import would leave the P5 dependency and schema gates unresolved. Contact/company linking must move to declared references and owner operations or entity queries, with the per-plugin acceptance cases above retained.

## Remaining scope

P5.2 still introduces the `people` and `organizations` plugin identities, migrates persisted identities and removes the corresponding direct dependencies. The remaining P5 contracts, capabilities, owner-provided sources and schema cleanup are pending. This slice does not claim that P5's zero-dependency/SQL/foreign-key gates have passed.
