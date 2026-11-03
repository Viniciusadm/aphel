# P5.5 Messaging contract and email channel

`aphel-contract-messaging` owns channel identifiers, message and polling payloads, credential containers, reply-window policies and declarative channel configuration. It depends on contract primitives and serialization/schema libraries, with no kernel, CRM or transport dependency.

The native SDK supplies `MessagingChannel`. Enabled providers contribute configuration validation, delivery, polling, address classification, reference threading and reply-window policy. The messaging host resolves these contributions instead of constructing an email transport. Email's adapter lives in the email plugin and uses that owner's mailbox providers. The existing WhatsApp adapter registers through the same point.

## Messaging host review

- [x] Channel identities are validated open strings rather than an email/WhatsApp enum.
- [x] Active delivery and connection operations resolve enabled providers. Compiled provider metadata supports existing channel display, encrypted credential cleanup and export without invoking a disabled transport.
- [x] Poll scheduling skips unavailable providers.
- [x] Reply limits, reference threading and reply windows come from provider policy; the composer uses the same projected metadata.
- [x] Party matching chooses contact-point fields by the provider's address classification and scoped field keys.
- [x] The provider catalog exposes localized configuration fields, options, defaults and credential names, never credential values.
- [x] Channel creation offers enabled providers and renders their configuration metadata. Unknown providers do not require new form code; unavailable providers cannot be edited.
- [x] Optional credential clearing is generic. Credential debug output includes names only.
- [x] Item responses follow the shared client's unwrapped data envelope, while paginated channel/conversation responses retain their metadata.
- [x] The messaging crate no longer depends on the email plugin crate.

## Email owner review

- [x] The adapter uses email-owned IMAP cursors and mailbox transports.
- [x] Configuration validation, sender normalization, message references, polling cursor and retry/error classifications preserve the previous behavior.
- [x] Channel labels, field labels and help text belong to the email owner's bilingual metadata.

## WhatsApp provider review

- [x] The existing adapter contributes through the channel point and declares its phone addressing, peer threading and reply-window policy.
- [x] Cloud delivery, webhook verification and encrypted credential handling remain in the existing implementation.

## Manual acceptance pending

No runtime connection, test instance, probe, automated test or smoke run was executed under D4.

- [ ] Create each enabled channel type, update its configuration, retain/clear optional credentials and verify redacted responses.
- [ ] Receive and reply to an email conversation, including message references, IMAP cursor advancement and retries.
- [ ] Verify WhatsApp signature/challenge handling, peer threading and reply-window enforcement on both server and composer.
- [ ] Disable the email provider and confirm its channel cannot send or poll, while the other provider remains usable.
- [ ] Verify hidden party fields, source error messages, stale-version behavior where applicable and conversation visibility.
- [ ] Confirm channel creation/editing, conversation opening, assignment and connection results use the standard API envelope.

The existing person/deal linkage in the inbox and email's other native consumers remain in the later P5 source/contract cleanup. WhatsApp's built-in webhook integration is a candidate for owner separation in P5.22/P5.23; it does not depend on the email plugin.
