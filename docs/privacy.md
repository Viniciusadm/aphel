# Privacy, LGPD and data retention

Aphel provides tools to help the organization process personal data in accordance with the Brazilian General Data Protection Law (LGPD, Law 13.709/2018). They are in **Settings › Privacy** and require the `privacy.manage` permission (administrators already have it).

> **Important:** software does not guarantee compliance. LGPD depends on the company's processes: defining the legal basis for each processing activity, appointing the data protection officer (DPO), responding to data subjects within the deadline, keeping records of processing activities, assessing risks, training the team and taking care of copies that live outside Aphel (spreadsheets, emails, backups, integrated systems). The features below support that work, but do not replace it.

## What Aphel offers

| Feature | Where | What it does |
| --- | --- | --- |
| Personal and sensitive data tagging | Settings › Fields | Each field (including system fields, such as email and phone) can be marked as **personal data** or **sensitive data** (LGPD, art. 5, II). All sensitive data is also personal. Contacts' name, email, phones and address come already marked |
| Find data subject | Privacy › Find data subject | Searches by email, phone or document and shows where the data appears |
| Data subject requests | Privacy › Requests | Record with deadline, execution (export, anonymize, delete) and report |
| Retention | Privacy › Retention | Policies that archive or delete old data, with preview and history |
| Audit | Settings › Audit log | Every search, request, execution, download and policy is recorded, without the personal data in the diff |

## Finding a data subject's data

The search takes an email, phone or document (CPF, CNPJ, etc.) and looks for:

- **Records** (people, organizations, deals and activities) whose fields marked as personal — system or custom — contain the value. Email is compared in lowercase; phone in the international E.164 format (the same one the people module stores, with Brazil as the default country) and also by digits only; documents by digits only. Any other text is searched as a substring.
- **Notes** that mention the term or belong to the records found.
- **Attachments** of the records found.
- **Activities** linked to the records found or that mention the term.
- **Import rows** whose original content contains the term.

The term goes in the request body (`POST /api/v1/privacy/subjects/search`), never in the URL, so it does not appear in access logs. The search is recorded in the audit log (`privacy.subject_searched`) only with the identifier type and the counts found. Archived records and records in the trash also appear.

Only fields marked as personal are included in the search. If the organization stores CPF in a custom field, mark it as personal data so that it is found, exported and anonymized.

## Data subject requests

From the results, select the items and record the request:

- **Export data** — generates a `.zip` with `dados.json` (system and custom fields of each record, notes, activities, attachment metadata, notes that mention the data subject and import rows) and the `anexos/` folder with the files (up to 256 MB of attachments per export; the rest are listed with `omitted_by_size_limit`). The download is authenticated (`GET /api/v1/privacy/requests/{id}/download`), recorded in the audit log, and the file is deleted automatically 30 days after completion. Whoever executed it receives the notification "Your export is ready". The package includes **all** of the data subject's fields, including those hidden from the requester's role: reading is done with the system scope, because the data subject has a right to all of their data and the `privacy.manage` permission of whoever recorded and executed the request has already been required.
- **Anonymize** — replaces personal fields with neutral values: required text fields receive "Titular anonimizado" (people) or "Anonimizado" (other types); the others are left empty. Personal custom values are deleted. The record's free texts (contact notes, company and activity description, deal loss note), the record's notes and attachments (including the file in storage) and the duplicate suggestions are also deleted. **Deals, amounts, stages, pipelines and dates are kept**, so that reports and the sales history remain correct.
- **Delete permanently** — deletes the record, custom values, notes, attachments and files, tags and links, through the same path as permanent deletion from the trash. Activities linked to a deleted contact lose the link; select them too if they need to go.

Flow: the request starts as **pending**, with a default deadline of 15 days (LGPD, art. 19; it can be adjusted) and shows as **overdue** when it passes the deadline without being completed. When executed, it moves to **processing** and runs in the background (job `privacy.request.run`); it ends as **completed** or **failed** with a report of counts. A pending request can be cancelled. Anonymize and delete require typing a confirmation word.

### What is kept about the request

For data minimization, the request **does not store the searched term**: it stores only the type (email, phone, document, text), a masked label (`m***@e***.com`, `***4321`) and a SHA-256 hash of the normalized term together with the organization, useful for recognizing repeated requests without exposing the data. The selected items are stored by identifier. This way, even after an anonymization, the request list does not reveal again who the data subject was.

### History (audit and events)

