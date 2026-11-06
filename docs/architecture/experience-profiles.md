# Experience profile contract, version 1

This is the data and resolution specification for the first experience increment. Organization catalog persistence and its API are implemented as described below. Personal preference persistence, server policy enforcement and runtime profile application are implemented. The editor is not implemented yet.

The contract covers two ways of working on the same authorized deal: the existing pipeline-oriented interface and a task-oriented interface that puts tasks before stage guidance. Backend rules, data ownership and the integrated frontend's trust requirements remain unchanged; the [field-policy boundary](validation-rules.md#personalization-limit) also applies to every nested profile property. The UI and write-path validations deferred in the execution checklist are still pending.

## Profile document

A profile is a closed JSON object with these required properties:

- `format_version`: the integer `1`. Other values are incompatible, not a request to guess a migration.
- `key`: an immutable, namespaced catalog identity, independent of the label or component function name.
- `label`: a nonempty plain-text catalog label, at most 120 characters. It is administrator-authored data, not executable markup.
- `frame`: an object containing exactly `module`, the provider identity for `app.Frame`.
- `record_detail`: one object containing `record_type`, `presentation` and, optionally, `regions`. Version 1 covers one exact record type per profile; no wildcard, trait selector or profile inheritance is serialized.

`record_detail.presentation` contains exactly `module`, the provider identity for `records.Presentation`. These two provider references select the contracts opened in steps 05 and 07. Step 12 replaces their temporary selection props and mode-based defaults with the shared profile resolution. A module must provide exactly one component for the requested contract. Neither registration order nor the last installed plugin breaks ties.

`record_detail.regions`, when present, contains exactly `main`: an ordered list of section sources. Each source contains exactly `module` and `key`, matching `RecordTab.source`. References are stored as pairs, not parsed from a concatenated string. `tasks:tasks` and `workflow:stage-guidance` are display identities for the pairs used in the examples.

The configurable `main` region describes the overview content of the alternative presentation. Fields, save handlers, required platform content and access controls are not section references. The support region remains owned by the presentation and uses the shared field model/layout; it has no configurable entry in version 1. Unlisted available sections remain accessible in the presentation's section navigation. Leaving a section out of `main` does not mean hiding it.

The standard `kernel` presentation accepts no `regions` override in this increment: omitting `regions` preserves its existing overview, tabs, summary and breakpoint. The alternative recipe accepts `main` with tasks followed by stage guidance. Any additional provider or region must publish compatible semantics and be reviewed before activation; an unknown region is an error, not an extension hook.

Profile keys use `[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)+`, at most 128 ASCII characters. Provider, record-type and section keys are nonempty ASCII identifiers using letters, digits, `.`, `_` and `-`, at most 128 characters. Profile documents are limited to 16 KiB of UTF-8 JSON, and `main` to 32 unique source pairs. Arrays preserve order and cannot contain duplicates. Unknown properties are rejected at every nesting level; `null` is not a substitute for an omitted optional profile property.

There is no generic `options`, HTML tree, script, binding, expression, import URL or function name field. Home, navigation priorities, command highlights, themes, view filters, dashboard widgets and field layouts are outside this version. The home/priority paths have implementations or proposed scenarios, but were not exercised interactively; they are not new profile settings on that evidence alone.

## Examples

### A: pipeline operation

```json
{
  "format_version": 1,
  "key": "org.pipeline",
  "label": "Pipeline operation",
  "frame": { "module": "kernel" },
  "record_detail": {
    "record_type": "deals.deal",
    "presentation": { "module": "kernel" }
  }
}
```

This selects the current standard providers. `/deals`, its views and stage board retain their existing owners. Opening the list is the scenario entry action; the document does not introduce a home preference or copy the list configuration.

### B: pending work

```json
{
  "format_version": 1,
  "key": "org.pending-work",
  "label": "Pending work",
  "frame": { "module": "frame-fixture" },
  "record_detail": {
    "record_type": "deals.deal",
    "presentation": { "module": "detail-fixture" },
    "regions": {
      "main": [
        { "module": "tasks", "key": "tasks" },
        { "module": "workflow", "key": "stage-guidance" }
      ]
    }
  }
}
```

