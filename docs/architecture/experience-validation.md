# Experience acceptance evidence and remaining gate

This review covers the pipeline (A), work follow-up (B) and won-deal task integration (I) defined in the architecture checklist. It is a partial acceptance review after the personal-settings implementation. The complete experience is not validated: no authorized A/B/I browser, HTTP or database run, failure injection, reload persistence or request-count measurement has been performed.

## Environment and evidence boundary

The inspected checkout started clean at `d1f5c778`. Compilation/lint/probe/build at that revision passed as recorded in the checklist. A read-only listener inspection found a frontend on 127.0.0.1:5173 but no listener on port 3001, the backend target explicitly configured in `web/vite.config.ts`. Other local services/ports were not assumed to be this application's authorized validation environment. The first sandboxed `ss -ltn` could not open netlink; the approved read-only rerun produced the listener inventory. This does not establish that every possible remote deployment is unavailable.

Starting a server can apply migrations on boot. Creating the organization/users/records, importing profiles, publishing an automation, changing module state, resetting preferences or injecting failures would change an installation. Those actions were not performed under the prohibition on scratch instances and real-installation changes. There are no captured scenario IDs, authenticated sessions or before/after request baselines to substitute for that missing evidence. No temporary instance, test suite or smoke run was introduced.

## What source review establishes

A and B share `RecordDetailPage`, `useRecordDetailData`, `RecordPageLayout` and `useRecordPageModel`. The loader uses the existing offline-aware record/schema query keys. Both presentations receive the same record/model, field save path, field-layout rules, permission-filtered contributions and commands. `AppLayout` keeps authentication-adjacent services, 2FA, language, mandatory notices, PWA and offline shell outside the selected frame. The development alternative does not create another router, query client or record loader. These are source facts, not evidence that both journeys render and authorize correctly in a browser.

B remains a development-only fixture. `web/src/app/modules.ts` imports its providers only under the existing development modes. A production build does not contain a deployed alternative work experience merely because its JSON validates. Source omission/order occurs after contribution visibility/permission filtering; commands derive from composed tabs. The tasks contribution checks `tasks.read` before enabling its existing activity query. Record-specific visibility and the actual absence of denied network calls still require execution.

Profile version 1 and the organization aggregate remain closed declarative DTOs. Configuration-null preserves baseline behavior; user SQL-null experience is read as empty. Reads project blocked/retained preferences rather than deleting them. Preference writes lock organization policy and user state and revalidate current permissions/options. These code paths do not prove migration, SQL rollback, concurrent update behavior or persistence after restart. No previous incompatible profile format is silently migrated.

The native integration uses the existing automation run/job identity and tasks adapter receipt. Task insert, audit/outbox and mutation receipt use the producer transaction; a successful receipt replay returns the same task ID. The worker reloads state under its lock and rechecks pause/cycle/depth/rate protection. Required permissions and active publisher authority remain separate from the incoming event actor. Review cannot certify a crash/retry/duplicate-delivery run, external hook effects or universal exactly-once semantics.

Integrated frontend trust is explicitly application-level. Preset and preference endpoints do not install code. Web plugin upload/upgrade/reactivation requires the reviewed version parameter under `system.manage`; the existing dialog separates that admission from backend capabilities. This is administrator intent, not sandboxing, supplier authentication, content-signing or a guaranteed rollback/termination of already loaded code.

## Small-extension authoring simulation

[small-extension.tsx](examples/small-extension.tsx) is a 20-line example consuming only `@aphel/web-sdk`. It contributes an authenticated informational route and a display renderer for the existing text field type, with both locales. The renderer is a presentation override, not a new field-storage type. If registered, it would affect text display wherever that contributed renderer applies; the example is not registered in this checkout or shipped in the production build.

The author needed `defineModule`, a module key/translations, `app.routes`, `app.fieldType` and the ordinary render props inferred by the SDK. They did not need an experience catalog, frame/detail contract, custom registry, personal-policy implementation, copied record loader or new backend runtime. Real business data would still require its existing authorized API; this informational example deliberately does no data access. This narrow exercise demonstrates compile-time simplicity, not a timed independent-developer study or installed-plugin lifecycle conformance.

Reproduce the compilation from the repository root with the existing installed tooling:

```sh
web/node_modules/.bin/tsc --project docs/architecture/examples/tsconfig.small-extension.json --noEmit
```

The example config extends the application's compiler options and maps the documented public SDK import to its host source, as the current plugin development workflow requires. It creates no independent SDK release and does not execute or register the example. The installed TypeScript 7 package provides the CLI; an initial attempt to use its absent JavaScript compiler-API entrypoint failed and was replaced by this successful CLI approach, without downloading dependencies.

## Controlled acceptance run still required

