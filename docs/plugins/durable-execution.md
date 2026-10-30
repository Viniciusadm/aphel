# Durable execution and hook state

The unreleased `aphel:plugin@1.0.0` contract now includes `defer-effect`, `hook-state-json`, and `execution-context-json`. Rebuild Rust and JavaScript plugins against the matching SDK and reinstall the resulting package. Existing backend components that lack the new imports must be rebuilt before using these capabilities.

## Transaction boundaries

Record hooks run while the host owns the record transaction. Immediate record writes, writing module operations, key-value mutations, notifications, emitted events, and outbound HTTP are refused in this context. A hook may return field patches and may request durable effects instead.

`defer-effect(key, command-json)` returns an intention identifier. In a transactional hook, the host collects intentions and writes them together with their jobs through the original transaction. A veto, trap, timeout, later validation failure, or rollback discards the intentions. The return value does not mean the derived mutation has run.

After commit, workers execute intentions independently. Each supported mutation commits its receipt in the same MySQL transaction as its data, domain events, audit entries, and further intentions. A failure before commit rolls everything back. A crash after commit can leave the intention marked pending, but the next worker finds its receipt and completes bookkeeping without repeating the mutation.

The original record and the scheduling of its effects are atomic. The original record and the derived records are **eventually consistent**, and multiple intentions are independent transactions. Failed intentions do not roll back an already committed original operation. The queue applies its normal backoff and attempt limit; operations administrators can inspect and retry failed jobs while retaining the intention identifier.

Event deliveries are persisted separately for each plugin. A failure in one plugin does not restart plugins that already completed their delivery. Event and schedule retries reuse their execution identity. Immediate mutating host calls are refused with `plugin_effect_required` in retryable event, schedule, and automation callbacks; migrate these calls to explicit durable effects. Private routes may still perform immediate writes, but callers must not infer request-level idempotency from that API.

`record.committed` is a durable callback, scheduled inside the original transaction. It observes the snapshot from that operation even if another write happens before delivery. It cannot veto a commit that already happened.

## Commands and keys

JavaScript:

```javascript
ctx.deferEffect('create-followup', {
  kind: 'create',
  record_type: 'contact',
  values: { first_name: 'Follow-up' },
})
```

Rust:

```rust
ctx.defer_effect("create-followup", json!({
    "kind": "create",
    "record_type": "contact",
    "values": { "first_name": "Follow-up" }
}))?;
```

Commands use snake_case JSON in both SDKs:

- `create`: `record_type`, `values`.
- `update`: `record_type`, `id`, `values`.
- `delete`, `archive`, `restore`: `record_type`, `id`.
- `kv_set`: `collection`, `id`, `value`; `kv_delete`: `collection`, `id`.
- `emit`: `name`, `record_type`, `record_id`, `payload`; optional record fields are null or omitted together.
- `notify`: `name`, `recipient_user_id`, `title`, optional `body`, `record_type`, `record_id`, `link`, `data` and `dedupe_key`.
- `invoke`: `operation`, `input`.
- `email`: `to`, `subject`, `text`, optional `to_name`, `html` and `purpose` (`transactional` or `marketing`).
- `http`: `method`, `url`, optional `headers` and `body`.

Keys have 1–128 bytes and identify an effect within the plugin execution. Reusing a key with an identical command deduplicates it; changing the command produces `plugin_effect_key_conflict`. Include the hook name and record identifier when a logical execution can visit several hooks or records. Commands are limited to 64 KiB; a hook can collect at most 100 intentions.

Durable commands cannot provide a record result synchronously. Use the eventual record event for rules that need the result. Do not include timestamps, random values, or fresh external responses in a retry's command unless they are stable for the logical execution.

## Hook snapshots

Use `ctx.hookState()` in JavaScript or `ctx.hook_state()` in Rust. Outside a hook, the result is null.

The object contains `operation_id`, `record_type`, `record_id`, `phase`, `before`, `before_available`, `proposed`, `persisted`, and `effects_allowed`. Each available snapshot has `values` for registered system fields and `custom_fields` for custom fields.

- During `record.saving`, `before` is captured through the transaction before mutation; `proposed` overlays the draft and preceding filters. Create has an absent `before`.
- During validation, `proposed.values` comes from the module’s write descriptor and `proposed.custom_fields` comes from the writing transaction after normalization. This also works for modules that validate before storing native columns. Saved callbacks refresh the snapshot through the writing transaction.
- Deletion captures `before` before removing the record; its proposed state is absent.
- `record.committed` receives the operation snapshot as `persisted`, rather than a new read of whichever version is latest at delivery time.
- Domain hooks and mutations without a saving phase may not supply a previous snapshot. They explicitly set `before_available = false`; null is not an instruction to reconstruct missing state by querying the database.

`get-record` and `list-records` remain independent persisted reads. They do not silently overlay drafts or pending writes and do not promise to see uncommitted data. Use the hook-state API for validation of the current operation. Different records written in one transaction have separate snapshots under the same operation identifier.

