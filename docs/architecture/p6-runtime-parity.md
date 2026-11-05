# P6 manual acceptance

What compilation, clippy, the arch check and `make probe` cannot prove. Run it on a development instance with the `wasm_runtime` module enabled and two test plugins (call them `provider` and `consumer`; the reference plugins `aphel-probe` and `aphel-hello-js` can be extended). Tick each line when it passes.

Not compile-verified and therefore part of this list: the Rust and JavaScript guest SDK edits (`on_operation`, `on_contribution`, generated `operations` and `domain_hooks` constants).

## Provided operations (P6.1, P6.3)

- [ ] `provider` declares `[[provides.operations]]` (one `read`, one `write` with a `permission`) and implements `on_operation`. Install shows no validation error and the consent dialog lists it.
- [ ] After enabling, the operations appear in `aphel catalog` and in `GET /api/v1/wasm-runtime/host-catalog` within about 2 seconds.
- [ ] A native plugin reaches a provided operation with `ctx.call`; the output matches what the guest returned.
- [ ] `consumer` lists the key in `operations` and reaches it with `invoke`; without listing it, `invoke` answers 403 `plugin_operation_not_declared`.
- [ ] A caller without the declared `permission` is refused; a `read` operation cannot write records; a `write` operation is refused inside a hook (409 `plugin_write_in_hook`).
- [ ] A cycle (`provider` operation that invokes `consumer`, which invokes `provider`) fails instead of hanging.
- [ ] A plugin that provides a key owned by a native module is refused on install and upgrade; a second plugin providing the same key as `provider` is refused on install.
- [ ] Disabling `provider` makes `ctx.call` answer `contract_unavailable` and removes the key from the catalog; enabling restores it; uninstalling removes it for good.

## Contributions and declared hooks (P6.3)

- [ ] A `[[contribute]]` entry is accepted, appears while enabled and is gone after disable. `on-contribution` runs with the restricted scope (`App::runtime_contributions(point)` called from a native test point).
- [ ] `[[hooks.declare]]` with a key outside `<plugin key>.` is refused; a valid one appears in the host catalog and another plugin can subscribe with `[[hooks.subscribe]]`. Firing from a backend is not implemented, so nothing triggers it yet.
- [ ] Existing plugins migrated from `[[hooks]]` to `[[hooks.subscribe]]` keep receiving `record.*` and domain hooks.

## Contracts and dependents (P6.4, P6.5)

- [ ] `consumer` with `[uses] required = ["x@1"]` and no provider is refused on install, upgrade and enable with 422 `plugin_contract_unavailable`; with `provider` providing `x@1` it installs; `x@2` does not satisfy it; an optional contract never blocks.
- [ ] With both enabled, disabling, uninstalling and upgrading `provider` to a version without `x@1` (or without the operation `consumer` lists) are refused with 409 `plugin_has_dependents` naming `consumer`; after `consumer` is disabled they succeed.
- [ ] A plugin that was already broken does not block unrelated changes.

## Module independence (P6.7)

- [ ] A build with `wasm_runtime` removed from `aphel.modules.toml` starts, serves the web app and shows no plugin screens. (Backend `cargo check` and web `typecheck` already pass without it.)

## Moved hook forwarding (adapter removal)

For each hook, subscribe a test plugin and confirm it is called with the documented `values`, that a veto blocks, and that filter patches apply:

- [ ] `deal.stage_changing`, `deal.winning`, `deal.losing` (veto blocks the move); `deal.won`, `deal.lost` (action, inside the transaction).
- [ ] `lead.qualifying`, `lead.disqualifying` (veto); `lead.converting` (a returned `{ deal, company }` patch changes the created deal and can force a company); `lead.converted`.
- [ ] `quote.accepting` (veto blocks public and manual acceptance).
- [ ] `activity.completing` (veto), with `activity`, `previous` and `type_key`.
- [ ] `email.outgoing` (a returned `subject` and `body` patch applies, html is sanitized, an invalid patch answers 422); an unlinked e-mail uses record type `email`.
- [ ] `sequence.step_running` (a definitive error removes the contact, a transient one retries).
- [ ] `automation.action_running` (an error skips the action; the record type seen is the record's).
- [ ] The native operations that moved with these files still work from `invoke`: `leads.qualify|disqualify|convert`, `deals.move_stage|move_pipeline`, `email.send|link|unlink`, `quotes.*`, `sequences.*`, `tasks.complete`.

## Gate

The P6 gate in the checklist can be ticked when a WebAssembly plugin provides an operation or contract a native plugin consumes **and** a native provider is consumed by a WebAssembly plugin, both from the lists above.
