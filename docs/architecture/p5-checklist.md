# P5 execution checklist

Versioned P5 progress; mirrored in the workspace `PLATFORM_ARCHITECTURE_CHECKLIST.md`. Each completed item has its own implementation commit.

## P5 Dissolve the CRM graph

Schema policy: the project has no production deployment or existing data to preserve. Per user instruction, P5 rewrites schemas directly for fresh installations. Legacy migration adoption, identity rewrite jobs, checksum exceptions and SMTP fallback compatibility have been removed before P5.2.


Plan: §7.7. Target of each module: §4.3. Evidence: §1.3, §1.7. Before any new abstraction, check the over-generalization risk in §9.1.

**5.1 Party**

- [x] **P5.1** `party` contract and `party.contact_points` trait (§5.2).
- [x] **P5.2** `people` and `organizations`.

Implementation note: P5.1 connects people and organizations as providers and duplicates, trash and privacy as consumers. The contract is kernel/CRM-free; SDK helpers preserve existing field metadata. Plugin-specific review and pending manual acceptance cases are tracked in [the party migration checklist](p5-party.md). P5.2 introduces the people/organizations plugin, schema, API and web identities for fresh installations. P5.3 introduces the staged contract, provider-owned assignment operations and generic checklist progress. Review and pending manual acceptance: [workflow checklist](p5-workflow.md).

**5.2 Workflow**

- [x] **P5.3** `workflow` extracted from `pipelines`, with no knowledge of `deal`.

**5.3 Tasks and calendar**

- [x] **P5.4** `tasks` and `calendar` extracted from `activities`.

**5.4 Messaging**

- [x] **P5.5** `messaging` contract, with `email` as a channel.

**5.5 Supporting capabilities**

- [x] **P5.6** ⇄ `assignment`
- [x] **P5.7** ⇄ `money`
- [x] **P5.8** ⇄ `documents` (§6)
- [x] **P5.9** ⇄ `line-items` (§6). Review and pending manual acceptance: [line items](p5-line-items.md).

**5.6 Generic modules become source-driven**

- [x] **P5.10** ⇄ `timeline`. Review and pending manual acceptance: [timeline sources](p5-timeline.md).
- [x] **P5.11** ⇄ `privacy`. Review and pending manual acceptance: [privacy sources](p5-privacy.md).
- [x] **P5.12** ⇄ `reports`. Review and pending manual acceptance: [entity reports](p5-reports.md).
- [x] **P5.13** ⇄ `dashboard`. Review and pending manual acceptance: [dashboard sources](p5-dashboard.md).
- [x] **P5.14** ⇄ `favorites`. Review and pending manual acceptance: [favorites sources](p5-favorites.md).
- [x] **P5.15** ⇄ `followers`. Review and pending manual acceptance: [followed-event sources](p5-followers.md).
- [x] **P5.16** ⇄ `imports`. Review and pending manual acceptance: [entity-driven imports](p5-imports.md).
- [x] **P5.17** ⇄ `webhooks`. Review and pending manual acceptance: [entity-driven webhooks](p5-webhooks.md).
- [x] **P5.18** ⇄ `sso`, free of CRM dependencies. Review and pending manual acceptance: [SSO ownership](p5-sso.md).

**5.7 Thin domain plugins**

- [ ] **P5.19** ⇄ `deals`
- [ ] **P5.20** ⇄ `leads`
- [ ] **P5.21** ⇄ `quotes`, `proposals`
- [ ] **P5.22** ⇄ Remaining domain plugins from §4.3
- [ ] **P5.23** Bridge plugins, only where traits and contracts do not cover the need (§5.4).

**5.8 Clean schema**

- [ ] **P5.24** Bring the foreign key and SQL ratchets to zero and rewrite migrations per plugin.

**Gate**

- [ ] No plugin depends on another plugin crate.
- [ ] Cross-plugin SQL and foreign keys are zero.
- [ ] `sso` or its successor has no CRM dependency.