This is a structurally valid development example using the actual identities from steps 05 and 07. It describes the task-oriented frame and the alternative's existing two-section overview. It does not claim those fixtures are production providers or that the profile is currently executable. In production they are absent and the references are unavailable.

The current dev loader enables frame and detail fixtures in separate Vite modes, so it does not yet make both available together. The detail fixture also hardcodes its primary sections; `RecordPresentationProps` currently has no regions input. Later profile application must supply the validated composition through the existing model/presentation boundary, preserve the standard path, and make the combined development scenario executable before treating B as an activated profile. No second registry or interpretation of arbitrary component props is required or authorized by this specification.

Legacy notes adaptation does not currently publish `RecordTab.source`. Its omission in the step 07 fixture therefore does not establish a stable `notes:notes` profile reference. Do not manufacture that identity from its tab label or registry sequence. An owner-provided stable source is necessary before persisted references can target it.

### Invalid documents

```json
{
  "format_version": 1,
  "key": "org.invalid-fields",
  "label": "Invalid field policy",
  "frame": { "module": "kernel" },
  "record_detail": {
    "record_type": "deals.deal",
    "presentation": { "module": "kernel" },
    "required_when": false
  }
}
```

Reject `record_detail.required_when` as an unknown, forbidden field-policy override. It must not be ignored, forwarded to a component or written to `record_layouts`.

```json
{
  "format_version": 1,
  "key": "org.invalid-composition",
  "label": "Invalid composition",
  "frame": { "module": "frame-fixture" },
  "record_detail": {
    "record_type": "deals.deal",
    "presentation": { "module": "detail-fixture" },
    "regions": {
      "main": [
        { "module": "tasks", "key": "tasks" },
        { "module": "tasks", "key": "tasks" }
      ]
    }
  }
}
```

Reject the duplicate source at `record_detail.regions.main[1]`. Further invalid cases are `format_version: 2`, a provider function path instead of a module identity, `regions.support`, regions on the standard presentation, an unqualified string such as `"tasks"`, `extends`, `dashboard_id`, `widgets`, `filter`, `visible_when` or nested executable options. Unknown versions require an explicit migration and review before activation.

## Organization values and personal adjustments

The organization owns the enabled catalog, one default profile and the customization policy. The administrator operates these values; there is no separate administrator override layer. A distributed preset becomes an organization-owned document through an explicit import/update operation. It is not another runtime inheritance level. Blueprint can initialize these values once; login, profile selection and preset updates do not repeatedly reseed resources.

The logical policy consists only of an allowlist of profile keys, one default key from that list, and limits for profile selection, existing navigation customization, section order and section omission. Policy restricts allowed adjustments; it is not a competing layer of profile values. The organization persistence and transport below implement this owner without creating another preference store.

Personal profile selection and section adjustments belong to the existing user-preference owner extended in step 11, not another preference store. They are independent values: selected profile key, optional ordered source pairs, and optional hidden source pairs for the configured record type. Do not persist a full effective profile or copies of organization defaults. Start with one adjustment per surface/record type, not a second set of every existing preference per profile.

Section order is a partial priority list: matching listed sources first, then remaining organization sources in organization order. Hidden sources are subtracted after ordering; an order entry never unhides a source. A hide request must be a subset both of the organization's allowlist and of the presentation/owner-reviewed optional content. Required field/platform content can never enter that set. For this increment, `tasks:tasks` is the only reviewed source eligible for personal omission, and only when the organization permits it; `workflow:stage-guidance` is not eligible for personal omission. The tasks owner contributes typed `PersonallyOmittableRecordSection` metadata through the existing contribution mechanism. The registry supplies the owner identity and disabled owners are excluded; mere section presence grants no omission permission. Workflow contributes no such metadata. Expanding it requires owner review, not a user-supplied flag.

