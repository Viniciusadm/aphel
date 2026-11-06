---
title: Contacts and companies
description: Create and maintain the people and companies you sell to, link them together, group companies into hierarchies and merge duplicates.
sidebar:
  order: 4
---

Contacts and companies are the address book of your workspace. A **contact** is a person you talk to. A **company** is the organization a contact works for or that you sell to. Deals, tasks, notes, emails and most other records attach to one or both of them.

In the menu they appear as **Contacts** and **Companies**. Behind them are the `people` and `organizations` modules.

## Availability

| Area | Module | Availability |
| --- | --- | --- |
| Contacts | `people` | Required by the base CRM blueprint, so it is on in every blueprint |
| Companies | `organizations` | Required by the base CRM blueprint, so it is on in every blueprint |
| Duplicate detection and merge | `duplicates` | Recommended by the base CRM blueprint; an administrator can switch it off |
| Lifecycle stage and the lead inbox | `leads` | Optional. See [Leads](/docs/using/leads/) |
| Score | `scoring` | Optional. See [Leads](/docs/using/leads/) |

If something described here is missing for you, the module is disabled or you lack the permission listed for it. See [Modules](/docs/admin/modules/) and [Blueprints](/docs/admin/blueprints/).

## Contacts

Open **Contacts** in the menu (or press `g` then `c`). You see the contacts your role allows you to read, with the search box, filters, saved views and the archived filter described in [Records and views](/docs/using/records-and-views/).

### Fields

| Field | Notes |
| --- | --- |
| Name | The first name. Required, up to 140 characters. |
| Last name | Up to 140 characters. |
| Full name | Built from the name and last name. Read only. |
| Email | Saved in lowercase. |
| Phone, Mobile | Phone numbers are normalized to the international format. |
| Job title | Up to 140 characters. |
| Company | The contact's primary company. |
| Owner | The user responsible for the contact. A new contact is owned by whoever creates it. |
| Street, Number, Address line 2, District, City, State, Postal code, Country | The address. |
| Source | Where the contact came from. Up to 140 characters. |
| Notes | Free text. |

Email tracking adds read-only fields such as **Last contact**, **Last email received**, **Last email sent**, **Email interactions** and **Last contacted by**. The [Leads](/docs/using/leads/) module adds **Lifecycle stage** and the scoring module adds **Score**. Administrators can add more fields; see [Custom fields](/docs/using/custom-fields/).

### Create a contact

1. Open **Contacts** and select **New contact**. You can also use the create menu, or press `c` on the page.
2. Fill in the name. The dialog also offers last name, email, phone, mobile, job title, company and owner. Every other field can be edited later on the contact page.
3. Save. Aphel opens the new contact.

The quick-add form (Name, Phone, Email) creates a contact without leaving the page you are on. It needs `people.create`.

