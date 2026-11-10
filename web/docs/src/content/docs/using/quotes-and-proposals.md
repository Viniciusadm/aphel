---
title: Quotes and proposals
description: Price deals with products, price lists and currencies, send quotes for approval and acceptance, and generate PDF proposals and e-signature requests.
sidebar:
  order: 12
---

This page covers everything between "the deal needs a price" and "the customer signed": the product catalog, price lists, deal items, quotes with versions and approval, PDF proposals and electronic signature.

:::note
These are optional modules, suggested by blueprints as follows. Quotes: Agency, Consultancy, Freelancer, Other business. Products (catalog): Agency, Consultancy, Freelancer, Other business. Price lists: Agency. Deal items (line items): Agency, Consultancy, SaaS. Proposals: Agency, Consultancy. Electronic signature: Agency, Consultancy, Real Estate. Approvals: Consultancy, B2B Sales. Currencies (money) and the shared documents engine come from the base CRM blueprint. Turn modules on or off in **Settings > Modules**; dependencies are enabled with them. See [Modules](/_aphel/docs/admin/modules/) and [Blueprints](/_aphel/docs/admin/blueprints/).
:::

## Permissions

| Permission | Allows | Default roles |
| --- | --- | --- |
| `catalog_items.read` | See products and price lists | Administrator, Manager, Member (organization-wide) |
| `catalog_items.create`, `.update`, `.delete` | Manage products and price lists | Administrator |
| `quotes.read` | Open **Quotes** and the Quotes tab of a deal | Administrator, Manager, Member |
| `quotes.create` | Create quotes and new versions | same |
| `quotes.update` | Edit quotes, send for approval, record acceptance | same |
| `quotes.send` | Send a quote to the customer | same |
| `quotes.delete` | Delete quotes | same |
| `proposals.manage` | Manage proposal templates | Administrator |
| `proposals.generate` | Generate proposals from a deal | Administrator |
| `esign.view`, `esign.send` | See and send signature requests | Administrator |
| `approvals.manage` | Manage approval policies | Administrator |
| `settings.manage` | Quote defaults, currencies, discount limits, deal item settings | Administrator |

For manager and member roles, record permissions apply to their team and their own records respectively. Assign more permissions in [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).

## Products

Open **Products** (page title "Products and services"). Each product has a name, code, price, currency, unit, recurrence (none, monthly, quarterly or yearly), term in months, a description and an **Active** flag. Click **New product** (needs `catalog_items.create`) and open a product to edit or delete it.

Deleting a product removes it from the catalog, but deal items that already use it keep their name, price and unit.

A product can be turned into a **bundle** from its **Bundle** tab: choose component products and quantities, and price it either as a **Fixed price** (the product's own price, or the price list's) or the **Sum of components** (all components must use the bundle currency). A bundle cannot contain another bundle, and a product that is already a component cannot become a bundle. In a quote, a bundle enters as one line showing its components.

## Price lists

From **Products**, click **Price lists**. A list sets prices in one currency for the whole company, for a company **segment**, or for specific companies, with an optional validity period (dates are inclusive).

1. Click **New list** and enter the name, optional key, currency and what it **Applies to**: All organizations, Company segment or Specific organizations.
2. Set **Valid from** and **Valid until**, or leave them blank for no limit.
3. Tick **Default list for the currency** to use it when no company or segment list applies. Only lists for all organizations can be the default, and the new default replaces the previous one.
4. Open the list and add prices per product. Repeat a product with different **Min. qty** values to create quantity tiers: the tier with the largest minimum not above the quoted quantity applies. Products missing from the list use the catalog price.

A new quote picks the best active list for its currency: the company list first, then the segment list, then the currency default. Deleting a list does not change existing quotes, which keep the prices they used.

### Discount limits

With `settings.manage`, the **Discount limits** tab on the same page sets the **Maximum discount** per role (blank means no limit). A quote whose discount exceeds the limit of its owner's role is flagged **Discount above limit** and needs approval before it can be sent, provided you have an approval policy for it (see [Approvals for quotes](#approvals-for-quotes)).

## Currencies

With `settings.manage`, open **Settings > Sales > Currencies and exchange rates**.

- **Enabled currencies.** The base currency is always enabled. Add other currencies by their three-letter ISO code (for example USD).
- **Exchange rates.** A rate is the value of one unit of the currency in the base currency, valid from a date. The latest effective rate applies. You cannot register two rates for the same currency and date.
- **Automatic update.** Choose a rate source (Manual, or Frankfurter from the European Central Bank), optionally the service address of a self-hosted instance, and use **Update now** to fetch rates. Rates are then fetched daily for the enabled currencies. Rates you entered by hand on the same date are kept.

