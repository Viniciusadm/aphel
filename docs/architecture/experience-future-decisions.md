# Experience increment: future decisions

Date: 2026-11-06. Evidence baseline: `c4a8f48f`, following the implemented profile/admin/personal settings and the [partial acceptance review](experience-validation.md). This is a documentary scope decision, not production acceptance or authorization to implement the alternatives below.

## Decision and evidence limits

Keep the modular monolith and the shared React/router/cache host, version-1 declarative profiles, existing component/contribution contracts, organization-owned catalog/default/policy and scoped personal preferences. Keep themes code-free and data owned by views, dashboards, field layouts and domain/capability plugins. The won-deal follow-up remains an ordinary automation, not another integration runtime.

No additional feature is approved by this decision. Complete the outstanding A/B/I acceptance and request observations before claiming that the increment meets its behavioral or cost targets. The work experience B remains a development fixture; JSON validation and a readable fallback do not make it a deployed production experience. The 20-line public-SDK example compiled, demonstrating one narrow route/renderer authoring path, not a measured independent-developer or installed-plugin study.

The seven options are evaluated separately below. “Not needed now” means the current recorded scenarios supply no demonstrated need for that expansion. “Requires its own increment” establishes a boundary and entry conditions, not an approved backlog commitment. Missing demand and missing runtime evidence are recorded rather than replaced with invented use cases or performance claims.

## Configurable command highlighting

**Real use case and reusable mechanism:** A/B use existing contextual commands, creation actions, navigation and record-section commands. The shared palette and module `command` contributions already own command identity, availability and execution. There is no recorded operator request or observation establishing a need for profile-specific highlights.

**Established limit:** version-1 profiles contain no command-highlighting field. Command grouping/shortcuts and composition-aware record commands do not constitute an organization/personal highlighting policy. The stage-17 review supplied no reachability or frequency measurements.

**Cost and decision:** **Not needed now.** A later request would need stable contributed IDs, missing-command diagnostics, permission/context filtering, keyboard/mobile placement, policy/reset semantics and upgrade handling. Reuse the palette rather than copying executable handlers into JSON. Reopen only for a demonstrated command-discovery problem that existing shortcuts/contextual commands do not solve; plan that small extension separately.

## Additional specialized journeys

**Real use case and reusable mechanism:** A is the existing commercial record/stage journey; B reorganizes existing tasks/guidance; I maps a published won event to a native task action through automations. No additional operational journey has been requested or observed. The equine/vendor examples in the original analysis are illustrations, not inspected available products.

**Established limit:** profile format 1 selects frame/detail providers and one exact record surface. It is not a workflow language, universal UI tree or mandatory-journey manifest. Complex interactions must exist in plugin code before a profile can select them. The present B fixture is not a packaged production workflow.

**Cost and decision:** **Not needed now.** A concrete journey that cannot use existing routes, record models, operations and automation would require its own bounded plugin increment. Plan data ownership, authorization, validation, transactional versus deferred work, loading/error/offline behavior and accessibility there. A `bare` route or separate frontend remains an exceptional option when its lifecycle costs are justified, not the default for personal customization.

## Multiple named dashboards

**Real use case and reusable mechanism:** the recorded A/B scenarios do not require separate named dashboards. The existing dashboard module owns widget contributions and the resolved personal/default layout; users retain that layout when changing experience.

**Established limit:** `back/modules/dashboard/src/store.rs` stores per-user layouts and the organization default, not a named reusable dashboard catalog. A serialized `dashboard_id` in a profile would currently invent a resource.

**Cost and decision:** **Requires its own increment** if multiple independently addressable dashboards become an approved requirement. The dashboard owner must define identity, CRUD/permissions, defaults, widget settings, package references, migration and deletion/reset behavior before profiles can reference them. No named-dashboard schema or API is added now, and no current layout is duplicated into profiles.

## Separate personal adjustments per profile

