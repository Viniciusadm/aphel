# Distributing experience configuration

Distribute the existing `OrganizationExperiences` JSON body, containing the profiles, organization default and policy. No executable installer, second plugin registry, independent SDK package or new package envelope is needed. The examples are [pipeline revision 1](examples/pipeline-r1.json) and [development work revision 1](examples/work-development-r1.json).

The pipeline example uses production `kernel` frame/detail providers and `deals.deal`. Install/enable deals and its declared prerequisites through the existing module cycle first. The work example also requires tasks, workflow and the two development fixture providers. Those providers are registered together only by the `experience-fixture` development build; they are absent from production. Its successful parsing is not evidence of a production-ready alternative experience. The existing probe validates both distributed documents and their serialization round trip without database or network.

## Export, review, validate and import

Use an existing authorized installation and an administrator with `settings.manage`. These commands are instructions, not actions run against an installation by the implementation. Supply authentication via a protected cookie file; do not put tokens in published examples. `APHEL_URL` includes the scheme and host only; use HTTPS outside local development. The CSRF header is required on writes with cookie authentication.

```sh
curl --fail-with-body --cookie "$APHEL_COOKIE_FILE" \
  "$APHEL_URL/api/v1/organization/experiences" > current-experiences.json
jq '.data.configuration' current-experiences.json > exported-experiences.json
```

A null configuration means there is no preset to export; keep the application baseline or explicitly choose a sample. Do not PUT `null`. The export body is the same schema accepted on import. It contains no personal preferences, compiled providers, theme assets, field layouts, saved views, dashboard instances or automation definitions.

```sh
curl --fail-with-body --cookie "$APHEL_COOKIE_FILE" \
  -H 'X-Requested-With: aphel' -H 'Content-Type: application/json' \
  --data-binary @exported-experiences.json \
  "$APHEL_URL/api/v1/organization/experiences/validate" > experience-report.json
```

Validation requires `settings.manage`, performs no configuration writes and uses the same closed DTO and semantic validator as PUT. A successful report describes the proposed configuration, the current organization `updated_at`, and dependency diagnostics. `record_type_unavailable` means the record context is absent. `provider_verification_required` and `section_verification_required` deliberately remain unresolved: the server cannot inspect React component compatibility or a viewer's permission-dependent sections. Check modules and the frontend host diagnostics before declaring the experience available. The development example must report fallback outside its fixture build; do not conceal that by installing similarly named empty modules. Dormant references are retained, never fetched as executable code.

Review the entire proposed catalog, default and customization policy against the exported current values. PUT replaces this aggregate, rather than merging presets by name. Preserve existing profile keys and policy references deliberately when combining examples with an existing catalog. Replacing or removing a key can change effective personal choices without deleting stored references. The preview version is not a reservation: obtain a fresh GET immediately before approval and compare it with the reviewed snapshot. A concurrent change after that GET produces 409.

```sh
experience_version=$(jq -r '.data.updated_at' current-experiences.json)
curl --fail-with-body --cookie "$APHEL_COOKIE_FILE" \
  -H 'X-Requested-With: aphel' -H 'Content-Type: application/json' \
  -H "If-Match: $experience_version" -X PUT \
  --data-binary @exported-experiences.json \
  "$APHEL_URL/api/v1/organization/experiences" > saved-experiences.json
```

GET again and compare `.data.configuration` with the reviewed body. Reimporting identical configuration with a current If-Match is a no-op. Invalid/unknown properties and future format versions fail with 422 before a transaction writes anything; stale If-Match fails with 409. Real writes retain the existing atomic organization JSON/event/audit transaction. Never strip unknown fields to force an unsupported version into version 1.

## Versions and upgrade ownership

