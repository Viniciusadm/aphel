# P5.10 Timeline sources

Timeline is an optional host over contributed sources and open, localized kind definitions. It authorizes the subject through the record engine, validates source pages, merges their entries and produces a stable global cursor. The pure timeline contract owns kind metadata; native SDK interfaces own source, event-scope and event-decoration ports.

## Static review

- [x] The timeline crate has no business-plugin implementation dependencies or SQL over their tables. Its built-in reader reads core event/user data.
- [x] Tasks and notes query their own storage and implement their own authorization, visibility and presentation policy. Task sources apply query constraints, serialization contributions and field masking, including location and linked-record visibility.
- [x] Parent read access is required for pages and kind discovery. Disabled native record owners are rejected through a shared SDK availability helper, also used by line items.
- [x] People and deals own organization roll-up selections and their record scopes. The kernel compiles declared scoped reference subqueries without business literals.
- [x] Duplicates owns merged-record event selection. Automations owns automation actor-name enrichment. Email owns restrictions and mailbox/content enrichment, including suppression when its provider is disabled.
- [x] Event restrictions retain all registered privacy decorators, while normal sources, kind definitions and related selections follow enabled-module contributions.
- [x] Source IDs are namespaced and bounded. Each source gets the same exclusive timestamp/ID cursor and at most `per_page + 1` rows; the host validates, filters, sorts and deduplicates before issuing the next cursor.
- [x] Event categories come from contributed definitions and registry-declared event prefixes. CRUD change categories use generic entity actions rather than CRM type lists. Structured owners suppress their duplicate raw event prefixes.
- [x] Generic event payload masking remains driven by the event record's field policy. Source-specific masking stays with the source owner.
- [x] The API and plugin operations expose localized kind metadata. The web host fetches those definitions, supports open keys and derives generic record-change labels from action and record metadata.
- [x] Timeline is selected by installer profiles and has no required flag. No old kind aliases, alternate routes or migration paths were introduced.

## Manual acceptance pending

No automated tests, probes or runtime instance were executed under D4. OpenAPI/catalog generation constructs the production route registry without starting a server or connecting to the database.

- [ ] Page through mixed event/task/note sources with equal timestamps; verify no duplicate or skipped entries across cursor boundaries, kind changes and the last page.
- [ ] Verify private notes, mailbox content levels, hidden task/location fields, related-record visibility and scoped organization roll-ups.
- [ ] Verify merged history and automation actor labels, including missing/deleted records and disabled contributors.
- [ ] Add a source/kind contribution and exercise namespace, kind, cursor and page-limit validation.
- [ ] Inspect native and administrator records in both locales, including dotted event prefixes, field-change labels and file events.
- [ ] Disable timeline independently and inspect installer profiles, record tabs and source/provider disablement.

The broader removal of implementation dependencies from contributor owners remains part of P5.22–P5.24. The timeline host itself no longer imports those implementations.