Currencies are available on records only when the Money module is enabled.

## Deal items

Deals can carry items. Open a deal and use its **Items** tab: **Add from catalog** or **Free item**, with quantity, unit price, discount (none, percent or amount), tax and, where relevant, billing frequency and term. Items you add from another currency than the deal's are flagged.

An administrator decides how the deal amount relates to its items in **Settings > Fields and records > Products and items**:

| Mode | Behavior |
| --- | --- |
| Manual | The amount is typed on the deal; items are informational. |
| Sum of items | A deal with items gets its amount computed from them and the field becomes read-only. A deal without items stays manual. |

Switching the mode does not rewrite current amounts: each deal follows the sum on the next change of its items, or when you click **Use total as amount**.

## Quotes

### Create a quote

Open the deal and go to its **Quotes** tab, then click **New from items** (copies the deal items) or **New blank quote**. You can also browse all quotes under **Quotes** in the main menu, with search by title and a status filter.

A new quote takes its currency, contact and company from the deal, the next number of the organization, version 1 and status **Draft**. The owner defaults to you. Valid-until and terms default to the quote settings, and the price list to the best match.

In the editor, fill:

- **Title**, **Valid until**, **Price list** (or "No price list (product price)"), **Terms**, **Notes**, **Currency**.
- **Items**: add catalog products or free items with description, quantity, unit price, discount, and tax %. **Update prices from the list** reprices items from the chosen price list.
- **Total discount**: none, percent or amount, applied to the sum of the lines after their own discounts.

Totals are computed on the server when you save: line = quantity times unit price minus line discount; the total discount is spread across lines before tax; tax is the line tax % on the discounted line. A quote can have up to 200 items.

### Quote states

| State | Meaning |
| --- | --- |
| Draft | Being edited. |
| Pending approval | Waiting for an approval decision. The quote is locked. |
| Approved | Approved, or no approval was needed. Still editable; editing it sends it back to Draft. |
| Sent | Emailed to the customer with a public link. |
| Accepted | The customer or a user accepted it. |
| Rejected | The customer rejected it, optionally with a reason. |
| Expired | A sent quote passed its valid-until date. |

Only Draft and Approved quotes can be edited. To change a sent or closed quote, click **New version**: the previous version becomes read-only, the sent link shows that it was replaced, and the new version starts as a Draft. Versions are listed in the editor, and old versions show "You are looking at an old version". An accepted quote cannot get a new version or be deleted. A quote must have at least one item to be sent for approval, sent or accepted.

### Approvals for quotes

**Send for approval** (needs `quotes.update`) asks the approvals module whether a policy with the moment **Quote sending** applies. If none applies, the quote becomes Approved immediately ("No approval needed"). Otherwise it goes to Pending approval; when all approvers approve it becomes Approved, and if one rejects (or the request is cancelled) it returns to Draft.

Sending a quote that needs approval without an approved request is refused with "This quote needs approval before sending". If you do not use approvals, you can send a Draft directly.