The existing `navigation.home`, `order` and `hidden`, locale and keyboard-shortcut preferences retain their owner and values. There are no profile navigation defaults in version 1. If organization policy blocks navigation customization, resolution ignores those personal navigation adjustments but retains the stored values. Without an experience policy, the existing preference behavior is unchanged. Personal selection of frame and detail providers independently of the chosen profile is excluded.

A preference write rejects blocked, duplicate, out-of-profile or nonoptional section adjustments with an error at the offending path. Previously saved adjustments that became blocked or incompatible after an organization update are ignored for resolution, retained and diagnosed; do not erase them during reads or login. References for another profile/surface do not become active merely because that profile was selected previously.

## Deterministic resolution

Resolution consumes one catalog/configuration snapshot, organization policy, the user's existing preferences and allowed adjustments, active contributions and the current authorization context. It returns selected identities, effective composition and diagnostics; it never installs/enables plugins or performs business writes.

1. Validate the catalog documents and format versions. On future writes/imports, reject invalid documents atomically rather than partially accepting known properties. A later incompatible update must not replace the currently active compatible document. Invalid persisted input must not execute or be silently reinterpreted.
2. Choose the user's selected key only if selection is permitted and the enabled profile is in the organization's allowlist. Otherwise choose the organization's valid default and report why an explicit personal choice was ignored. New writes of a disallowed choice are rejected; old choices are retained. With no usable default/catalog, choose the technical application baseline: standard frame/detail and current module behavior, not a fabricated catalog profile.
3. Take the selected organization's canonical profile document as its values. Do not merge the preset, blueprint, teams, role and personal copies recursively. Roles still participate in authorization and the existing legacy home fallback, not profile inheritance.
4. Apply only permitted personal section order/hidden adjustments for that record surface, by identity as described above. Keep missing references in stored data. For a different record type, keep the profile frame but use the standard record presentation and that type's existing sections.
5. Resolve active providers and sections through the existing component/contribution system. Require exactly one active component for each selected module/contract pair. Honor record compatibility, visibility hooks, permission requirements and backend authorization. Reordering/omitting rendered pieces must not change the registry visibility-hook call order or trigger denied queries. Apply composition before building the page's available commands, so hidden/unavailable sections cannot leave stale section commands.
6. A missing, disabled or ambiguous frame/detail provider falls back for that surface to the corresponding standard `kernel` implementation and emits a diagnostic with the requested identity. Do not replace the saved selection with `kernel`, and do not choose a different installed provider. With a default detail fallback, use that presentation's own composition; do not pass unsupported alternative regions to it.
7. A missing, ineligible, denied or ambiguous section is omitted from effective content with a comprehensible diagnostic; other sections and fields remain usable. A default placeholder without the requested source is not a substitute. Preserve its reference for reactivation. Rendering errors use the existing section/presentation boundaries. A required journey/provider that cannot operate must not be reported as successful merely because a fallback record view is readable; version 1 does not serialize arbitrary journeys or required plugin installation plans.

Home resolution stays in the single existing `HomeRedirect` path. With a profile, an allowed personal home remains first; there is no profile home field, so the current plugin-role → blueprint → mobile fallback follows. If navigation customization is blocked, skip the personal home but retain it. With no profile/policy, preserve the exact current precedence. Replacing that resolver with a second redirect effect is prohibited. Views, widgets and dashboard resolution are not run again because the profile changed.

## Missing references, conflicts, versions and reset

Syntax validity, registry knowledge, runtime availability and authorization are distinct. A reference to a known but disabled provider/section is still a valid stored reference. A missing module is a dormant unresolved reference, not an instruction to fetch code. Activation/import review must show it; a complete experience cannot be advertised as available before its required providers and record context can operate. User-specific denial does not invalidate the organization document or grant access.