If the new contact looks like an existing one, a warning appears with a **Merge** button. See [Duplicates](#duplicates-and-merging).

### Link a contact to companies

A contact has one **primary company** and any number of **additional companies**.

- Set the primary company with the **Company** field on the contact.
- Open the **Companies** tab of the contact to add additional links. Choose the company, optionally a role, and select **Link company**.

The roles are **Decision maker**, **Buyer**, **Influencer**, **User** and **Other**. The role is optional and applies only to additional links.

Things to know:

- The primary company is listed first and carries a **Primary** badge. It cannot be removed from the tab; change it through the **Company** field.
- Linking a company that is already linked, or is the primary company, is refused with "This company is already linked to the contact."
- Removing a link only removes the association. Neither the contact nor the company is deleted.
- Only companies you are allowed to read are listed.
- You need `people.update` on the contact to add or remove links. Without it the tab is read only.

On a company, the **Contacts** tab lists its people: those whose primary company it is first, then the additionally linked ones. The tab is read only; links are always edited from the contact.

## Companies

Open **Companies** in the menu (or press `g` then `e`).

### Fields

| Field | Notes |
| --- | --- |
| Name | Required, up to 255 characters. |
| Domain | For example `acme.com`. When empty, it is taken from the website. |
| Website | Must be a valid address. |
| Segment | Free text, up to 140 characters. |
| Size | One of 1–10, 11–50, 51–200, 201–500, 501–1000 or 1001+. |
| Phone, Email | Normalized the same way as for contacts. |
| Owner | The user responsible for the company. |
| Street, Number, Address line 2, District, City, State, Postal code, Country | The address. |
| Description | Free text. |
| Source | Up to 140 characters. |
| Parent company | Places the company inside a group. See below. |

### Create a company

1. Open **Companies** and select **New company**.
2. Fill in the name. The dialog also offers domain, website, email, phone and owner.
3. Save.

### Company groups

Companies can be organized into a hierarchy, for example a holding with branches. On the company page, open the **Group** tab.

The tab shows the **Group structure** as a tree and the **Group totals**: companies in the group, contacts, open deals and won revenue, added up across the company and all of its subsidiaries.

To change the structure, you need `organizations.update` on the company you change:

- **Set parent company** (or **Change parent company**) on a company makes it a subsidiary of the one you pick.
- **Add subsidiary** picks a company that becomes a subsidiary of this one. If it already had a parent, the parent is replaced.
- **Remove parent company** turns the company and its own subsidiaries into a group of their own.

Rules:

- A group can have at most 10 levels.
- A company cannot be its own parent or the parent of a company above it in the group.
- The tree shows up to 1000 companies; a notice tells you when the group is larger.
- Totals and the tree count only what you can read. A company you cannot open but that sits between two you can open appears as **Company without access**, so the tree keeps its shape. The contact and deal totals are blank if you lack `people.read` or `deals.read`.
- When a company is deleted, its subsidiaries move up to the deleted company's parent (or become top-level groups). If the company is restored from the trash, the subsidiaries that were detached are reattached when the structure still allows it.

## Archive and delete

On the contact or company page you can **Archive** or **Delete** the record.

- **Archive** hides the record from lists. It stays in the workspace and comes back with **Unarchive**. Use the archived filter on the list to see archived records. You need `people.archive` or `organizations.archive`.
- **Delete** removes the record from lists and sends it to the trash, where an administrator can restore it (**Settings > Data > Trash**, which needs `settings.manage`). You need `people.delete` or `organizations.delete`.

## Duplicates and merging

The `duplicates` module finds records that probably describe the same person or company and lets you merge them.

### How duplicates are found

Detection runs when a record is created, and a full check runs every day. Administrators can also run it from the queue with **Check now**.

| Record type | Rule | Matches when |
| --- | --- | --- |
| Contacts | Same email | The emails are equal, ignoring upper and lower case. |
| Contacts | Same phone | A phone or mobile number is the same. Numbers need at least 8 digits. |
| Contacts | Similar name and same company | The names are similar and the primary company is the same. |
| Companies | Same domain | The domains are equal, ignoring a leading `www.`. |
| Companies | Similar name | The company names are similar. |

All rules are on by default. Deals, products and custom objects can have their own rules; the queue lists every record type that supports detection.

When you create a record that matches an existing one, a warning appears right away. It is only a warning: the record is still created, and the pair joins the duplicate queue.

### The duplicate queue

Go to **Settings > Data > Duplicates**. The page needs `people.read` to open.

1. Choose the **Record type** (Contacts, Companies and so on) and the **Status**: **Pending**, **Not duplicates** or **Merged**.
2. For each pair, select **Compare and merge**, or **Not a duplicate** to dismiss it. A dismissed pair stays dismissed in later checks.

Only pairs where you can read both records are listed. Dismissing needs the update permission of the record type.

Administrators with `settings.manage` also see the **Detection rules** card to switch rules on or off, and the **Check now** button.

### Merge two records

You can start a merge from the queue (**Compare and merge**) or from any contact or company page with **Merge with…**, then pick the duplicate.

1. On the merge screen, one record is **Primary (kept)** and the other is **Duplicate (merged)**. Use **Swap primary** to change which is which.
2. For each field, choose to keep the primary value or use the duplicate value. Fields that are the same on both are marked. If you make no choice, the primary value stays unless it is empty, in which case the duplicate's value fills it.
3. Select **Merge** and confirm.

What happens:

- Activities, notes, attachments, tags, deals and links of the duplicate move to the primary record. For contacts, the additional company links move as well; for companies, the contacts and deals move.
- The duplicate goes to the trash, and its old address opens the primary record. Its history shows in the primary record's timeline.
- On the screen, the merge is described as final. Treat it that way.

You need permission to update the primary record and to delete the duplicate (`people.update` and `people.delete` for contacts, `organizations.update` and `organizations.delete` for companies). Without both, the merge button is not available. A merge can also be stopped by a validation rule or a pending approval, and merging companies is refused if moving the subsidiaries would break the hierarchy limits above.

## Permissions

| Permission | What it allows |
| --- | --- |
| `people.read`, `organizations.read` | See contacts or companies in lists and detail pages. Also needed to open the Duplicates page. |
| `people.create`, `organizations.create` | Create records. |
| `people.update`, `organizations.update` | Edit records, link and unlink companies, change the company group. |
| `people.archive`, `organizations.archive` | Archive and unarchive. Granted to Admin, Manager and Member by default. |
| `people.delete`, `organizations.delete` | Delete records (they go to the trash). |
| `people.export`, `organizations.export` | Export records. Not granted to the Member role by default. |
| `settings.manage` | Edit duplicate detection rules, run the check, restore records from the trash. |

Each record permission also has a scope: own records, the records of your teams, or every record. By default Members see their own records, Managers see their teams' records and Admins see everything. See [Roles and permissions](/docs/admin/roles-and-permissions/) to adjust this.

## Gotchas

- The **Company** field and the **Companies** tab are different things: the field holds the primary company, the tab holds the additional ones.
- Search on Contacts matches name, email, phone and mobile; on Companies it matches name, domain, email and phone.
- Setting a company as the parent of another cannot create a loop and cannot make the group deeper than 10 levels; the screen tells you which rule blocked the change.
- If you open the old address of a merged record, you land on the record that absorbed it.
