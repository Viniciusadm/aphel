# Host catalog

What this Aphel offers to runtime plugins besides records: the hooks a manifest can subscribe to in `[[hooks]]` and the operations its backend can call with `invoke` after listing them in `operations`. The live list, including whether each module is enabled, is `GET /api/v1/wasm-runtime/host-catalog`. How they work: [README](README.md#module-operations-and-domain-hooks).

## Hooks

