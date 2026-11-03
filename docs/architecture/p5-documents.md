# P5.8 Documents

`documents` is an optional rendering capability. `aphel-contract-documents` defines document headers, spans, blocks, arbitrary tables, summary lines, output files and signature envelopes/notices without kernel, storage, CRM or business-plugin dependencies. Native SDK ports expose rendering, owner-authorized document sources and signature provider factories.

## Documents review

- [x] Quotes and proposals use one PDF engine for font metrics, encoding, wrapping, page assembly, headers, footers, tables and page breaks.
- [x] Business snapshots, pricing, field visibility and localized document text remain with their owners. The renderer loads no records or tables.
- [x] Tables accept declared columns/weights, rows and summary labels instead of knowing quote or proposal totals. Row continuation repeats headers when a row spans pages.
- [x] Layout validation bounds blocks, nodes, text, cells, columns and rendered pages; mismatched tables and invalid weights fail instead of silently dropping content.
- [x] Weight normalization avoids intermediate floating-point overflow. Unsupported oversized headers are rejected, and content/page limit failures propagate before file persistence.
- [x] The PDF engine preserves the existing font/encoding and assembly implementation; business adapters translate their existing models to the shared document model.
- [x] Quote document resolution remains behind the quote owner's read/field permission checks, and disabled sources are excluded.

## Quote/proposal owner review

- [x] Quote item, tax and discount summaries are composed by quotes; proposal variables, markup and localized item summaries are composed by proposals.
- [x] Rendering failures propagate through the existing mutation/transaction boundaries, before a proposal file is uploaded.
- [x] Both owners declare their renderer capability dependency without a Rust dependency on the documents implementation.

## Signature provider review

- [x] Signature payloads, lifecycle notices and provider interfaces belong to the shared documents contract/SDK.
- [x] The Clicksign adapter implements that interface and its module contributes the factory. Credential/environment construction remains with the provider owner.
- [x] Existing send, cancel and signed-file retrieval paths resolve the contributed provider factory.

## Manual acceptance pending

No PDF was generated as a probe or test, and no runtime instance, automated test or connection call was executed under D4. The layout and interfaces were reviewed statically.

- [ ] Render quotes with hidden fields, items, discounts, taxes, notes and multiple pages; compare content, totals and attachment names with the business snapshot.
- [ ] Render proposals in both locales, including variable formatting, bold text, lists, rules, explicit page breaks, empty/unavailable item tables and multi-page rows.
- [ ] Inspect font encoding, long words, column alignment, repeated headers, footer numbering and PDF metadata in a viewer.
- [ ] Check invalid column layouts, oversized content/page counts and transaction/file rollback on rendering failure.
- [ ] Verify source disablement and read/field access checks before resolving a document.
- [ ] Send, cancel and retrieve a signed document through the provider factory, including unavailable-provider and invalid-credential behavior.

Proposal template/business state and e-sign's existing record linkage remain with their current owners for the thin-domain and remaining-module cleanup in P5.21/P5.22. Rendering and signature interfaces are shared now; they do not introduce alternate legacy paths.