Duplicate catalog keys, multiple defaults, an unallowlisted default, duplicate section sources, multiple active providers for the same selected contract/module, and unsupported regions all have explicit diagnostic/error paths. There is no "first entry wins" for these conflicts. Catalog/preference shape, organization policy and adjustment authorization are enforced by the server; frontend filtering alone is insufficient. The server uses its installed-module/entity data and any reviewed published compatibility metadata; it does not import React or resolve component functions. Actual component uniqueness and effective rendered availability are rechecked by the host through the existing registry. Missing compatibility metadata must not be invented or reported as verified support. Step 09 defined this boundary; step 10 implements the closed data/catalog validation below, without introducing a frontend capability metadata mechanism or a backend component resolver.

`format_version` describes this closed grammar, not a plugin version. An unavailable plugin version and an incompatible data-format version are different failures. No generic JSON merge or implicit migration is specified. Validate a proposed replacement against the supported format and provider contracts before activation; keep prior valid state if replacement fails. Do not create profile-to-profile inheritance or a global backend component resolver.

Restoring the profile choice clears only the personal selection, returning to the organization's current default. Restoring section composition clears only that surface's personal order/hidden adjustments. Restoring existing navigation uses its existing owner; it does not clear profile selection or section choices. No reset writes to field layouts, schema, validation rules, themes, views, dashboard widgets or automations. Patch semantics distinguish omitted (unchanged) from explicit reset, following the existing preference DTO's approach to optional/null values; the transport semantics are specified below.

## Validation specification and review cases

Validation is layered: closed shape/version/limits first; identity uniqueness and record/provider compatibility second; organization policy and the current writer's right to adjust third; availability/authorization at resolution. A runtime outage does not cause a format migration, and a policy violation must not be accepted just because its provider is currently unavailable.

Review cases for the later implementation are:

- A selects exactly the two standard contracts and leaves native behavior unchanged. B selects the two existing fixture identities and orders the actual tasks/workflow source pairs, using one model/cache/mutation path.
- Reverse provider registration order; the same requested module still wins. Two contributions from that module produce ambiguity and the standard fallback, not last-installed selection.
- Deny or disable tasks; its content is absent and cannot query/render cached data as authorized content. Re-enable it and the saved source returns in order. Keep business-rule providers fixed when comparing write invariants.
- Ignore a now-blocked stored personal choice with a diagnostic, but reject the same choice on a new write. Reject a request to hide workflow or required field/platform content.
- Store source order `[workflow, tasks]` when permitted: remaining organization sources follow in their original order. Hiding an allowed optional source removes it without altering field/workflow requirements.
- Switch profiles and reset one surface. Other preference surfaces, views, widgets, fields and automations retain their existing values.
- Reject the invalid examples at their documented paths. Reject malformed/unsupported JSON before component resolution; never fetch an import URL or evaluate an expression from a profile.
- Report B's missing development providers accurately on this checkout. These are implementation/validation gaps to close in the profile application stage, not evidence of an existing production catalog.

These are validation requirements and document-review outcomes, not a claim that runtime/API tests were executed. The catalog API below implements step 10. Steps 11 and 12 add guarded personal preferences and frontend application below without broadening this format.

## Organization catalog API and persistence

The kernel's organization-profile owner stores a single catalog under `organization_settings.settings.experiences`. It is a sibling of `configuration` and `installer`; the existing general settings PATCH cannot overwrite this namespace. No new table or migration is needed. An existing installation without the key reads `configuration: null`, which represents the unconfigured application baseline. No catalog is seeded at login or when reading it.

Authenticated members can `GET /api/v1/organization/experiences`. The response uses the standard `{ data }` envelope and includes `configuration`, the organization row's `updated_at`, `requires_host_resolution` and diagnostics. Ordinary members cannot mutate it: `PUT /api/v1/organization/experiences` requires `settings.manage`. PUT replaces the complete catalog/default/policy aggregate, rather than updating independent references in separate transactions.

A configuration request has this closed shape; each `profile` uses the version 1 document above:

```json
{
  "profiles": [
    {
      "enabled": true,
      "profile": {
        "format_version": 1,
        "key": "org.pipeline",
        "label": "Pipeline operation",
        "frame": { "module": "kernel" },
        "record_detail": {
          "record_type": "deals.deal",
          "presentation": { "module": "kernel" }
        }
      }
    }
  ],
  "default_profile": "org.pipeline",
  "policy": {
    "allowed_profiles": ["org.pipeline"],
    "customization": {
      "profile_selection": true,
      "navigation": true,
      "section_order": false,
      "section_hidden": []
    }
  }
}
```

The aggregate is limited to 32 unique profile keys and 256 KiB of canonical UTF-8 JSON. Each canonical profile remains limited to 16 KiB. Unsupported versions and unknown properties are rejected, including nested field-rule or executable options; an explicitly null `regions` is rejected. The default, when non-null, must be enabled and allowlisted. Omitting `default_profile` is equivalent to null in this full-replacement PUT; it does not mean keeping the previous default. Allowlist keys must reference enabled entries and cannot repeat. `section_hidden` is a list of distinct source pairs present in a `main` region of at least one enabled, allowlisted profile. It is an administrator limit, not proof of owner-reviewed optionality: step 11 also enforces the reviewed optional-content restriction for every personal hide request. Saving a broad organization limit never grants a user authority to hide required content or change data permissions.

Removing or disabling a profile requires removing its allowlist references and replacing/clearing the default in the same PUT. References remaining in personal preferences are not deleted or rewritten. Changing a profile key is a removal and addition, not an implicit rename or alias inferred from its label. An empty catalog with `default_profile: null`, an empty allowlist and no hidden-source limits explicitly chooses the baseline; a null default is also allowed when the organization wants no profile applied by default. Navigation or other customization booleans remain explicit organization choices in either case.

PUT requires `If-Match` with the exact `updated_at` returned by GET (RFC3339; quoted values are also accepted by the existing extractor). Missing/malformed values return 422. The use case requires permission, validates the aggregate, then locks the organization row and compares the full microsecond timestamp. A stale value returns 409 `version_conflict`; no data, event or audit is committed. The same transaction updates only `$.experiences`, advances `updated_at` monotonically by at least one microsecond, publishes `organization.experiences_updated` and writes its audit diff. Equal configuration is a no-op after the version check. Unrelated organization writes can also invalidate this row-level version; clients must reload rather than silently merge.

Profiles are configuration references, not executed journeys. The server does not infer React provider support from module installation. `provider_verification_required` diagnostics are marked `required: true`; section checks use `section_verification_required` with `required: false`. This flag distinguishes unavailable required presentation infrastructure from degradable section content, and is not permission for personal omission. An unknown record type additionally yields `record_type_unavailable`. The frontend host must verify current provider uniqueness, compatibility, permissions and source availability before advertising a usable experience. References are retained; saving them does not fetch, install or enable any plugin. Development-only providers remain development-only even if their names appear in a stored document.

Read and write operations deserialize and validate stored catalog data; corrupt or incompatible persisted input raises a server error instead of silently resetting it or applying a partial configuration. The frontend consumes this API through the shared resolution described below. Personal selection/policy enforcement is implemented in step 11; step 12 applies the permitted references while retaining session services and existing home precedence. The existing database-free probe now checks valid/dormant catalog input and rejects unsupported versions, duplicate keys/sources, dangling references, nested field-policy overrides, explicit null regions and executable provider URLs. API authorization, transaction rollback, concurrent writes and persistence across a process restart still require validation against an authorized existing database; the probe, compilation and OpenAPI generation do not prove those runtime behaviors.


## Personal preferences API (step 11)

The existing authenticated `GET/PATCH /api/v1/preferences` remains self-only: the user ID comes from the authenticated context, never the payload. Closed request objects reject unknown properties. No new endpoint, query key, preference store, editor or provider selection path is introduced. Views, dashboard, field rules and organization resources are untouched.

The nullable `user_preferences.experience` JSON column is introduced by kernel migration `2843_add_user_preferences_experience.sql`. SQL NULL reads as empty personal experience preferences; existing navigation, keyboard shortcuts and `users.locale` retain their owners. The migration has not been applied to an installation during this execution.