To require approval for large discounts, an administrator creates a policy in **Settings > Sales > Approvals** with record type Quote and moment **Quote sending**, using a condition on the **Discount above limit** field. Policy options are described in [Automations and webhooks](/_aphel/docs/using/automations/#approvals). No such policy exists by default.

### Send to the customer

1. Click **Send to customer** (needs `quotes.send`).
2. Check the recipients: **To** and **Cc**, comma-separated. Empty sends to the deal contact's email.
3. Edit the subject and message. The text `{{link}}` becomes the quote link; without it the link is appended.
4. Click **Send**. The email goes from your mailbox, is linked to the deal, and contains the public link.

You cannot send a quote whose valid-until date has passed: change the date first. **Send again** resends a Sent quote. Use **Copy public link** or **Open public page** to share it another way.

### The customer's page

The public page shows the items, totals, terms and notes. The customer can **Accept quote** by entering name and email (the date, IP address and browser are recorded) or **Reject** with an optional reason, and can print or save the page as PDF. Pages for expired or replaced versions say so. Answers are rate limited per address and per link.

When the customer opens the link for the first time, the quote records it as viewed.

### Record an acceptance yourself

If the customer agreed outside the link (paper, phone, email), click **Record acceptance**, enter the name (empty uses the quote contact), optional email and a note. The deal is updated exactly as for an acceptance through the link.

### What accepting does

On acceptance, Aphel copies the items and amount of the accepted version to the deal, and, if configured, moves the deal to a stage. The quote editor shows whether the deal was updated ("Items and amount applied to the deal") or why not. A failure happens, for example, when the deal and quote currencies differ or the quote owner is no longer active.

With `settings.manage`, open **Settings > Sales > Quotes** to set:

- **Default validity (days)**: 1 to 3650, or empty for none.
- **Default terms** for new quotes.
- **Deal stage on acceptance**: per pipeline, a stage to move the deal to, or **Do not move**. Deals already closed, or already in that stage, are not moved.

Sent quotes whose valid-until date passes become **Expired** automatically.

### PDFs

**Download PDF** gives a simple PDF of the quote. With Proposals enabled, **Generate PDF** creates a PDF from a proposal template and attaches it to the deal.

## Proposals

Proposals are PDF documents generated from a deal using a template. Open **Proposals** in the main menu (needs `proposals.manage`) to manage templates.

### Templates

Click **New template**. Fields: name, key (fixed after creation, used by configuration packages and automations), description, **Active**, **Default template** (used when no template is chosen, including automations) and the content.

The content is plain text with light formatting: `#` for titles and subtitles, `**bold**`, `-` for list items, `---` for a divider. Use **Insert variable** to add data in double braces, grouped as Deal, Contact, Company, Owner, Generated by, Organization, Proposal, Items and, when generated from a quote, quote fields (number, version, title, valid until, terms, notes, totals, public link). Two block variables, the items table and a page break, must be alone on their line. The editor reports unknown or unclosed variables with line and column, and you cannot save with issues. The content is limited to 100,000 characters. A **Preview** can use a real deal.

Enabling the module creates a starter template. From the template list you can **Make default**, **Activate**, **Deactivate** and **Delete**; proposals already generated stay attached to their deals.

### Generate a proposal

On a deal, open the **Proposals** tab (needs `proposals.generate` and permission to update the deal), click **Generate proposal**, choose a template, optionally preview, and click **Generate PDF**. The PDF uses the current deal data, gets the next proposal number, is attached to the deal and listed with its template, author and date. From a quote, **Generate PDF** does the same with the quote's data. The module also provides an automation action to generate a proposal and a trigger when one is generated.

## Electronic signature

The e-signature module sends PDFs attached to a deal to Clicksign for signature and tracks the status. Aphel has no legal validity of its own for signatures: validity comes from the provider.

### Set up (administrator)

1. In Clicksign, generate an API access token and register a webhook pointing to the address shown in Aphel.
2. In **Settings > Modules**, open **Electronic signature** and click **Configure**.
3. Choose the **Environment** (Sandbox for tests, Production for real signatures), paste the **Clicksign access token** and the **Webhook HMAC secret**. Both secrets are stored encrypted and never shown again.
4. Decide whether to **Attach the signed PDF to the deal** (on by default).

In sandbox, signatures have no real effect.

### Send a document

On a deal, open **Signatures**, click **Send for signature** (needs `esign.send`), choose an attached PDF (up to 20 MB), add signers (up to 20, name and email; deal contacts can be added in one click), add an optional message (up to 1000 characters) and click **Send**. Each signer gets an invitation by email and confirms identity with a code.

From a quote in Approved, Sent or Accepted state, **Send for signature** generates the quote PDF with Proposals and sends it to one signer. It needs both Proposals and e-signature enabled.

| Status | Meaning |
| --- | --- |
| Queued, Sending | Delivery to Clicksign in progress. |
| Waiting | Sent; signers have not all signed. |
| Completed | Everyone signed. The signed PDF is attached to the deal if the option is on. |
| Refused, Cancelled, Expired | Closed without completion. |
| Cancellation queued, Cancelling | A cancel was requested. |
| Failed | Delivery failed. |
| Check with provider | Delivery is unconfirmed. Check Clicksign before creating another request. |

Cancel a request from the deal to cancel it in Clicksign. Signature events (sent, signed, refused, cancelled, completed, expired) appear in the deal timeline and are available as automation triggers.

## Gotchas

- **Prices are snapshots.** Changing a price list or product does not change existing quotes; use **Update prices from the list** on a draft.
- **Editing an approved quote resets it to Draft** and requires approval again.
- **A sent quote is read-only.** Create a new version; the old link then shows it was replaced.
- **Currency mismatch** between quote and deal blocks the deal update on acceptance.
- **Proposals and e-signature permissions default to administrators.** Grant `proposals.generate`, `esign.view` and `esign.send` to the roles that need them.
