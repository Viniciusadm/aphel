---
title: Custom fields
description: How administrators add fields, formulas, validation rules, form layouts, field permissions and custom objects to shape Aphel around their business.
sidebar:
  order: 3
---

Aphel adapts to your business without code. Administrators can add fields to existing record types, calculate values with formulas, block inconsistent data with validation rules, rearrange forms, restrict fields per role and create entirely new record types called custom objects.

All screens on this page live in **Settings** under the group **Fields and records**, and need the `settings.manage` permission. Everyone else benefits from the result but cannot change it.

| Screen | Purpose | Module |
| --- | --- | --- |
| Fields | Add and edit fields of a record type, including formula fields. | built in (Formulas module for formula fields) |
| Field permissions | Hide or lock fields per role. | built in |
| Layouts | Arrange forms and set conditional rules. | Record layouts |
| Validation rules | Block saves that break your rules. | Validation rules |
| Custom objects | Create new record types. | Objects |

A screen only shows if its module is enabled under **Integrations > Modules**.

## Fields

Open **Settings > Fields**. Choose a record type from the **Record types** list: Contacts, Companies, Deals, Activities, Products or Quotes. The list shows every field in its display order.

### Kinds of fields

| Badge | Meaning |
| --- | --- |
| System | Part of the product. You can only change its name (and its privacy classification), and you cannot deactivate it. |
| Module *name* | Created and managed by a module. It cannot be edited or deactivated here; change it through the module. |
| (no badge) | A custom field you created. Fully editable. |
| Inactive | Hidden from forms, filters and lists. Values already stored are kept. |

Other badges show a field's rules: **Required**, **Unique**, **Searchable**, **Personal data** and **Sensitive data**.

### Create a field

1. Choose **New field**.
2. Enter a **Name** (1 to 140 characters).
3. Check the **Key**. It identifies the field in filters, imports and the API. Use lowercase letters, digits and single underscores, start with a letter, at most 64 characters, no trailing underscore. The key **cannot be changed later**, and it cannot reuse the key of a system field or of a relationship of the record type.
4. Choose the **Type** and fill the type settings.
5. Set the rules (**Required**, **Unique value**, **Searchable**) and the privacy classification.
6. Select **Save**.

Each record type accepts up to 200 custom fields.

### Field types

