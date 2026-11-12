---
title: Install and operate the Blog and institutional site packs
description: Import Blog 1.7.0 or Institutional site 1.0.1 on Aphel 0.1.1, finish the site from the overview and configure classification, review and appearance.
---

## Release requirements

Blog `1.7.0` contains Content `0.6.0` and requires Aphel `>=0.1.1`, built with `objects`, `site`, `publication` and `tagging`. The pack checks the host version and compiled modules before installing a plugin or creating resources. Compiled but disabled required modules are shown in consent and enabled by the same import. A pack cannot add Rust modules to a running binary.

`editorial_ui` is not required, and the retired Pages plugin must not be installed: Content owns site pages. Approval is optional and is not enabled by this pack. The host version is coordinated with the shared block editor, appearance renderer and native publication service; Content refuses older hosts even if they have an earlier publication module.

## Import and publish

1. Sign in as an administrator and open the existing plugin import screen. Select `blog-1.7.0.aphelpack`.
2. Review the required modules, Content permissions, integrated frontend code and resources in consent, then import. The API equivalent is `POST /api/v1/wasm-runtime/packs` with multipart field `package`, first with `dry_run=true`, then with `acknowledge_web_version=1.7.0` after consent.
3. Check that the import is complete. The pack creates a Blog collection, an About page in draft, menu items for both (About shows once published), the public Categories and Tags vocabularies and an Editorial collection presentation, and makes the Blog the site home page when the organization had not chosen one. It creates no sample entries or mandatory terms and leaves existing identity, themes, menu items and home page intact.
4. Open Site → *Overview*. It shows the public address and a getting-started list (name and logo, home page, menu, first published content, search engines) with a button to each screen that finishes a step, the published and draft counts and the latest published items.
5. Open Site → *Content*, select Blog, create an entry, save its title/body and choose Publish. Review the About page in Site → *Pages* and publish it. Its list and detail use the site's configured address and mount; with the default mount the list is `/s/blog`. No domain, external credentials or additional plugin installation is needed.

If `/blog` is already claimed, the pack's declared suffix strategy chooses `/blog-2` and then subsequent available suffixes. Open the collection to see its actual address; navigation uses that resolved address. Required provisioners are checked before installation, including providers of modules that consent will enable.

Administrators and managers can publish, schedule and withdraw immediately (`content.publish`). Grant it to other roles through role settings when needed. CRUD permissions remain separate from publication and from `site.appearance.edit` and `site.presentation.edit`.

## Review and scheduling

Enable Approvals to require review, then create a policy for `content.entry` at `publication.activate`. Choose approvers other than the requester: people cannot approve their own requests, and an empty eligible approver set blocks publication explicitly. Use explicit locked fields when drafts should remain editable during review.

Approval identifies the captured revision. Editing the draft while it is under review does not change what approval publishes. Scheduling also stores a captured revision; subsequent edits stay private. The worker must run (`aphel serve --with-worker`, or a separate worker) for scheduled activation and approval events.

A configured active approval policy remains a requirement when Approvals is disabled. Publication stops with reason `publication_approvals_unavailable`; enable the provider and submit again after resolving the blocked request. Disabling a provider does not turn a reviewed entity into direct publication. Without a configured policy, the approval provider is optional.

See [Publication](/_aphel/docs/plugins/publication/) for history, retries, cancellation, retention, authorization rechecks and superseded schedules.

## Classification and appearance

Select categories and tags in the shared entry editor. Their public selection is captured with the revision; draft changes do not change public filters. Categories support descendants. Classification can be disabled: list/detail pages and publication still work, category routes return not found, and classification controls are hidden. The Blog pack includes classification by default and will enable it again when reimported.

Site → Appearance manages global identity and collection variants. Save a draft, preview list/detail or an empty collection at desktop/mobile width, then Apply appearance. Editorial, Magazine and Minimal presets use the same tokens and templates. Appearance publishes independently of entry content and publication dates. Without Publication, Site renders theme defaults and does not offer an editing bypass. See [Site pages](/_aphel/docs/plugins/site-pages/) for theme contracts, inheritance and public projections.

## Institutional site

Institutional site `1.0.1` contains the same Content plugin and needs only `objects`, `site` and `publication`. It creates four pages in draft with example text, Home (`/inicio`), About (`/sobre`), Services (`/servicos`) and Contact (`/contato`), menu items for the last three, and points the home page at Home when none was chosen. Until the pages are published the site keeps its welcome screen and the items stay hidden. Replace the example text in Site → *Pages* (live preview, history) and publish each page; Site → *Overview* tracks what is left. Importing it next to Blog keeps the Blog home page and existing menu items, and occupied addresses take a suffix (`/sobre-2`).

## Reimport and recovery

Reimporting the same pack preserves edited collection data, navigation, vocabularies, entries and appearance, without duplicates. Seed resources are not recreated after a person removes them. A resource failure reports HTTP 207 and `complete: false`; inspect the failed resource, correct its cause and import again. Completed resources are kept and incomplete appearance publication resumes using its stable seed identity. The installer does not claim a global rollback of earlier steps.

The generic provisioner supports other packs. A second pack can use a different stable key, collection defaults and navigation id with the same record, vocabulary, link and appearance contracts; no installer branch identifies Blog by name.

## Build and verification

From the workspace containing `app/` and `plugins/`:

```bash
CARGO_NET_OFFLINE=true cargo run --manifest-path app/sdk/cli/Cargo.toml --offline -- bundle plugins/content/packs/blog -o plugins/packs/blog/releases/blog-1.7.0.aphelpack
CARGO_NET_OFFLINE=true cargo run --manifest-path app/sdk/cli/Cargo.toml --offline -- bundle plugins/content/packs/institutional -o plugins/packs/institutional/releases/institutional-1.0.1.aphelpack
```

The adjacent SHA-256 file verifies the delivered archive. The archive contains `pack.toml` and the Content plugin package, including its manifest, rebuilt WASM, shared-SDK web bundle and site templates. Native modules and shared web exports come from the matching host.

The release validation uses a fresh, disposable MySQL installation and the normal installer/API. It covers a single pack import from disabled optional modules, an empty public blog without Pages, default permissions, immutable immediate and scheduled revisions, protected projections, classification enabled/disabled, review by an independent approver, independent appearance publication, reimport preservation, withdrawal from detail/list/sitemap, incompatible-host and missing-capability rejection, and a second pack resolving an occupied address. These are operational checks, not a new test suite. `make probe` also checks native publishable definitions, saved draft versions, protected-field writers, the technical actor and hook priorities without a database.

The baseline is verified without Pages. Installing Pages `0.10.1` separately then verifies free-page and collection route coexistence, collision rejection and propagation of published global tokens to both consumers without changing editorial dates. Verification uses HTTP/API and rendered HTML; it does not claim browser visual or assistive-technology testing.