**Real use case and reusable mechanism:** switching A/B already retains one personal record-detail adjustment and ignores incompatible values, with independent selection/composition/navigation resets. There is no recorded demand to maintain multiple separate compositions for the same surface under different profile keys.

**Established limit:** `ExperiencePreferences` stores a profile reference and a single record-detail adjustment, not a map of preference copies by profile. The current contract deliberately preserves ownership by surface; raw incompatible references can remain dormant.

**Cost and decision:** **Not needed now.** A later demand must justify extra state and define whether the key is profile, surface or both, exact precedence, policy revocation, profile deletion/rename, reset scope and conversion of the existing single adjustment. Do not guess a previous profile association or discard dormant choices during migration. Reuse the preferences owner; any expansion needs its own reviewed format and acceptance cases, without team inheritance or copies of the full catalog.

## Coexisting backend providers

**Real use case and reusable mechanism:** I consumes the existing native automation/task contract; the scenarios do not request two simultaneous task stores or selectable interchangeable backend providers. Existing contracts, operations, dependency checks and provider services remain the reuse points.

**Established limit:** WASM `held_modules` participates in replacing native modules when contracts are provided; operation-key conflicts and catalog validation constrain registration. Announcing the same contract does not prove semantic or data compatibility. `uses.optional` does not waive operation availability checks, and receiving an event does not grant permission or cross-plugin transaction authority.

**Cost and decision:** **Requires its own increment** before coexistence or automatic provider switching can be promised. Start from a concrete consumer/use and explicit installation-level selection, with stable data identity, storage ownership, authorization, actor/causation semantics, dependent lifecycle and migration/recovery rules. Demonstrate compatibility of persisted references and failure behavior; do not treat API equivalence as storage equivalence or add a universal weighted resolver. No new registry, bus or provider-selection protocol is created here.

## Autonomous SDK distribution

**Real use case and reusable mechanism:** the stage-17 example compiles against the public `@aphel/web-sdk` surface using the host-checkout mapping documented by the current development workflow. It needs no experience internals. No external maintainer's independently versioned development/release requirement was demonstrated in this session.

**Established limit:** public exports and runtime shared assets/import maps are not an independently released type/dependency package. The successful local authoring compilation still relies on this checkout. External reference repositories were not inspected to claim migration compatibility or independent consumer success.

**Cost and decision:** **Not needed now.** Reopen when a real external author needs supported builds without the host checkout. An independent release would require an export/dependency audit, peer/runtime identity rules, type packaging, API/version policy, consumer compatibility checks, release ownership and coordination with host shared assets. Preserve additive existing exports and avoid another React/router/query instance. A separate release process needs its own increment; this decision does not publish a package or imply stable independence already exists.

## UI that cannot receive application-level trust

**Real use case and reusable mechanism:** the delivered scenarios use trusted integrated code. No real less-trusted supplier was admitted or requested. The hypothetical vendor restricted to one selected record, while its user can access much more, is a distinct trust requirement. Current authorized backend APIs and bounded contribution contexts can inform a future design; handing over the full shared SDK/session cannot satisfy it.

**Established limit:** an imported ES module runs in the host document. WASM capabilities, a separate React root, CSS namespacing, error boundaries and a consent label do not restrict its DOM/storage/session-authorized API access. Current public plugin-app CSP includes `frame-ancestors 'none'`; that app is not an already-approved embeddable sandbox. Admission of a version is administrator intent, not supplier authentication, content-signing or isolation.

**Cost and decision:** **Requires its own increment before admitting code that must be less privileged than the user.** Until that work is explicitly justified and accepted, either trust the frontend as application code or do not load it. No iframe, microfrontend, alternate origin, proxy or bridge is built in this step.

A future isolation proposal must meet these requirements before implementation approval:

