---
title: Data tools
description: Import, export, bulk-edit, deduplicate, restore, anonymize and back up the data in your Aphel workspace.
sidebar:
  order: 13
---

This page covers the tools you use to move data in and out of Aphel and to keep it clean: imports, exports, bulk actions, duplicate detection, the trash, privacy requests, the full organization export and the assistant that migrates data from another CRM.

Most of these tools live in two places:

- **On a record list** (Contacts, Companies, Deals, Products): the **Import** and **Export** buttons in the list toolbar, and the bulk action bar that appears when you select records.
- **In the Data area of Settings**: history pages and administrator tools. The area is called **Data** and is described as "Import, export and take care of data quality and privacy". It lists only the items you are allowed to open.

## Which modules are available

All of these are modules. They can be switched on or off by an administrator (see [Modules](/_aphel/docs/admin/modules/) and [Blueprints](/_aphel/docs/admin/blueprints/)).

| Tool | Module | Availability |
| --- | --- | --- |
| Import | `imports` | Recommended in the base CRM blueprint, so it is on in every blueprint unless an administrator disables it |
| Export | `exports` | Required by the base CRM blueprint |
| Full export | `org_export` | Required by the base CRM blueprint |
| Bulk actions | `bulk_actions` | Recommended in the base CRM blueprint |
| Duplicates | `duplicates` | Recommended in the base CRM blueprint |
| Trash | `trash` | Recommended in the base CRM blueprint |
| Privacy | `privacy` | Recommended in the base CRM blueprint |
| Migrate from another CRM | `migrations_assistant` | Recommended only in the SaaS blueprint; an administrator can enable it in any workspace |

If a button or settings item described here is missing, the module is probably disabled, or you lack the permission listed for it.

## Importing records

You can import Contacts, Companies, Deals, Products and custom objects. The record types that accept imports are the ones that appear with an **Import** button on their list. Tasks, for example, cannot be imported.

### Requirements

- You need the **create** permission of the record type (for example `people.create`). If you plan to update existing records, you also need the **update** permission (for example `people.update`).
- The file must be a CSV (UTF-8 or Latin-1) or a spreadsheet (XLSX, XLS or ODS; only the first sheet is read).
- The first line must contain the column names.

### Limits

| Limit | Value |
| --- | --- |
| Rows per file | 50,000 |
| Columns per file | 200 |
| Characters per cell | 10,000 (longer values are cut) |
| File size | The organization upload limit, 25 MB by default (administrators change it in Settings, **General**, **Maximum upload size**) |

CSV separators (comma, semicolon, tab or pipe) and the encoding are detected automatically.

### Step by step

1. Open the record list, for example **Contacts**, and click **Import**.
2. **File.** Choose the file and click **Upload and continue**. Aphel shows how many rows it found, the encoding and the separator.
3. **Mapping.** For each column of your file, choose the Aphel field it belongs to, or **Ignore column**. Aphel suggests a field by comparing the column name with field names, so well-named headers match on their own. Required fields are marked **required**; you cannot start until they are all matched. Custom fields are marked **custom**. A preview shows the first rows.
4. Set the options under **Duplicates and relationships** (see below).
5. Click **Validate** to check every row without saving anything. The result lists rows with problems, by line. Duplicates and module rules are only checked during the real import.
6. Click **Import N rows**. The import runs in the background; you can leave the page and you get a notification when it finishes (see [Search and collaboration](/_aphel/docs/using/search-and-collaboration/) for notification preferences).
7. **Result.** You see the totals: rows, created, updated, skipped and failed.

### Duplicate options

| Option | Choices |
| --- | --- |
| **Find duplicates by** | **Do not check**, **Email**, **Phone**, **Domain** or **Name**. Only the choices the record type supports are enabled. The default is the first supported of Domain and Email, otherwise no check. |
| **When a duplicate is found** | **Skip the row**, **Update the existing record** or **Create anyway** |

Gotchas:

- Duplicates inside the same file are detected too: the first row creates the record and the next ones are treated as duplicates of it.
- **Update the existing record** needs the update permission; without it the row fails.
- **Create anyway** turns the duplicate check off.

### Relationships and owners

