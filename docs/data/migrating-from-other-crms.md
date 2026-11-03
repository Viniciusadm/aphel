# Migrate from another CRM

The migration assistant (*Settings → Migrate from another CRM*) brings into Aphel organizations, people and deals exported as CSV from **Pipedrive**, **HubSpot**, **RD Station CRM** and **Agendor**. It is not a new importer: it converts the source tool's file and creates a regular, already-mapped import in the CSV importer, which you review and start on the import screen.

> **Warning:** the column names recognized below were gathered from each tool's public documentation and from typical exports. They **may vary** depending on the account language, the product version and each company's custom fields. Unrecognized columns remain in the file and can be mapped manually (including to custom fields) when reviewing the import.

## How it works

1. **Choose the source tool.** Each one defines the date format (day/month or month/day when the date uses slashes) and the decimal separator of amounts.
2. **Import in the suggested order:** organizations → people → deals. Each step creates a draft import; review the mapping, **start** it and wait for it to finish before the next one, so that links find the records already imported.
3. **Upload the step's CSV.** The assistant shows the recognized columns, the unrecognized ones, how many values were converted, conversion problems and the first converted rows — without writing anything.
4. **Adjust the mappings** when the file has pipelines, stages or owners: for each distinct source value, choose the Aphel pipeline, stage or member (or keep the value from the file).
5. **Create the import.** The converted file becomes a draft import with the mapping pre-filled and these options:
   - organizations: duplicates by **name** (HubSpot: by **domain**), skipping existing ones;
   - people: duplicates by **email**;
   - deals: no duplicate check;
   - referenced organizations that do not exist are created by name.
6. **Review and start** on the import screen (`/imports/<id>`), which validates row by row and generates the error CSV to fix and re-import.

### Links between files

- People → company: through the company/organization column, matched against the organizations' **name**.
- Deals → company and person: through the organization/company and person/contact columns, matched against the company name and the contact's name (or email).
- Deals → pipeline and stage: by name. Use the assistant's mappings to translate the source stages (`Qualified`, `Proposta enviada`…) into your pipeline's stages; stages without a mapping must exist in Aphel with the same name. Create the pipelines and stages before importing the deals.
- Owners: by email or by the **display name** of an active member. Without a match, the record is left without an owner (the row does not fail).

### Conversions applied

| Conversion | What it does |
| --- | --- |
| Email | First email in the cell, lowercased |
| Phone | Removes formatting; with `+` keeps the country code; 10 or 11 digits become `+55…`; 12–13 digits starting with `55` get the `+` |
| Amount | Removes the currency symbol and the thousands separator according to the tool's convention (`R$ 1.500,00` → `1500.00`; `1,500.50` → `1500.50`) |
| Date | `YYYY-MM-DD`, or `dd/mm/yyyy`/`mm/dd/yyyy` depending on the tool (if the day is greater than 12 the order is detected); the time is discarded in date-only fields |
| Percentage | `40%` → `40`; fractions (`0.4`, or `0,4` in the tools with a decimal comma) → `40` |
| Owner | Name → email of the member with the same display name |
| No. of employees | Number (or a range such as `11-50`) → Aphel company size range |
| Stage / pipeline | Mapping chosen in the assistant |

### What is not migrated

Activities, tasks, notes, attachments and history are not imported by the assistant (the CSV importer accepts organizations, people, deals and custom objects). Neither is the won/lost **status** of deals: they all come in as open, in the indicated stage; mark them as won or lost afterwards (or use final stages). Tags and custom fields from the source can be mapped manually during review, as long as they exist in Aphel.

## API

| Route | What it does |
| --- | --- |
| `GET /api/v1/migration-assistant/sources` | Tools, step order and recognized columns with their conversion |
| `POST /api/v1/migration-assistant/preview` | Multipart `source`, `step`, `file` and `maps` (optional): analysis without writing |
| `POST /api/v1/migration-assistant/imports` | Same form: creates the mapped draft import (`201`, with the `import` and the analysis) |

`maps` is a JSON object with the mappings per source value: `{ "stage_id": { "Qualified": "Qualificação" }, "pipeline_id": { "Sales": "Funil de vendas" }, "owner_id": { "Ana Souza": "ana@empresa.com" } }`. Requires `<type>.create` for the step's type. Errors: `migration_source_not_found` (404) and the same ones as the importer (`empty_import`, `invalid_csv`, `too_many_rows`, `too_many_columns`, `file_too_large`).

## Recognized columns per tool