| Type | Stores | Type settings |
| --- | --- | --- |
| Text | Short text | Maximum length (1 to 1024), pattern (regular expression, up to 500 characters), default value |
| Long text | Multi-line text | Maximum length (1 to 65535), default value |
| Number | Numbers | Minimum, maximum, decimal places (0 to 6), default value |
| Currency | Money amounts | Minimum, maximum, decimal places (0 to 6, 2 when empty), currency (3-letter code such as BRL or USD, the organization currency when empty), default value |
| Date | A day | Default value |
| Date and time | A moment | Default value |
| Single select | One option | Options, default option |
| Multi select | Several options | Options, default value |
| Checkbox | Yes or no | Default checked or unchecked |
| Link (URL) | A web address | Default value |
| Email | An email address | Default value |
| Phone | A phone number | Default country (2-letter code; BR when empty), default value |
| User | A member of the organization | none |
| Related record | A link to a record of another type | The related record type (required) |
| Formula | A calculated value | Expression, result type; see [Formulas](#formulas) |

Settings that do not apply to a type are refused. A default value must satisfy the field's own rules (range, decimals, length, pattern, existing option).

### Rules

- **Required**: records cannot be saved with the field empty. Formula fields cannot be required.
- **Unique value**: two records cannot share a value. Available for text, number, link, email and phone fields. Turning it on fails when existing records already repeat a value; fix the duplicates first.
- **Searchable**: the value is included in quick record search. Available for text, long text, link, email and phone fields.
- **Personal data** and **Sensitive data**: the privacy classification (LGPD). Personal data identifies a person and is included in data subject search, export and anonymization. Sensitive data (health, religion, origin and similar) is always personal too, so ticking it also ticks personal data.

### Options of select fields

Single and multi select fields manage a list of options in the **Options** section.

- Each option has a **name** (up to 140 characters), a color (hex, such as `#1f6feb`) and an **Active** switch.
- Drag options to set the order in which they appear. A field can have up to 500 options.
- Options get an internal value generated from the name, and two options cannot share one.
- **Inactive options** can no longer be chosen, but records that already use them keep the value.

### Change a type

A field can change type only within a compatible group: Text, Link, Email and Phone among themselves, or Number and Currency among themselves. Long text, selects, checkbox, date, user and related-record fields keep their type. A formula field cannot become another type and no other type can become a formula. System fields cannot change type.

### Deactivate and reorder

Use **Deactivate** in the field's menu to remove a field from forms, filters and lists without losing data. A confirmation explains that stored values are kept and return if you **Activate** the field again. Drag fields (or use **Move up** and **Move down**) to reorder them.

:::note
Fields that are required without a default value cannot be restricted by field permissions, and system fields marked required are always editable.
:::

## Formulas

A formula field is calculated automatically from other fields on the same record. People cannot edit it by hand. It needs the Formulas module.

### Create a formula field

1. In **Fields**, choose **New field** and set the type to **Formula (calculated field)**.
2. Choose a **Result type**: Number, Currency, Date, Text or Yes/No. This decides how the value is stored, shown and filtered. For numbers you can set **Decimal places** (0 to 6) and for currency the currency code.
3. Write the **Expression**. The panel next to the editor lists fields, relations and functions; click an item to insert it. The editor validates as you type and shows the detected result kind and the fields the formula depends on.
4. Optionally pick a **Sample record** in the preview to see the value it would calculate.
5. Save.

### Writing expressions

- Refer to a field by its key, for example `amount`.
- Operators: `+ - * /`, comparisons `= != < <= > >=`, and `and`, `or`, `not`. Text goes in single or double quotes. `true`, `false` and `null` are available.
- An expression can have at most 2000 characters, 400 elements, 32 levels of nesting and 20 arguments per function.
- Multi-select, user and related-record fields cannot be used inside formulas.
- Related records can only be used inside the aggregate functions.

Examples:

```text
days_until(renewal_date)
if(amount > 10000, "Large", "Standard")
```

Functions:

| Group | Function | What it does |
| --- | --- | --- |
| Dates | `today()` | Today's date. |
| Dates | `days_between(start, end)` | Days between two dates (end minus start). |
| Dates | `days_until(date)` | Days left until the date. |
| Dates | `days_since(date)` | Days elapsed since the date. |
| Dates | `add_days(date, days)` | Adds or subtracts days. |
| Logic | `if(condition, then, else)` | Returns the second value when the condition is true, otherwise the third. |
| Logic | `coalesce(value, fallback, ...)` | First value that is not empty. |
| Logic | `is_empty(value)` | True when the value is empty. |
| Text | `concat(value, ...)` | Joins values into a text. |
| Text | `upper(text)`, `lower(text)` | Changes case. |
| Text | `len(text)` | Number of characters. |
| Numbers | `round(number, decimals)` | Rounds to the given decimal places. |
| Numbers | `abs(number)`, `floor(number)`, `ceil(number)` | Absolute value, round down, round up. |
| Numbers | `min(value, value, ...)`, `max(value, value, ...)` | Smallest or largest of the values. Also `min(relation.field)` and `max(relation.field)`. |
| Aggregates | `count(relation)` | Number of related records. |
| Aggregates | `sum(relation.field)`, `avg(relation.field)` | Sum or average of a field of the related records. |

### How and when they update

A formula is recalculated when a field it uses changes, when a field or link it depends on changes, and when the formula is created or edited. A formula that uses `today()` is recalculated once a day by a scheduled job, so a "days left" value stays current without anyone touching the record.

### Gotchas

- A formula cannot depend on itself, directly or through other formulas. Circular dependencies are rejected when you save.
- A formula can only use fields you can access. Using a field hidden from your role fails with "You have no access to this field".
- The same formula language is used by validation rules.
- The errors the editor shows name the position in the expression, for example unknown field, unknown function, wrong number or type of arguments, or "value types do not match".

## Validation rules

A validation rule blocks a record from being saved when it breaks a business rule. Rules apply wherever data is written: forms, inline editing, the API, imports, automations and bulk actions. Open **Settings > Validation rules** (needs `settings.manage`).

Rules can be defined for Contacts, Companies, Deals, Activities, Quotes, Products and custom objects.

### The rule is the error condition

A rule is a yes/no formula that describes **what is wrong**. When it evaluates to true, the save is blocked and the message is shown. For example, this blocks a large deal that has no source:

```text
amount > 10000 and is_empty(source)
```

### Create a rule

1. Choose the **Record type**, then **New rule**.
2. Enter a **Name** (up to 120 characters) and an **Error message** (up to 500 characters) that the user will see.
3. Write the **Error condition** as a boolean formula.
4. Optionally choose a **Highlighted field**, so the error appears next to that field in the form. With **None (general error)** it appears as a general error.
5. Choose **When to apply**: **On create and update**, **On create only** or **On update only**.
6. Leave **Active rule** on, and select **Save**.

The **Key** is a stable identifier used by configuration packages. Leave it blank to generate it from the name.

### Test, order and switch off

- **Test against a record**: pick an existing record to see whether the rule would block it. The test evaluates the record's saved data.
- Drag rules or use **Move up** and **Move down** to reorder them.
- **Deactivate** suspends a rule without deleting it. **Delete** removes it permanently.
- A record type can have at most 100 rules.
- When several rules fail, every failing message is shown.
- If the condition of a stored rule no longer compiles, for example after a field was removed, the rule is skipped instead of blocking all saves.

:::caution
Rules apply to imports, automations and bulk actions, so a rule that is too strict can make a whole import fail. Test a rule against typical records before activating it. Restoring a record from the Trash is also checked against the active rules.
:::

## Record layouts

Layouts control how fields are arranged on a record's forms and detail panel, and when fields show up or become required. Open **Settings > Layouts** and choose a **Record type**: Contacts, Companies, Deals, Tasks, Products or a custom object.

### Three layouts per type

| Form | Used for |
| --- | --- |
| Create | The form for new records. Required fields are always added to it. |
| Edit | The form for existing records. Without an edit layout, the create layout's rules also apply to edits. |
| View | The side panel on the record page. It accepts visibility rules only. |

A status line shows whether you are editing a **Custom layout** or the **Default layout generated from the fields**. **Restore default** removes the custom layout, and the form is generated from the fields again.

### Arrange the form

- **Tabs**: add, rename, remove and move tabs. A layout has 1 to 10 tabs.
- **Sections**: each tab holds 1 to 20 sections with an optional name.
- **Columns**: each section has 1 to 3 columns.
- **Fields**: drag a field from **Available fields** into a column, or use **Add field**. A layout holds at most 300 fields. Names of tabs and sections are up to 80 characters.
- Use **Preview** to try the form and its rules without saving, then **Save layout**.

### Conditional rules: visible_when and required_when

Open **Rules** on a field in the layout to set conditions. Conditions use the same filter builder as lists and look at the values being filled in the form.

| Rule | Meaning |
| --- | --- |
| **Show only when** (`visible_when`) | The field appears only when the condition is true. |
| **Required when** (`required_when`) | The field becomes required when the condition is true. |

Constraints:

- Conditions can only use fields that are placed in the same layout.
- A field that is always **Required** cannot be hidden, and required fields are always visible.
- A hidden field is never required.
- View layouts do not accept required rules.
- Conditional required rules are enforced when saving, not only in the form. Writes through the API, imports and automations are checked too: creates against the create layout, updates against the edit layout, falling back to the create layout.

### Common patterns

- Show a rental price field only when the deal type is "Rental".
- Require a lost reason only when the status is Lost.
- Move rarely used fields to a second tab so the main form stays short.

## Field permissions

Field permissions decide, per role, which fields of a record type are hidden, read-only or editable. Open **Settings > Fields**, choose the record type, then select **Permissions**.

| Access | Effect |
| --- | --- |
| Editable | Default behavior. |
| Read-only | The field is shown but cannot be changed, not even by imports or bulk actions. |
| Hidden | The field disappears from lists, forms, filters, exports and webhooks. Filtering or sorting by it is refused. |

The matrix has one row per field and one column per role. Use **Apply to column** to set the same access for every field of a role. Choose **Save permissions** and confirm. The change is immediate for the affected users and applies across the app, the API, exports and webhooks.

What cannot be restricted:

- The **Administrator** role always edits every field.
- Required and structural fields: name, title, owner, system dates (created and updated), pipeline, stage and status, plus custom fields that are required without a default value. They show as **Always editable**.

If the page says there are no configurable roles, create roles under **Settings > Roles** first. Someone else may change the matrix while you edit; if so you are asked to reload, and your unsaved changes are discarded.

## Custom objects

Custom objects are record types you design: Contracts, Properties, Projects, Equipment. Open **Settings > Custom objects**. Each object gets its own list, form, record page, permissions, layouts, views, reports, search and API, just like built-in records.

### Create an object

1. Choose **New object**.
2. Enter the **Singular name** and **Plural name** (for example Contract and Contracts, up to 80 characters each).
3. Set the **Key**: lowercase letters and underscores only, starting with a letter, up to 40 characters. The key **cannot be changed later** and becomes the record type `custom.<key>`.
4. Optionally add a description (up to 500 characters), an icon and a color.
5. Pick the **Title field**: the text, email, phone or link field used as the record title. A new object starts with a required **Name** field.
6. Set the options below, then select **Create**.

| Option | Effect |
| --- | --- |
| Records have an owner | Enables the **Owner** field and owner and team based permissions. |
| Records move through stages | Adds a **Stage** field and a Stages board that uses the pipelines of the Workflow module. |
| Active | Inactive objects disappear from the menu and the API and keep their data. |

An organization can have up to 50 custom objects you create. Objects created by modules and plugins do not count. An object can have up to 200 fields, managed with the **Manage fields** button of its settings page.

### Where objects appear

Active objects appear in the **CRM** section of the menu, in the order you set. On the **Custom objects** page, drag objects to reorder the navigation. Open the object's settings page to reach:

- **Open records** to see the list.
- **Fields** to create, edit and reorder fields.
- **Edit layouts** to arrange forms and the detail panel.
- **Roles** and **Field permissions** to control who can use it.

### Permissions

Each object has six permissions: `custom.<key>.read`, `.create`, `.update`, `.delete`, `.export` and `.archive`. Assign them in **Settings > Roles**. A user without `custom.<key>.read` does not see the object in the menu at all.

### Relationships

A relationship links records of two types, for example Contracts to Companies. In the **Relationships** section of the Custom objects page choose **New relationship** and set:

| Setting | Meaning |
| --- | --- |
| From, To | The two record types. They can be custom objects, people, organizations, deals or products. |
| Cardinality | **One to one**, **One to many** or **Many to many**. |
| Labels | The label shown on each side. Each side gets a tab with the linked records. |
| Keys | Identify the relation on each side for the API, imports and filters. Lowercase letters, digits and underscores, up to 48 characters. They cannot be changed later. |
| Required link | Every record on the "To" side must be linked when created, and the last link cannot be removed. Only available when the "To" side is a custom object. |
| When deleting | **Block deletion while linked records exist**, or **Delete and just remove the links**. |

An organization can have up to 200 relationships. Keys must not collide with a field or another relation of the chosen types. On a record page, the **Relationships** tab lists the linked records and lets you **Link** and **Unlink** them. You need access to the other record to see it.

### Delete or deactivate

An object with records cannot be deleted. The dialog offers **Deactivate**, which hides it and keeps the data. To delete an object, delete its records first. Deleting an object removes its fields, views, layouts, reports, permissions and relationships, and cannot be undone.

## Tips

- Plan keys before creating fields, because keys are permanent. Names can be changed at any time.
- The Fields screen offers deactivation, not deletion. Nothing is lost, and you can activate a field again.
- Use configuration packages to copy fields, layouts, views and pipelines between environments. See [Configuration packages](/_aphel/docs/admin/config-packages/).
- For the full list of permission keys, see the [permissions catalog](/_aphel/docs/reference/permissions-catalog/). To work with fields over the API, see the [records API](/_aphel/docs/reference/records-api/).
- Related reading: [Records and views](/_aphel/docs/using/records-and-views/) and [Roles and permissions](/_aphel/docs/admin/roles-and-permissions/).
