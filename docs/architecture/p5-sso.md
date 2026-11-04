# P5.18 SSO identity and ownership

The former oidc plugin is now sso. It owns corporate OpenID Connect login, the identity callback/session flow, identity configuration and its own OAuth state schema. It has no CRM contract or plugin implementation dependencies.

## Static review

- [x] Backend package/module selection, blueprint profiles, catalog entries, API routes, web registration/navigation, settings namespaces and documentation use sso.
- [x] The schema creates sso_oauth_states directly for fresh installations. No old route, identity alias, adoption migration or checksum exception was added.
- [x] Task synchronization, task type selection, attendee links, remote-calendar jobs and conflict handling live with tasks in remote_calendar.
- [x] Mailbox OAuth connection, credential refresh and MIME handling live with email in oauth.
- [x] Google and Microsoft call the corresponding task/email owner operations. SSO does not import those plugins or know their business record types.
- [x] Calendar busy intervals are pure calendar contract data; CalendarAvailability is a native SDK port consumed by booking and contributed by calendar connectors.
- [x] Google, Microsoft and task synchronization no longer depend on the booking implementation for availability types.
- [x] Existing PKCE, state consumption, issuer/audience/nonce/time checks, verified-email policy, member lookup, domain restrictions, session creation and second-factor behavior remain in the identity flow.

## Manual acceptance pending

D4 prohibits tests, probes and runtime instances. Verification uses compilation, strict lint, frontend type checks/build and generated API metadata.

- [ ] Configure corporate SSO on a fresh schema; inspect discovery, callback URLs, disabled providers, domain restrictions and required-SSO password policy.
- [ ] Inspect successful login, inactive/unknown users, second factors, invalid/reused/expired state, nonce and token failures, and rate limiting.
- [ ] Connect Google/Microsoft mailboxes and calendars; inspect refresh/revocation, task import/export, local/remote conflicts, attendee matching and busy intervals.
- [ ] Inspect optional combinations without tasks, email, booking or SSO enabled and verify missing capabilities fail clearly.
- [ ] Confirm the new sso API/web identities in both locales and regenerate deployment-specific identity-provider callback registrations.

Google/Microsoft still consume shared identity and owner implementation utilities. Those consumer-side SDK/contract boundaries and provider-owned state/calendar storage helpers remain explicit work for P5.22–P5.24. This item removes CRM dependencies from SSO itself; it does not claim the whole plugin graph is independent or that runtime acceptance has passed.
