---
title: Publication
description: Publish the draft of a record as an immutable revision, now or at a time, with optional approval, withdrawal and history, using the optional publication module.
sidebar:
  order: 54
---

The optional `publication` module keeps the published content of a record apart from its draft. People edit draft fields; publishing captures them as an immutable revision and copies that revision into protected fields in one transaction. Approval policies, schedules, withdrawal, history and retention come from the module, so a plugin only declares which fields take part.

## Make an entity publishable

1. Give each published field a draft twin of the same type. People edit the draft; the published field is `read_only` and lists `publication` in `writers`, so only the owner plugin and the publication module can change it.
2. Add a state field (`select` with the options `draft`, `published` and `withdrawn`) and two `datetime` fields for the first publication and the last revision, all `read_only` with `writers = ["publication"]`.
3. Contribute the definition to `publication.publishable` and declare `uses.required = ["publication@1"]`.

```toml
[uses]
required = ["publication@1"]

[[permissions]]
key = "blog.publish"
label = "Publish posts"
grant_to = ["manager"]

[[entities.fields]]
key = "draft_title"
label = "Title (draft)"
type = "text"

[[entities.fields]]
key = "title"
label = "Title"
type = "text"
read_only = true
writers = ["publication"]

[[entities.fields]]
key = "publication_state"
label = "Publication"
type = "select"
default = "draft"
read_only = true
writers = ["publication"]
options = [{ value = "draft", label = "Draft" }, { value = "published", label = "Published" }, { value = "withdrawn", label = "Withdrawn" }]

[[contribute]]
point = "publication.publishable"
name = "post"
data = { entity = "blog.post", format = 1, permission = "blog.publish", fields = [{ draft = "draft_title", published = "title" }, { draft = "draft_body", published = "body", file = true }], state = "publication_state", published_at = "published_at", revised_at = "revised_at" }
```

`format` is the version of your snapshot layout. `file = true` marks fields that hold file ids (directly or inside JSON such as blocks); the revision keeps those files from being collected. A definition that breaks these rules is refused when used, with `409 publication_definition_invalid`.

Expose only the published fields on the site and filter on the state, for example `conditions = { publication_state = "published" }`.

## Publishing

| Step | Route | Operation | Who |
| --- | --- | --- | --- |
| Capture the saved draft | `POST /api/v1/publication/revisions` `{ record_type, record_id, expected_version, note? }` | `publication.capture` | update access to the record |
| Publish now or at `at` (UTC) | `POST /api/v1/publication/requests` `{ revision_id, at?, idempotency_key }` | `publication.request` | the `permission` of the definition |
| Withdraw | `POST /api/v1/publication/withdrawals` `{ record_type, record_id, idempotency_key }` | `publication.withdraw` | the `permission` of the definition |
| Cancel an open request | `POST /api/v1/publication/requests/{id}/cancel` | `publication.cancel` | the `permission` of the definition |
| Retry a failed request | `POST /api/v1/publication/requests/{id}/retry` | `publication.retry` | the `permission` of the definition |
| Copy a revision back into the draft | `POST /api/v1/publication/revisions/{id}/restore` `{ expected_version }` | `publication.restore` | update access to the record |
| Status, revisions | `GET /api/v1/publication/status`, `GET /api/v1/publication/revisions[/{id}]` | `publication.status`, `publication.revisions`, `publication.revision` | read access to the record |
| Status of a list of records | `GET /api/v1/publication/statuses?record_type=…&record_ids=a,b` (up to 100) | | read access; unreadable records are left out |

`expected_version` is the `updated_at` of the saved draft; a newer save answers `409`. Capture and request are separate on purpose: a request names an immutable revision, so approving or scheduling it never publishes later edits of the draft.

Every request or withdrawal increases the record's generation and replaces the open requests of the previous one. A request runs only while its generation is current and the published revision is still the one it saw, so an old schedule never overwrites a newer publication. Repeating an `idempotency_key` returns the first result; using it for another command answers `409 publication_idempotency_conflict`.

## Approval

The module adds the approval moment `publication.activate`. A policy for that moment on your entity makes every publication wait for approval of the exact revision: the request becomes `awaiting_approval`, the approvers receive the approval request, and approval publishes it (at the scheduled time, if any). Rejection, or a change of the policy before the decision, blocks the request; publishing again starts a new approval. Without a policy, people with the permission publish directly. If a configured active policy still exists while its provider is disabled, publication is blocked with `publication_approvals_unavailable`; enable the provider and submit again. Requirement discovery asks the policy owner for configuration metadata even while its decision service is disabled, without reading another module’s tables.

## What happens at publication

Publishing re-checks, in one transaction, that the requester is still active and holds the permission, that the policy is satisfied for this revision, and that no newer request replaced it. It then writes the published fields through the record pipeline (your hooks run and can veto), records the active revision and publishes `publication.activated` (`publication.withdrawn` for a withdrawal). A failed check blocks the request with a `reason_code` such as `publication_requester_unauthorized`; temporary errors are retried by the worker up to ten times and then mark the request `failed`.

Revisions that are not published, not waiting and not held by an approval are removed after 90 days, keeping at least the 50 most recent per record.
