# Proposing templates and modules

Before writing a template or a module, open an issue with the `proposta` label describing:

1. **Who uses it and what problem it solves.** Type of business, team size, what is done outside the CRM today.
2. **Why it does not fit in the core** (for modules) or **why it is not just a variation of an existing template** (for templates). Some things stay out of the core on purpose: finance, inventory, help desk, mass marketing and native e-signature come in as modules or integrations.
3. **What changes in the configuration** (templates): pipelines and stages, fields with type and options, automations. A draft of the exported package helps.
4. **Extension points used** (modules): routes, widgets, automation actions, package sections, permissions and events. If a hook, service or contribution is missing in the kernel or in the owning module, propose the extension point in the same issue (catalog in [hooks](../modules/hooks.md)).
5. **Personal data and secrets:** which fields are personal or sensitive and where the credentials are kept (always in the secret vault).

## Path of a proposal

| Stage | Who | Result |
| --- | --- | --- |
| Triage | Maintainers | Label `aceita`, `precisa de detalhes` or `fora do escopo` (with the reason) |
| Design | Author | Comment with the final design; contract changes approved before the code |
| Implementation | Author | Pull request following `CONTRIBUTING.md` (template: catalog entry; module: crate + documentation) |
| Publication | Maintainers | Entry in `catalog/index.json`; modules in the main repository also go into the default build when it makes sense |

Large proposals that change the core enter the roadmap through the roadmap governance process.