Matching ignores case, accents and punctuation (`E-mail` = `email`). When more than one column matches the same field, the first one wins.

### Pipedrive (`pipedrive`)

Dates are month/day/year when they use slashes (ISO `YYYY-MM-DD` dates are always accepted); decimal point (`1,500.00`).

**Companies** (`company`, typical file `organizations.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Organization - Name`, `Organização - Nome`, `Name`, `Nome` | — |
| Owner (`owner_id`) | `Organization - Owner`, `Organização - Proprietário`, `Owner`, `Proprietário` | name → member email |
| Address (`address_street`) | `Organization - Address`, `Organização - Endereço`, `Address`, `Endereço` | — |
| City (`address_city`) | `Organization - Address - City/town/village/locality`, `Organização - Endereço - Cidade`, `Organization - City`, `Organização - Cidade` | — |
| State (`address_state`) | `Organization - Address - State/county`, `Organização - Endereço - Estado`, `Organization - State`, `Organização - Estado` | — |
| Postal code (`address_postal_code`) | `Organization - Address - ZIP/Postal code`, `Organização - Endereço - CEP`, `Organization - Postal code`, `Organização - CEP` | — |
| Website (`website`) | `Organization - Website`, `Organização - Site`, `Website`, `Site` | — |
| Industry (`industry`) | `Organization - Industry`, `Organização - Setor`, `Industry`, `Setor` | — |
| Size (`size_range`) | `Organization - Number of employees`, `Organização - Número de funcionários` | no. of employees → size range |

**People** (`contact`, typical file `people.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Person - Name`, `Pessoa - Nome`, `Name`, `Nome` | — |
| First name (`first_name`) | `Person - First name`, `Pessoa - Primeiro nome`, `First name`, `Primeiro nome` | — |
| Last name (`last_name`) | `Person - Last name`, `Pessoa - Sobrenome`, `Last name`, `Sobrenome` | — |
| Email (`email`) | `Person - Email`, `Person - Email - Work`, `Pessoa - E-mail`, `Pessoa - E-mail - Trabalho`, `Email`, `E-mail` | lowercase, first email |
| Phone (`phone`) | `Person - Phone`, `Person - Phone - Work`, `Pessoa - Telefone`, `Pessoa - Telefone - Trabalho`, `Phone`, `Telefone` | phone in E.164 |
| Mobile (`mobile`) | `Person - Phone - Mobile`, `Pessoa - Telefone - Celular`, `Mobile`, `Celular` | phone in E.164 |
| Company (linked by name) (`company_id`) | `Person - Organization`, `Pessoa - Organização`, `Organization`, `Organização` | — |
| Owner (`owner_id`) | `Person - Owner`, `Pessoa - Proprietário`, `Owner`, `Proprietário` | name → member email |
| Job title (`job_title`) | `Person - Job title`, `Pessoa - Cargo`, `Job title`, `Cargo` | — |

**Deals** (`deal`, typical file `deals.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Title (`title`) | `Deal - Title`, `Negócio - Título`, `Title`, `Título` | — |
| Amount (`amount`) | `Deal - Value`, `Negócio - Valor`, `Value`, `Valor` | decimal amount |
| Currency (`currency`) | `Deal - Currency`, `Negócio - Moeda`, `Currency`, `Moeda` | — |
| Pipeline (`pipeline_id`) | `Deal - Pipeline`, `Negócio - Funil`, `Pipeline`, `Funil` | pipeline mapping |
| Stage (`stage_id`) | `Deal - Stage`, `Negócio - Etapa`, `Stage`, `Etapa` | stage mapping |
| Company (linked by name) (`company_id`) | `Deal - Organization`, `Negócio - Organização`, `Organization`, `Organização` | — |
| Contact (linked by name) (`contact_id`) | `Deal - Contact person`, `Negócio - Pessoa de contato`, `Contact person`, `Pessoa de contato` | — |
| Owner (`owner_id`) | `Deal - Owner`, `Negócio - Proprietário`, `Owner`, `Proprietário` | name → member email |
| Expected close date (`expected_close_date`) | `Deal - Expected close date`, `Negócio - Data de fechamento esperada`, `Expected close date`, `Data de fechamento esperada` | date YYYY-MM-DD |
| Probability (`probability`) | `Deal - Probability`, `Negócio - Probabilidade`, `Probability`, `Probabilidade` | percentage 0–100 |
| Source (`source`) | `Deal - Source channel`, `Negócio - Canal de origem`, `Source channel`, `Canal de origem` | — |

### HubSpot (`hubspot`)

Dates are month/day/year when they use slashes (ISO `YYYY-MM-DD` dates are always accepted); decimal point (`1,500.00`).

**Companies** (`company`, typical file `organizations.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Company name`, `Name`, `Nome da empresa`, `Nome` | — |
| Domain (`domain`) | `Company Domain Name`, `Website URL`, `Nome do domínio da empresa`, `URL do site` | — |
| Phone (`phone`) | `Phone Number`, `Número de telefone` | phone in E.164 |
| Address (`address_street`) | `Street Address`, `Endereço` | — |
| City (`address_city`) | `City`, `Cidade` | — |
| State (`address_state`) | `State/Region`, `Estado/Região` | — |
| Postal code (`address_postal_code`) | `Postal Code`, `CEP`, `Código postal` | — |
| Industry (`industry`) | `Industry`, `Setor` | — |
| Size (`size_range`) | `Number of Employees`, `Número de funcionários` | no. of employees → size range |
| Owner (`owner_id`) | `Company owner`, `HubSpot Owner`, `Proprietário da empresa`, `Proprietário do HubSpot` | name → member email |
| Description (`description`) | `Description`, `Descrição` | — |

**People** (`contact`, typical file `people.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| First name (`first_name`) | `First Name`, `Nome` | — |
| Last name (`last_name`) | `Last Name`, `Sobrenome` | — |
| Email (`email`) | `Email`, `E-mail` | lowercase, first email |
| Phone (`phone`) | `Phone Number`, `Número de telefone` | phone in E.164 |
| Mobile (`mobile`) | `Mobile Phone Number`, `Número de celular` | phone in E.164 |
| Job title (`job_title`) | `Job Title`, `Cargo` | — |
| Company (linked by name) (`company_id`) | `Associated Company`, `Company Name`, `Empresa associada`, `Nome da empresa` | — |
| Owner (`owner_id`) | `Contact owner`, `HubSpot Owner`, `Proprietário do contato`, `Proprietário do HubSpot` | name → member email |
| Address (`address_street`) | `Street Address`, `Endereço` | — |
| City (`address_city`) | `City`, `Cidade` | — |
| State (`address_state`) | `State/Region`, `Estado/Região` | — |
| Postal code (`address_postal_code`) | `Postal Code`, `CEP`, `Código postal` | — |
| Source (`source`) | `Original Source`, `Fonte original` | — |

**Deals** (`deal`, typical file `deals.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Title (`title`) | `Deal Name`, `Nome do negócio` | — |
| Amount (`amount`) | `Amount`, `Valor` | decimal amount |
| Expected close date (`expected_close_date`) | `Close Date`, `Data de fechamento` | date YYYY-MM-DD |
| Pipeline (`pipeline_id`) | `Pipeline` | pipeline mapping |
| Stage (`stage_id`) | `Deal Stage`, `Etapa do negócio` | stage mapping |
| Owner (`owner_id`) | `Deal owner`, `HubSpot Owner`, `Proprietário do negócio`, `Proprietário do HubSpot` | name → member email |
| Company (linked by name) (`company_id`) | `Associated Company`, `Empresa associada` | — |
| Contact (linked by name) (`contact_id`) | `Associated Contact`, `Contato associado` | — |
| Probability (`probability`) | `Deal probability`, `Probabilidade do negócio` | percentage 0–100 |
| Currency (`currency`) | `Currency`, `Moeda` | — |

### RD Station CRM (`rd_station`)

Dates are day/month/year when they use slashes (ISO `YYYY-MM-DD` dates are always accepted); decimal comma (`1.500,00`).

**Companies** (`company`, typical file `empresas.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Nome`, `Nome da empresa`, `Empresa` | — |
| Website (`website`) | `Site`, `URL`, `Website` | — |
| Phone (`phone`) | `Telefone` | phone in E.164 |
| Email (`email`) | `E-mail`, `Email` | lowercase, first email |
| Industry (`industry`) | `Segmento`, `Segmentos` | — |
| Address (`address_street`) | `Endereço` | — |
| City (`address_city`) | `Cidade` | — |
| State (`address_state`) | `Estado`, `UF` | — |
| Postal code (`address_postal_code`) | `CEP` | — |
| Owner (`owner_id`) | `Responsável`, `Responsável pela empresa`, `Usuário responsável` | name → member email |
| Description (`description`) | `Resumo`, `Descrição` | — |

**People** (`contact`, typical file `contatos.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Nome`, `Nome do contato` | — |
| Email (`email`) | `E-mail`, `Email` | lowercase, first email |
| Phone (`phone`) | `Telefone` | phone in E.164 |
| Mobile (`mobile`) | `Celular`, `WhatsApp` | phone in E.164 |
| Job title (`job_title`) | `Cargo` | — |
| Company (linked by name) (`company_id`) | `Empresa`, `Nome da empresa` | — |
| Owner (`owner_id`) | `Responsável`, `Usuário responsável` | name → member email |
| Source (`source`) | `Fonte`, `Origem` | — |

**Deals** (`deal`, typical file `negociacoes.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Title (`title`) | `Nome`, `Nome da negociação`, `Negociação` | — |
| Amount (`amount`) | `Valor Total`, `Valor total`, `Valor Único`, `Valor único`, `Valor` | decimal amount |
| Pipeline (`pipeline_id`) | `Funil`, `Funil de vendas` | pipeline mapping |
| Stage (`stage_id`) | `Etapa`, `Etapa do funil` | stage mapping |
| Company (linked by name) (`company_id`) | `Empresa`, `Nome da empresa` | — |
| Contact (linked by name) (`contact_id`) | `Contato`, `Contatos`, `Nome do contato` | — |
| Owner (`owner_id`) | `Responsável`, `Usuário responsável` | name → member email |
| Expected close date (`expected_close_date`) | `Previsão de fechamento`, `Data de previsão de fechamento` | date YYYY-MM-DD |
| Source (`source`) | `Fonte`, `Origem` | — |

### Agendor (`agendor`)

Dates are day/month/year when they use slashes (ISO `YYYY-MM-DD` dates are always accepted); decimal comma (`1.500,00`).

**Companies** (`company`, typical file `empresas.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Nome`, `Nome fantasia`, `Empresa` | — |
| Website (`website`) | `Website`, `Site` | — |
| Phone (`phone`) | `Telefone`, `Telefone comercial` | phone in E.164 |
| Email (`email`) | `E-mail`, `Email` | lowercase, first email |
| Industry (`industry`) | `Setor`, `Segmento` | — |
| Address (`address_street`) | `Endereço`, `Rua` | — |
| Number (`address_number`) | `Número` | — |
| Address line 2 (`address_complement`) | `Complemento` | — |
| District (`address_district`) | `Bairro` | — |
| City (`address_city`) | `Cidade` | — |
| State (`address_state`) | `Estado`, `UF` | — |
| Postal code (`address_postal_code`) | `CEP` | — |
| Owner (`owner_id`) | `Responsável`, `Usuário responsável` | name → member email |
| Description (`description`) | `Descrição`, `Observações` | — |
| Source (`source`) | `Origem`, `Canal de origem` | — |

**People** (`contact`, typical file `pessoas.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Name (`name`) | `Nome`, `Nome da pessoa` | — |
| Job title (`job_title`) | `Cargo` | — |
| Company (linked by name) (`company_id`) | `Empresa`, `Nome da empresa` | — |
| Email (`email`) | `E-mail`, `Email` | lowercase, first email |
| Phone (`phone`) | `Telefone`, `Telefone comercial` | phone in E.164 |
| Mobile (`mobile`) | `Celular`, `WhatsApp` | phone in E.164 |
| Owner (`owner_id`) | `Responsável`, `Usuário responsável` | name → member email |
| City (`address_city`) | `Cidade` | — |
| State (`address_state`) | `Estado`, `UF` | — |
| Source (`source`) | `Origem`, `Canal de origem` | — |
| Notes (`notes`) | `Descrição`, `Observações` | — |

**Deals** (`deal`, typical file `negocios.csv`)

| Aphel field | Recognized headers | Conversion |
| --- | --- | --- |
| Title (`title`) | `Título`, `Nome do negócio`, `Negócio` | — |
| Amount (`amount`) | `Valor`, `Valor total` | decimal amount |
| Pipeline (`pipeline_id`) | `Funil` | pipeline mapping |
| Stage (`stage_id`) | `Etapa`, `Etapa do funil` | stage mapping |
| Company (linked by name) (`company_id`) | `Empresa`, `Nome da empresa` | — |
| Contact (linked by name) (`contact_id`) | `Pessoa`, `Contato`, `Nome da pessoa` | — |
| Owner (`owner_id`) | `Responsável`, `Usuário responsável` | name → member email |
| Expected close date (`expected_close_date`) | `Data de previsão`, `Previsão de fechamento`, `Data prevista` | date YYYY-MM-DD |
| Source (`source`) | `Origem`, `Canal de origem` | — |