- Specify the concrete supplier, records/actions it needs, attacker capabilities and trust boundary. Choose an actually separated document/origin or an effective opaque-origin sandbox; a same-origin frame must not retain access to the privileged document. Review cookies/site boundaries, storage, service-worker scope, navigation and authenticated endpoints; a different hostname alone is not an authorization design.
- Use a small typed protocol with message schemas, size/rate limits and allowed operations. Bind messages to the expected window/frame and a fresh channel/session identity, reject unsolicited/replayed/foreign messages and terminate the channel on lifecycle changes. For an opaque origin, origin equality alone cannot authenticate the sender. Do not offer arbitrary HTTP forwarding, script execution or access to the host query cache.
- Keep authority on the server. Bind the allowed record/action set to the initiating actor and current permissions; distinguish interactive user authority from any explicitly approved asynchronous service authority. Recheck mutations and references, record causation/audit where applicable and fail closed on revocation. No host session tokens, unrestricted SDK, secrets or broad system scope are passed to the isolated UI.
- Review both embedding CSP and the isolated document's CSP, network/resource allowlists, framing/navigation restrictions and sandbox flags. Do not remove `frame-ancestors` restrictions without reviewing the intended deployment. CORS/CSP and the bridge must not leave a more privileged direct URL/API path available outside the host flow; direct access must enforce its own authentication/authorization and not rely on a caller-supplied plugin label.
- Define activation/version admission, revocation, channel shutdown, in-flight work, reload and cleanup semantics. Do not promise undo of committed external effects or a hard CPU quota merely from using a frame. Cover focus/keyboard, dimensions/mobile, dialogs/navigation, translation, permitted theme tokens, errors and operational diagnostics.
- Prove absence of host DOM/storage/session authority, denial of out-of-protocol/direct access, invalid-frame/message rejection, current record permission enforcement and revocation, alongside ordinary usability and failure recovery. Plan validation in an authorized environment under project rules; this checklist does not authorize a new test infrastructure or deployment.

These requirements describe a separate security/product increment, not a universal integration runtime or a reason to replace the current shared host.

## Field conditions and semantic migration

No field-policy migration is required by this documentary decision or by keeping version-1 experience composition: the profile cannot edit field conditions. Existing rules and owners remain authoritative. In particular, `record_layouts` still enforces conditional required rules in the backend; `validation_rules` owns boolean validation/veto rules, and workflow owns its stage/checklist requirements. A presentation-hidden/read-only required field must not waive a business requirement.

If a future request separates stored layout conditions from business validation or permits personal field-policy changes, it **requires its own semantic migration increment**. Before changing the stored contract or removing any existing condition, inventory its owner and current effect on create/update/stage transitions. Distinguish visibility/presentation from obligations deliberately enforced today. Do not infer that an old rule was cosmetic merely because it was stored with a layout, and do not duplicate or silently relax it.

That future increment must preserve stable record/field/source identities, references, permission denials, required-field and stage/checklist semantics, hook ordering and transactional event/audit behavior across standard and alternative presentations. Specify explicit before/after configuration conversion, version compatibility and review/diff; unsupported input must fail without partial application or silent coercion. Validate semantic equivalence for ordinary and inaccessible/hidden required fields, unrelated patches, revocation/reset and interrupted conversion in an authorized environment. Preserve personal/sibling-owner configuration; do not make reads, login or preset upgrade rewrite business rules. Follow the project's fresh-schema policy rather than adding speculative legacy aliases, checksum exceptions or adoption transports.

## Increment closure and next action

The implementation scope is frozen at the delivered host/model/profile/distribution/admin/personal boundaries and the bounded native follow-up adapter. The documentary decision is complete without approving an expansion. It does not certify the staged implementation as fully validated, turn B into a production package, establish measured request savings or close the outstanding database/browser/recovery criteria.

The next concrete action is to resume the stage-17 acceptance gate in an existing prepared and explicitly authorized environment, preserving all earlier pending criteria. If that run exposes a regression, correct the bounded criterion with evidence. A new feature or trust/storage/format boundary requires a separate problem statement and plan; it is not authorized by this closure note.