The host applies field visibility to snapshots, legacy hook values, and change lists. Saving filters cannot change hidden, inactive, or noneditable fields. Read-only widget calls cannot produce immediate or deferred effects.

## Authorization and causality

The host records the plugin as executor for plugin mutations, domain events, and audit. The execution context separately identifies the initiator, trigger, parent execution/event, correlation identifier, and plugin chain. A scheduled execution has no synthetic human initiator: the installer supplies authority, not authorship.

Plugins cannot set these fields by sending similarly named JSON properties. Deferred commands retain the authority ceiling from scheduling and are reauthorized against the active user, current permissions, plugin capabilities, team membership, and original API token validity at execution time.

Plugins do not consume events from their own causal chain. Chains are limited to 16 plugins. Reentrant hooks may still validate and transform a derived write, but cannot enqueue another round of effects; inspect `effects_allowed` and only defer when true. This prevents direct and cross-plugin feedback loops without bypassing validation.

Referencing another record type requires its explicit read capability. Missing type-level permission produces an authorization error before the identifier is queried. An absent record and a record hidden by row-level permissions remain indistinguishable, preventing existence disclosure. Declared record-field targets are checked during package review, installation, and activation.

## Module contracts

A record handler opts into deferred mutations with `RecordTypeHandler::durable_mutations()`. A module operation opts in with `PluginOperation::with_durable_mutations()`. Unsupported operations are refused before execution.

The contract requires exactly one committing transaction using `Ctx::begin()` or the kernel transaction manager. A rolled-back transaction may be restarted; nested transactions and additional commits are refused. All business mutations, events, audit entries, and durable effects must share that transaction. Do not opt in code that writes through unrelated connections or performs external side effects. Third-party modules follow the same contract without adding dispatch cases to the kernel.

## Catalog readiness

Metadata loads independently of WASM compilation. The host publishes a complete metadata generation and compiles each backend lazily on first execution. Async module contexts ensure catalog readiness before consuming dynamic contributions. A loading or failed catalog is not represented as an authoritative empty list.

`GET /api/v1/plugins/readiness` requires `system.manage` and reports `loading`, `ready`, or `degraded`, the metadata version, and plugin keys with known backend errors. Catalog loading failures return 503 `plugin_catalog_unavailable`. Plugin settings displays loading and retryable errors. A broken backend remains subscribed to validation hooks and fails closed; it is not silently omitted.

Metadata refreshes are serialized. A generation changed during loading is rejected and retried rather than published. Notification kinds, widgets, and public events are replaced together. Transient database failures while loading a backend are retryable; a compilation failure is cached for that package version.

## External effects

The `http` and `email` commands deliver through a durable intention, so a retried callback does not schedule a second send. The host still cannot guarantee a single external side effect without a cooperating recipient: a crash after the remote response but before the local receipt repeats the call, so recipients must deduplicate. Do not use immediate HTTP from transactional hooks.

## Queued module delivery

`email.send`, `quotes.send`, `esign.send` and `esign.cancel` commit their delivery intention and job in the mutation transaction. The receipt covers queue acceptance, not delivery by the external provider. E-mails return `scheduled`; signature requests return `queued` or `cancel_pending`. Completion is reported by the existing sent/cancelled events. Workers retain the original authority ceiling and execution context, rechecking the active user, API token, teams, permissions, module enablement and plugin capabilities before delivery.

Signature workers claim a request before contacting Clicksign and persist `delivery_unknown` until confirmation is saved. They do not repeat an uncertain provider call automatically. After a crash or timeout, check Clicksign before submitting another request. A resumed e-mail delivery found in `sending` becomes failed with an explicit unknown-outcome reason instead of sending again. Provider errors known to be retryable retain the existing e-mail retry policy. Queue durability cannot guarantee exactly-once SMTP or Clicksign delivery.

`scoring.signal` commits its signal and a recalculation job together. Scores update asynchronously. Quote-to-deal item replacement validates all items before replacing them in one transaction. Lead conversion creates its optional company, deal and lifecycle transition atomically; record committed hooks run only after the shared commit. Automation distribution changes the owner in the assignment transaction. Rescheduling updates its activity in the booking transaction, using the same savepoint policy as cancellation.

## External queries

`ai.complete`, `ai.summary`, `ai.next_actions`, `ai.compose` and `enrichment.lookup` use `PluginOperation::with_external_query()`. They can be invoked synchronously from event, schedule and automation callbacks as well as private routes. They remain forbidden inside transactional hooks and read-only widgets and do not qualify as deferred mutations.

The host persists a claim before running an external query. A completed result is reused for identical operation/input pairs within the same logical plugin execution. Input hashes are stored instead of prompts; result JSON is retained with the plugin and deleted on uninstall. Replays with a changed authority fingerprint are refused. A query that started without a completed result is not repeated automatically (`plugin_query_outcome_unknown`); a new execution is required after checking the outcome. This prevents retries from silently making another provider call or charging another AI request. It does not recover an external response lost before local persistence.