PATCH omission preserves a value. Old payloads such as `{"locale":"en"}` preserve experience settings. `{"experience":{"profile":"org.pipeline"}}` changes only the personal profile reference. `{"experience":{"record_detail":{"record_type":"deals.deal","order":[],"hidden":[]}}}` replaces only that record surface adjustment; both arrays are required and bounded, and sources must belong to the selected/default profile's main region. Order is allowed only by organization policy; hiding additionally requires active owner-declared optionality. Required field/platform content cannot be targeted.

`{"experience":{"profile":null}}` resets only personal selection to the organization default. `{"experience":{"record_detail":null}}` resets only the record adjustment. `{"experience":null}` resets both experience choices. `{"navigation":{}}` resets only personal navigation. Resets remain possible under a restrictive policy. Changing a profile retains the saved record adjustment; incompatible references are ignored rather than rewriting another owner's state.

The write transaction locks organization settings with `FOR SHARE`, then the authenticated user's row with `FOR UPDATE`, merges supplied fields, validates policy, saves preferences and locale, and commits. Organization catalog replacement takes an exclusive lock, preventing policy changes during a personal write. Different users share the policy lock. Partial writes for one user serialize even before a preference row exists. These SQL behaviors still require database validation. Personal settings retain their existing absence of business events/audit entries.

Responses contain effective `experience` and `navigation`, plus `stored_experience`, `stored_navigation` and `experience_diagnostics`. Revoked, incompatible or blocked saved values remain in storage but are excluded from the effective projection. Unrelated patches do not resubmit or erase them; a fresh forbidden choice is rejected with field violations (422). Missing catalog configuration preserves existing navigation behavior and applies no experience preference. Corrupt catalog/storage data fails closed rather than silently assuming permissive policy.

The existing frontend preference cache remains shared. Consumers receive effective navigation; raw snapshots are informational. A server read re-evaluates policy, but this increment does not promise immediate invalidation of an open tab's five-minute cache after an administrator changes policy. Step 12 resolves frontend provider availability and applies the profile as described below. Database-free probe checks cover permitted/blocked choices, reviewed optionality, revoked references, old partial payload validation and reset permission; they do not prove HTTP authentication, persistence, locking or rollback.


## Frontend application (step 12)

`ExperienceProvider` is mounted once by the authenticated application shell. It observes the existing `['preferences']` query and loads the catalog once under `['organization', 'experiences']`; its context shares the chosen profile, current policy, effective navigation and frame. No other preference transport or module registry is introduced. Raw saved references are rechecked against the current catalog/policy, preventing mismatched cache snapshots from bypassing a newly restrictive policy. This does not make frontend checks an authorization boundary.

The routed content waits for catalog/preferences and module synchronization to settle online. A paused offline query permits the standard interface without waiting indefinitely for missing configuration; this adds no network listener or second offline cache. Mandatory language, theme, second-factor, notices, commands, PWA/offline and mobile services stay mounted outside the chosen frame. Read failure or an offline-paused initial query produces a diagnostic and the standard interface, with personal navigation withheld when policy cannot be loaded. Public and bare routes remain outside this provider; authenticated unknown routes reuse the same shell. No migration, backend schema, route registration or endpoint is changed in this step.

Home remains solely in `HomeRedirect`: permitted personal home, plugin role home, blueprint home, then mobile fallback. Its existing session marker and deep-link behavior remain intact; it waits for shared resolution and blueprint loading. A profile change does not launch a second home redirect. The existing sidebar arrangement function handles personal order/hidden values and locked entries, and also arranges the navigation model passed to alternative frames. There are no new profile navigation values, dashboard routes, view stores or command highlights.

Frame and record contracts select exactly one active contribution from the requested owner. Missing, disabled or duplicate providers use the corresponding standard component with a visible diagnostic. An error boundary also recovers frame rendering failures without remounting mandatory services. Record presentation selection only applies to the profile's exact record type; other record types retain the standard presentation.

