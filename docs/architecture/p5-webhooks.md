# P5.17 Entity-driven webhooks

Webhooks owns incoming endpoint secrets, signature/replay checks, mapping, request logs and outgoing delivery. The pure webhooks contract owns generic create/upsert action data and its incoming reference kind. Entity owners supply their normal declarations and writable handlers.

## Static review

- [x] Incoming actions select entity keys and logical fields from the current scoped catalog. Native, managed and dynamic entities use the same action model.
- [x] Targets require an enabled owner, a writable handler and create permission. Upsert options additionally require read/update permissions; active editable fields exclude hidden, computed and read-only fields.
- [x] Upsert requires a mapped, filterable unique field with a nonempty runtime value. Scoped queries use the shared query engine and reject multiple matches.
- [x] Every mapped value preserves its JSON type and writes through the registered handler. Owner validation and authorization remain authoritative at execution time.
- [x] Mapping has explicit payload and earlier-action roots. Returned identities enable references without implicit contact/deal/task links.
- [x] Assignment uses the optional SDK selector, validates target eligibility and checks the editable owner field. Missing or inactive providers reject the action.
- [x] Webhooks no longer depends on people, deals, tasks, workflow, assignment or the transitional CRM contract. Its reference kind moved to its own contract; automations consumes that contract.
- [x] Scoped record writes are available through Ctx alongside its existing scoped record read operation. Incoming actions do not build application state.
- [x] Outgoing nested field redaction uses public-resource metadata. People declares its nested organization resource; webhooks has no business-type branch.
- [x] The web editor uses entity metadata and the open assignment slot. Fixed business actions, workflow/task selectors and the CRM template were removed without compatibility aliases.
- [x] Signature verification, endpoint permissions, replay checks, bounded payloads, per-action commits, audit/outbox behavior and request history remain owned by webhooks.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Validation uses compilation, strict lint, type checks, build and generated API/catalog metadata.

- [ ] Configure native, managed and dynamic targets under create-only, own/team and hidden-field policies in both locales.
- [ ] Inspect typed mappings, owner-required fields, unique upsert matches, ambiguous matches, missing values and concurrently delivered requests.
- [ ] Chain references using `$.actions[0].id`; inspect missing/future action references and partial failure history.
- [ ] Disable an entity owner or assignment provider after saving; inspect execution-time rejection and permission revocation.
- [ ] Inspect signature tolerance, token rotation, replay windows, partial results, retries and crashes between writes and log persistence. Replay handling does not establish exactly-once record writes.
- [ ] Inspect outgoing nested resource redaction, hidden custom fields, deleted resources, retries, retention and privacy cleanup.

Notification implementation utilities remain part of the shared-capability cleanup in P5.22–P5.24. No runtime acceptance is claimed here.
