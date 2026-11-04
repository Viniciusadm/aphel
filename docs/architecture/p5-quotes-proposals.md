# P5.21 Quotes and proposals through capability interfaces

Quotes owns commercial quote state, versioned item snapshots, sharing/acceptance and document content. Proposals owns templates, generated-document metadata and rendering inputs. Catalog, pricing, approvals, record writes and outbound email are supplied by their respective owners.

## Static review

- [x] Quotes has no peer implementation dependency. Proposals no longer depends on tasks, deals or quotes implementations; its remaining configuration utility boundary is tracked in P5.22.
- [x] Pricing requests/list summaries/bundle snapshots belong to a pure pricing contract. Native resolver, bundle and discount interfaces live in the SDK; pricing retains tier selection, configuration and persistence.
- [x] Approval moments/status/events/default-policy data use the pure approvals contract. Scoped transactional gate interfaces and request projections use the SDK; approvals retains evaluation, decisions and persistence.
- [x] Quotes contributes its default discount-policy data. Approvals writes its own policy/step tables and detects duplicate contributed policy keys. Quotes no longer seeds foreign tables.
- [x] Email addresses and enqueue command data use the pure email contract. Quotes submits through the scoped transactional SDK sender, implemented by email's normal compose engine.
- [x] Quotes consumes catalog snapshots through the existing item-catalog capability. Bounded batches reject unrequested/ambiguous identities and deduplicate input IDs; the native catalog implements batch reads.
- [x] Catalog snapshots respect owner read access and hidden fields. Unavailable required catalog fields reject lookup; hidden codes are omitted. Pricing bundle expansion consumes the same snapshots rather than reading catalog tables.
- [x] Quote/proposal record reads and deal updates use Ctx owner-handler operations, including native owner availability checks. Proposals reads quote detail through the registered quote handler.
- [x] Quote related names use scoped generic record labels; only the core user directory remains in the quote store. Workflow stage membership is checked through the workflow catalog.
- [x] Decimal input wire metadata, random share-key encoding and HTML escaping are shared SDK utilities. Their business-token workflows, validation and sending behavior remain with the owners.
- [x] New contracts are registered in the capability catalog and contain no kernel/peer implementation dependency. Schema creation is unchanged here; foreign-FK removal remains in P5.24.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification uses compilation, strict lint, type checks/build and CLI API/catalog generation.

- [ ] Create/reprice/version quotes with direct items, quantity tiers, bundles, recurrence, discounts/taxes and multiple currencies; inspect inactive/missing catalog references.
- [ ] Inspect catalog read restrictions, hidden prices/names/codes, overlapping provider identities and missing pricing/catalog providers.
- [ ] Seed the default discount policy on a fresh installation; inspect duplicate contributions, existing customized policies and approval request/decision/cancellation flows.
- [ ] Send quotes with authorized/unauthorized mailboxes, recipient scopes, missing providers and transaction rollback during enqueue.
- [ ] Inspect public sharing, acceptance, expiration, PDF content, related-title visibility and deal synchronization when providers are disabled or records are deleted.
- [ ] Generate proposals from generic, deal and quote inputs; inspect field masking, optional plugins, locale/formatting and file authorization.

The default approval template interface currently supplies filter-based policies; policy-management behavior remains owned by approvals. Remaining shared configuration boundaries, domain vocabulary, frontend imports and the complete foreign-reference/deletion audit are tracked in P5.22–P5.24. Runtime acceptance has not been performed.