When anonymizing or deleting, Aphel **also redacts the history** of the affected records: in those records' `audit_logs` and `domain_events`, the known personal values (the values of the personal fields and free texts before the change) and the keys of those fields are replaced by `"[redacted]"`. The change made by the request itself is recorded with the field names and the value `"[redacted]"`, without the data. Fields hidden from the executor's role are also redacted (the values are read with the system scope).

**Mentions in other records.** Personal texts of other records (personal text fields, free texts, history) are also redacted, but only with the data subject's **strong values**, so as not to affect people with the same name:

- emails;
- phones (the value as is and digits only);
- documents and identifiers (values with at least 6 digits, made only of digits and punctuation, such as CPF, CNPJ and RG);
- values of fields marked as sensitive (`is_sensitive`);
- the **full** name as a phrase (first and last name together, whole word, for example "Maria Silva"). A first name alone ("Maria") is never redacted outside the record itself.

In the record's own history all known personal values are still redacted.

**Retries.** Each processed record is marked on the request (`privacy_request_items`) in the same transaction as the anonymization or deletion. If the job crashes and is resumed, records already processed are skipped, with no new change and without publishing `<type>.anonymized`/`<type>.purged` again.

## Retention

Each policy has an action, a period in days and, when it makes sense, a record type:

| Action | Effect |
| --- | --- |
| `archive_inactive` | Archives people, organizations or deals with no change, activity or note for N days (publishes `<type>.archived`) |
| `purge_archived` | Permanently deletes records archived for N days, with custom values, notes, attachments and files |
| `purge_audit_logs` | Deletes audit entries older than N days |
| `purge_security_events` | Deletes security events older than N days |
| `purge_domain_events` | Deletes already processed internal events older than N days. The records' timeline loses those items |
| `purge_orphan_files` | Deletes unreferenced files (no attachment, export, import, request, logo or avatar) created more than N days ago, including from storage |

The **trash has its own retention** (Settings › Trash, job `trash.purge`), so there is no retention action for it.

Safety rules:

- A policy starts **disabled**. Before enabling or running it manually, a **preview** is required ("this would archive 312 records today") made within the last hour and after the last change; changing the period disables the policy.
- Every day at 04:00 (UTC) the `privacy.retention` job creates a run for each active policy (that has not run in the last 20 hours and has no pending run) and enqueues it in the `privacy.retention.run` job, the same one used for manual runs; it also deletes data subject export files older than 30 days.
- The run records progress and a heartbeat (`heartbeat_at`) on each batch; if the worker crashes, the job resumes from where it stopped (the actions are idempotent: candidates are recalculated). Every 15 minutes the `privacy.retention.sweep` job re-enqueues `queued`/`running` runs with no heartbeat for more than 30 minutes and marks as **failed** those stalled for more than 24 hours, so none gets stuck in `running`.
- Each run produces an entry in the **run history** (`retention_runs`) with the number of affected items, removed files, errors, start and end, and publishes `retention_run.completed` or `retention_run.failed`.

## Technical limits

- **Backups are not changed retroactively.** Anonymized or deleted data remains in backups taken before the operation until those backups expire. Set the backup retention (see [backup.md](backup.md)) according to your policy.
- **Webhooks, integrations and emails already sent** cannot be recalled. If an external system received the data, the request must also be fulfilled there.
- **Files exported and downloaded** earlier (list exports, export requests) are the responsibility of whoever downloaded them.
- The webhook delivery log, application logs and copies outside the database (spreadsheets, email attachments) do not go through history redaction.
- The search only considers fields marked as personal and limits each group to 100 results; refine the term if the data subject has many records.
- Verifying that the requester is in fact the data subject, the legal basis for keeping data (tax obligations, defense in legal proceedings, for example) and the decision to fulfill or deny a request are up to the organization (the controller).

## API

All routes are under `/api/v1/privacy` (tag `privacy` in the documentation at `/api/docs`) and require `privacy.manage`:

| Route | Use |
| --- | --- |
| `POST /privacy/subjects/search` | Find data subject |
| `GET/POST /privacy/requests` | List and record requests |
| `GET /privacy/requests/{id}` | Detail with the items |
| `POST /privacy/requests/{id}/execute` · `/cancel` | Execute or cancel a pending request |
| `GET /privacy/requests/{id}/download` | Download the `.zip` of a completed export |
| `GET/POST /privacy/retention-policies` | List and create policies |
| `PATCH/DELETE /privacy/retention-policies/{id}` | Edit name and period, delete |
| `POST /privacy/retention-policies/{id}/preview` · `/enable` · `/disable` · `/run` | Preview, enable, disable and run now |
| `GET /privacy/retention-runs` | Run history |