A nonstandard record presentation declares reviewed compatibility on the contributed component's typed `experience` property: `recordTypes: readonly string[]` and `regions: readonly 'main'[]`, exported as `RecordPresentationComponent` through the existing SDK. Matching is exact, without inferring support from installation or registration order. A missing declaration, unmatched record type or unsupported main region falls back to the standard presentation. This is in-memory provider metadata, not a new registry or serialized executable value. Kernel presentation keeps its own composition and accepts no profile regions. The development presentation declares support for the example record type and main region; the backend does not resolve this component metadata.

`useRecordPageModel` retains the original hook/filter sequence and permission checks, then composes already eligible sections before registering section commands. Main priority is the permitted personal partial order followed by remaining organization main references. Hidden sources must also carry owner-authored `personallyOmittable` metadata on the effective section; only tasks declares it in this increment. Missing, denied or duplicate sources generate diagnostics and cannot be replaced by a default placeholder. Unlisted eligible sections remain accessible. The alternative presentation consumes `model.mainSections`; it no longer hardcodes task/workflow order. No source component is mounted merely to check its availability. Data hooks, field rules, save commands and resource cache keys retain their existing owners.

If the selected record renderer throws, the boundary renders the standard view with the original eligible tabs; subsequent model/command registration also returns to that baseline. A changed profile document/provider can retry the selected renderer. Provider failure or readable fallback is reported, never presented as evidence that the complete requested experience is available. Dormant references are not rewritten by resolution.

Development modes only register fixtures; they no longer select them automatically. `npm --prefix web run dev -- --mode experience-fixture` registers both example providers for an authorized existing installation whose catalog explicitly selects them. The prior frame/detail modes and failure fixtures remain development-only. Production builds omit these providers, so storing example B there produces explicit fallback diagnostics. No production experience package, admin/personal editor, preset importer or integration is implemented in this step.

Lint, TypeScript and builds check the wiring but do not prove the runtime criteria. Reload determinism, revocation, missing/ambiguous/disabled/reactivated modules, renderer failure, denied sections/no denied queries, command removal, deep links/home loops, desktop/mobile appearance and unchanged session/PWA/offline behavior remain in the execution checklist for an authorized existing environment. Cache freshness remains five minutes with normal query refetch behavior; no push invalidation or code unload is promised.

## Distribution transport

The version-1 catalog can now be reviewed through `POST /api/v1/organization/experiences/validate` (`settings.manage`, no writes). GET exports `.data.configuration` and the existing guarded PUT imports the same body. See [experience distribution](experience-distribution.md) for reproducible examples, dependency review, version distinctions, ownership-preserving upgrades and the separate admission of executable providers. No editor or personal-selection UI is included.

## Organization administration

The organization settings page at `/settings/experiences` now manages the existing catalog, shared default and permitted personal options. It stages edits/imports locally and requires validation plus an explicit review before PUT with If-Match. See [experience administration](experience-administration.md) for draft, availability, withdrawal and ownership boundaries. Personal selection/editor UI remains outside this step.

## Personal settings

Account settings at `/account/experience` now expose permitted selection, source ordering/omission and independent resets through the existing preference PATCH. Read-only omission choices reuse the server's active owner declarations and current profile/policy intersection. See [personal experiences](personal-experiences.md) for policy changes, retained references, the existing navigation editor and validation limits.

## Acceptance gate

The [acceptance evidence and remaining gate](experience-validation.md) consolidates A/B/I source trace, the public SDK authoring simulation, proposed request budget and the runtime checks still requiring an authorized environment. Source review and compilation do not complete those behavior criteria.

## Future scope decision

The [future decisions](experience-future-decisions.md) retain the current increment without approving further features. Named dashboards, backend-provider coexistence, less-trusted UI and any semantic field-policy migration require separately justified increments; the remaining options have no demonstrated need now. This documentary closure does not complete the outstanding acceptance gate.