`format_version: 1` is the structural profile format. The `r1` in the distributed example filename is its preset release revision; it is not a second field accepted by the JSON validator. A developer tracks subsequent preset revisions in version control and publishes the full reviewed JSON plus release notes. Keep profile keys stable across revisions. Package versions in `.aphelplugin`, theme versions, WIT/API versions and domain contracts such as `tasks@1.0.0` are independent. A matching preset revision cannot prove provider compatibility.

There is no older supported profile format to migrate: pre-profile installations have null configuration and retain baseline behavior. Reads do not rewrite them. A future format must add an explicit reviewed conversion before activating converted documents; the current importer rejects it without changing the existing aggregate. No downgrade coercion or speculative migration engine is supplied.

Upgrading configuration writes only the organization-owned experience aggregate through PUT. It does not PATCH personal preferences, reset navigation/locale/shortcuts, change saved views/dashboard/record layouts or publish an automation. Existing personal references and adjustments are retained; current policy and available providers decide their effective projection. Backend permission checks are unchanged. Cached pages may take the existing refresh interval or a reload to see policy updates; there is no push promise.

Blueprint integration is intentionally absent from these standalone JSON examples. Existing blueprints and configuration packages remain installation-time owners. No login/upgrade hook reapplies the preset or personal settings. Add an owner-contributed configuration section only if a concrete blueprint distribution later requires it; do not create an empty section merely to wrap an already supported GET/PUT body.

## Executable providers and themes

The samples ship no new executable bundle or theme. Existing tokens/themes retain their validated, code-free package and independent activation lifecycle. They need no frontend-code consent. Provider code, when actually needed, uses the existing `@aphel/web-sdk`, module registration, `app.provideComponent`, `APP_FRAME_CONTRACT`/`AppFrameProps` and `RECORD_PRESENTATION_CONTRACT`/`RecordPresentationComponent`. These exports already exist. The latter declares exact `experience.recordTypes` and supported `main` region. Do not copy host services, create a second React/router/cache instance or infer authorization from a component contract. The development fixtures are the concrete source examples; they are not `.aphelplugin` packages.

For a runtime executable provider, review its `.aphelplugin` separately with the existing admin `dry_run=true` upload. Identify package key, proposed version, installed version, declared capabilities, public routes/hosts, role grants and replacement conflicts. The package's self-reported name/version does not verify supplier origin or code safety. Only an actor with `system.manage` may admit code. An integrated frontend executes with the session of each user opening it, including DOM/input/storage and authorized API access; WASM backend capabilities do not confine that JavaScript.

Actual uploads with web code, including upgrades that remain disabled, require `acknowledge_web_version` equal to the manifest version. Missing/wrong admission returns 409 `plugin_web_unacknowledged` before schema application/storage. Dry-run is exempt. Re-enabling web code requires the installed version again before dependency/schema/module changes. The existing UI presents a separate unchecked trust acknowledgement and sends the reviewed version only after selection. Direct API clients must explicitly send the version too; personal profile PATCH/PUT never supplies this admission or invokes plugin installation. This is explicit administrator intent for the selected version, not a signed approval token, proof of a prior dry-run, content hash binding or isolation. Concurrent plugin upgrade/enable races are not serialized by this change.

Disabling affects future loading; reload open tabs to remove already loaded JavaScript/CSS. It neither reverses past effects nor guarantees termination of a blocked page. Existing web plugins already enabled before this change are not retrospectively disabled; new install/upgrade/reactivation requests must use the admission parameter. If integrated application-level trust cannot be granted, do not admit that provider; isolated UI remains a separate future decision.

## Remaining verification

Compilation and the database-free probe do not establish HTTP or runtime behavior. In an existing authorized environment, verify export/preview/reimport/no-op; future/unknown format rejection without writes; missing/disabled provider and source diagnostics; 401/403; If-Match concurrency and rollback; preset upgrade with raw personal values retained; web-only plugin admission, wrong-version refusal, disabled upgrade and re-enable; backend-only packages without web admission; separate conflict/role checks; and reload recovery. No temporary instance or real installation was modified to perform these checks during implementation.