- A column mapped to a field that points to another record (for example a Contact's Company) can contain the record id or the record's name. Names are looked up in Aphel.
- **Create missing referenced records when their owner permits it** is on by default. When on, a company that is not found by name is created, as long as you are allowed to create companies.
- Owner fields take the email of a member of the organization. An email that matches nobody makes that row fail.
- When your workspace has lead distribution rules (see below), a **Distribution rule** selector appears. Rows without an owner in the file then receive one from the rule.

### Reading the values

- Dates are read as day/month/year (`31/12/2026`) or year-month-day (`2026-12-31`). Month/day/year is not recognized.
- Date and time without a time zone use the organization time zone.
- Numbers accept both `1.234,56` and `1,234.56`.
- Yes/no fields accept `yes`, `no`, `true`, `false`, `1`, `0`, `x`, and the Portuguese words `sim` and `não`.
- Select fields match an option by label or value, ignoring case and accents. Multi-select fields accept several values separated by `;`, `|` or a comma.

### Fixing failed rows

On the result screen:

- **Download CSV with errors** gives you the failed rows with an extra error column. Fix the file and use **Reimport fixed file**; only those rows are imported, with the same mapping.
- **Retry the failed rows** runs the failed rows again without changing the file. Use it when the cause was outside the file, such as an owner who was not yet a member.

### Import history

Open **Data** and then **Imports** to see past imports with their file, type, status (**Draft**, **Queued**, **Running**, **Completed**, **Failed**), result and error file. You can continue a draft from there. Only the person who created an import can change or run it. You see your own imports; users with `settings.manage` see everyone's.

:::note
If a privacy request removes personal data that came from an import, the import is marked as redacted: its original file is deleted and it can no longer be changed or retried.
:::

## Exporting records

1. Open a record list and apply the filter, search and sort you want.
2. Click **Export**.
3. Choose the format, **CSV** or **Excel (XLSX)**, and the columns. Use **All** or **None** to select quickly, and tick **Record ID** if you need ids.
4. Click **Generate file**. The file is built in the background; you can close the window and you are notified when it is ready. Click **Download file**.

What you should know:

- The file uses the current filter, search and sort of the list and includes only records you can see.
- You need `<type>.export` (for example `people.export`, `organizations.export`, `deals.export`, `catalog_items.export`) and the read permission of the type.
- A single export is limited to 100,000 records and 200 columns. Narrow the filter if you hit the limit.
- Open **Data** and then **Exports** to see your previous exports and download them again. The page shows only your own exports.

## Bulk actions

Select records in a list with the checkboxes. The bulk action bar appears. If the list has more records than fit on one page, you can choose **Select all N matching records** to apply an action to everything matching the current filter.

| Action | What it does | Permission needed |
| --- | --- | --- |
| **Assign owner** | Sets a new owner | Update permission of the type |
| **Tags** | Adds or removes a tag | Update permission of the type |
| **Change field** | Sets one field to a value; leave the value blank to clear it | Update permission of the type |
| **Move stage** | Moves deals to a stage of a pipeline; deals that do not meet the stage rules are skipped | `deals.update` |
| **Enroll in sequence** | Enrolls contacts in an active sequence | `sequences.enroll` |
| **Export** | Exports the selected records | Export permission of the type |
| **Archive** / **Restore archived** | Hides records from lists or brings them back | Archive permission of the type, and only for types that support archiving |
| **Delete** | Deletes records; they go to the trash | Delete permission of the type |

Actions that are not available to you, or not supported by the record type, are simply not shown. **Move stage** appears only on deals, and **Enroll in sequence** only on contacts.

How it behaves:

- A bulk action is limited to 10,000 records.
- Up to 200 records are processed immediately. Larger actions run in the background; you can close the window and you are notified when they finish.
- Each record goes through the same validation and permission checks as a single edit. Records that cannot be changed are skipped and listed in a report with the reason.
- Open **Data** and then **Bulk actions** to see the history of your actions and open each report.

## Duplicates

Open **Data** and then **Duplicates** (you need `people.read`). Aphel looks for records that probably describe the same thing and lists them as pairs.

### Detection rules

| Record type | Rules |
| --- | --- |
| Contacts | **Same email**, **Same phone**, **Similar name and same company** |
| Companies | **Same domain**, **Similar name** |
| Other types | **Same identifier (e.g. code)**, when the type has identifier columns such as a product code, and **Similar name** |

Phones are compared by their digits and need at least 8. All rules are on by default. Users with `settings.manage` can turn rules on or off per record type in the **Detection rules** card at the bottom of the page and click **Save rules**.

The check runs every day and also when a record is created. If a new record looks like an existing one, a **Possible duplicate** message offers **Merge** or **Review duplicates**. Users with `settings.manage` can also click **Check now** to run the check immediately.

### Working the queue

Filter the list by record type and status (**Pending**, **Not duplicates**, **Merged**). For each pair:

- **Not a duplicate** removes it from the pending list. You need update permission on the record type.
- **Compare and merge** opens the merge screen. You need update permission on the record that stays (the primary) and delete permission on the one that is merged away.

You can also start a merge from any record with **Merge with...**, which asks you to pick the other record.

### Merging

On the merge screen choose which record is **Primary (kept)** (use **Swap primary** to switch), then pick the value to keep for each field that differs. Click **Merge** and confirm.

- Tasks, notes, attachments, tags, deals and other links of the merged record move to the primary one, and its history shows in the primary timeline.
- The merged record goes to the trash, and its old address redirects to the primary record.
- A merge cannot be undone from the merge screen.

## Trash

Open **Data** and then **Trash** (you need `settings.manage`). Deleted records (contacts, companies, deals, products, tasks, quotes and custom objects) stay here until they are purged.

- Use the search box and the type filter to find a record. The **Permanent deletion** column shows when each one will be erased.
- **Restore** brings a record back with its links. You also need the delete permission of the record type, within the scope you have for it.
- **Delete permanently** erases the record's values, notes, attachments and tags. This cannot be undone.
- **Time in the trash** sets how many days records are kept: 30 by default, between 1 and 365. A daily routine erases records older than that. Changing the value changes the date shown for records already in the trash.

## Privacy requests

Open **Data** and then **Privacy** (you need `privacy.manage`). The page is called **Privacy and LGPD**. Aphel helps you answer requests from the people whose data you hold; legal compliance still depends on your own processes.

### Finding a person

On **Find data subject**, enter an email, phone or document (3 to 255 characters). Aphel searches fields marked as personal data, notes, attachments, tasks and import rows. The term you search is not stored, and the search is recorded in the audit log. Select the items you want and click **Create request**.

### Requests

Choose what the person asked for and a response deadline (15 days by default, as in LGPD article 19; late open requests are flagged **Overdue**).

| Kind | What happens when you execute it |
| --- | --- |
| **Export data** | Builds a .zip with the records, notes, tasks and attachments. It stays available for download for 30 days. |
| **Anonymize** | Replaces personal data with neutral values and removes notes and attachments. Deals, amounts, stages and dates stay in the sales history. |
| **Delete permanently** | Deletes the records, custom values, notes, attachments and files. |

A request stays **Pending** until you open it in the **Requests** tab and execute it. A request can include up to 500 items. Anonymizing and deleting cannot be undone, so you must type a confirmation word (ANONYMIZE or DELETE). Only pending requests can be executed or cancelled.

### Retention

The **Retention** tab holds policies that archive or delete old data automatically. Available actions:

- **Archive records without activity**
- **Permanently delete archived records**
- **Clean old audit entries**
- **Clean old security events**
- **Clean old internal events**
- **Clean unused files**

Each policy has a name, a threshold in days (1 to 36,500) and, for record actions, a record type. Policies are created disabled. Use **Preview** to see how many items would be affected, then **Enable**. Changing the threshold disables the policy until you preview again. Enabled policies run every day at 04:00 UTC, and **Run now** starts one manually. An organization can have at most 50 policies. The trash has its own retention, set in **Trash**.

## Full export

Open **Data** and then **Full export** (you need `organization.export`). It builds a package with all the data of the organization for backup, auditing or moving to another installation.

1. Optionally untick **Include attached files** for a smaller, data-only package.
2. Click **Build export**. Only one export can run at a time. You are notified when it is ready.
3. Download it from **Previous exports**.

The package contains records of every type as JSON Lines, links between records, notes and history, attached files, a configuration package and a manifest with checksums. It gathers personal data of every record, so store it safely and delete copies you no longer need.

Restoring a package is done by an administrator from the server command line into an empty organization. See [Backup and restore](/_aphel/docs/admin/backup-and-restore/).

## Migrate from another CRM

Open **Data** and then **Migrate from another CRM** (you need `settings.manage`, plus the create permission of the data you bring). This assistant converts exports from other tools and prepares ready-to-run imports.

Supported sources: Pipedrive, HubSpot, RD Station CRM and Agendor. The recognized column names come from each tool's public documentation and can vary with account language and version. Columns the assistant does not recognize stay in the file and can be mapped by hand while reviewing the import.

Because those names were never checked against real exports, every upload shows a **Check against the source tool** panel before anything is imported. It lists what the file holds after conversion: the number of records, the total of each money column, how many deals are won, lost and open, the range of each date column and how many rows repeat another row's identifier, with a count of the values that could not be read. Type the number your source tool shows for the same list next to each item: Aphel answers **Matches** or **Differs by** the difference. After the import finishes, the step also confirms that every row of the file was accounted for (imported, updated, skipped or failed) and warns when some rows were not imported.

Work through the steps in this order, so that links find records already imported:

1. **Companies.** Upload the organizations CSV.
2. **People.** Each person is linked to a company by name; missing companies are created.
3. **Deals.** Each deal is linked to company and person by name. Pick the Aphel pipeline and stage for each pipeline and stage of the source tool, and match owners by name or email. Won and lost status is carried over.
4. **Activities** (tasks) and **Notes.** These are migrated directly (up to 5,000 rows per file) and linked to the deal, person and company already in Aphel. Notes without a matching record are listed in an error report. Attachments are not part of CSV exports and are not migrated.

For the first three steps the assistant creates a draft import: review the mapping, then start it as described in [Importing records](#importing-records). Choose the date order (day/month/year or month/day/year) and decimal style for the source before uploading.