Use one company per installation. Prepare an existing authorized validation environment with admin, operator and reader roles; give the operator the intended record/task scope, and keep a negative-control account without task read/create. Record the installation/build revision, module/provider versions, organization catalog version, real deal/pipeline/activity-type IDs and automation/publisher IDs before the run. Do not equate role names with permissions. Use separate existing installations only if distinct organizations are needed.

1. **A and B with the same records.** Load the pipeline and linked task routes as operator and reader. Open the same deal, compare fields/actions/required-entry errors and perform a permitted transition. Capture UI results and direct-API refusals for the reader. In the authorized development fixture build, explicitly configure B and repeat with the same IDs, tasks before stage guidance. Do not call B production-ready from this result.
2. **Configuration and preference lifecycle.** Export the current aggregate, preview/import/reimport with current If-Match, and compare saved output. Confirm null/old preference defaults and unchanged navigation/locale/shortcuts/views/dashboard. Check unsupported/unknown formats without writes, stale version conflicts, reload persistence and independent selection/composition/navigation resets. Retain raw blocked references while changing the effective result. Test concurrent organization PUT and personal PATCH, including a tampered blocked option; the server must refuse it.
3. **Availability and access.** Disable/re-enable optional owners and inspect retained references, diagnostics, commands and requests. Check a missing/duplicate/incompatible required frame/presentation: readable default fallback must not be reported as a functioning requested journey. Exercise record-specific denial without mounting/querying denied content, authenticated/public/bare/unknown routes, 2FA, offline, and service availability. A version-1 profile is not a general mandatory-journey manifest.
4. **I and recovery.** Resolve the real activity type in `deal-won-follow-up.json`, explicitly create/review/publish through the existing automation API, and capture event/run/step/task IDs. Duplicate the same event in controlled tooling, repeat after a failure before commit, after an ambiguous commit acknowledgement and after task commit/before step recording. Confirm one task/audit/outbox effect for the same key and another task only on a new win event. Verify publisher inactivity, permission revocation, pause, missing participants and cycle/causation protection. No fault-injection endpoint or automatic replay command is invented by this guide; controlled instrumentation needs separate environment authorization.
5. **Administration and trust.** Confirm ordinary accounts cannot use administrative operations, failed validation never reports saved success, and withdrawal/default changes affect the organization as explained. Test web-only install/upgrade/disabled upgrade/reactivation with missing/wrong/correct admission, backend-only exemption, conflicts/grants and open-tab reload behavior. Personal settings must never install code.
6. **Interaction.** Complete all relevant paths by keyboard and on a narrow/mobile viewport; check labels, focus restoration, dialogs, blocked-control explanations, loading/offline/error states and save failures. Record any deviations as failures, not as deferred successes.

Each result needs expected/actual behavior, role, real IDs, revision, timestamps and request/effect evidence. Reuse the existing guides for [administration](experience-administration.md), [personal settings](personal-experiences.md), [distribution](experience-distribution.md) and [follow-up recovery](deal-won-follow-up.md). They state owner/reset/version/trust boundaries; none substitutes for executing this gate.

## Request budget and measurement

There is no observed A/B request or latency baseline. The following is the acceptance budget for a controlled comparison, not measured performance:

- Keep one authenticated React/router/cache stack and the existing preference/catalog query keys. Within the same fresh-cache/focus/retry conditions, changing only the presentation must add no independent record/schema loader or duplicate preference/catalog transport.
- For the same active section set and mount/cache state, B should add zero record/schema fetches relative to A solely because of composition. Additional requests must correspond to an actually newly mounted/selected section, not a duplicate host. Each in-flight task query for the same existing activity key should share the cache request; default-stale remount refetches must be counted explicitly rather than hidden under a universal one-request promise.
- Hiding/denying a section must not initiate that section's previously unmounted denied data request or leave its commands available. Existing permitted background refetches and already-mounted requests are recorded separately.
- Capture cold navigation, warm reopen, profile switch, section-order change, hide/show, permission revocation and save failure with the same record IDs and network conditions. Record endpoint/key, count, status, bytes, mount/refetch reason and timing; compare A/B medians only after collecting a real baseline. No fabricated latency threshold, SQL count or improvement is claimed.

A static read confirms shared query keys and no query hooks in the alternative frame/presentation, but cannot measure deduplication, mount/refetch timing, listeners in a browser or database workload. Unexplained extra host/record requests, duplicated listeners or denied requests fail the gate and must be investigated within the existing mechanisms before expanding scope.

## Gate decision

The authoring compilation and source trace are complete. Production-code checks at `d1f5c778` passed, and the added developer example is independently typechecked. A/B/I runtime acceptance and performance observations remain blocked by the environment/authorization conditions above. Objectives 04–08 and 10–17 retain their outstanding behavior criteria; the documentary objectives remain documentary. There is no basis to mark objective 17 complete or to start step 18.
