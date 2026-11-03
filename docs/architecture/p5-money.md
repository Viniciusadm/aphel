# P5.7 Money

`money` is an optional capability for enabled currencies, dated exchange rates and base-currency conversion. Fresh schemas belong to `money_currencies`, `money_exchange_rates` and `money_settings`. The canonical rate entity is `money.exchange_rate`; native routes use `/money/currencies`.

`aphel-contract-money` owns monetary values, conversion/context data, checked conversion arithmetic and the `money.amount` field mapping. It has no kernel, database, CRM or plugin dependency. Native SDK ports provide conversion, dated rate lookup and owner-provided recalculation.

## Money capability review

- [x] Read access uses `money.read` or settings management rather than a deal permission. Default roles receive the read permission through owner registration.
- [x] Rates have registry metadata with localized field labels and global read scope. Mutations retain the capability's validated native routes.
- [x] The capability no longer extends a deal entity or owns fields on a foreign table.
- [x] Monetary record owners declare their amount/currency/conversion fields and register their own projections.
- [x] The recalculation job belongs to money and dispatches only enabled owner contributions. A capability without monetary record providers does not enqueue an unknown deal-owned job.
- [x] Conversion uses checked decimal multiplication and preserves cent rounding and missing-rate behavior. Dated-rate, manual-rate protection, provider configuration and organization-time-zone semantics remain with money.
- [x] Currency/FX settings and API clients use the new capability identity directly, without old-route aliases or data rewrites.
- [x] Currency settings UI contains no deal-specific controls or explanatory text.

## Consumer review

- [x] Deals owns its conversion fields and declares `money.amount`; its recalculation source reads rates through the SDK port and writes through the deal owner.
- [x] The deal currency control and its business labels belong to the deals web package.
- [x] Reports, dashboard, forecast, goals, pricing and item helpers import shared monetary arithmetic/constants through the SDK.
- [x] Only demo setup still imports the money implementation for administrative mutation commands; that consumer moves to owner operations during final P5 cleanup.

## Manual acceptance pending

No runtime instance, automated test, probe, connection call or smoke run was executed under D4.

- [ ] Install the fresh schema and check standalone capability enablement, default currency and read/manage permission boundaries.
- [ ] Add, edit, remove and synchronize rates, including future dates, protected manual rates, disabled currencies and decimal boundaries.
- [ ] Change the base currency and verify rebase, current-date selection and missing-rate results.
- [ ] Verify monetary field visibility and scoped record access in native and generic entity reads.
- [ ] Recalculate open deal projections and item amounts, retaining the rate on completed records.
- [ ] Run recalculation with no enabled record provider and with a provider disabled, checking that other owners remain isolated.
- [ ] Check both locales, the settings page and the owner-specific currency control.
