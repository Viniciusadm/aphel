use imports::DuplicateKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DateOrder {
    DayFirst,
    MonthFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecimalStyle {
    Comma,
    Dot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Transform {
    Keep,
    Email,
    Phone,
    Money,
    Date,
    DateTime,
    Percent,
    Owner,
    EmployeeCount,
    Stage,
    Pipeline,
    Status,
    Done,
}

impl Transform {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Money => "money",
            Self::Date => "date",
            Self::DateTime => "date_time",
            Self::Percent => "percent",
            Self::Owner => "owner",
            Self::EmployeeCount => "employee_count",
            Self::Stage => "stage",
            Self::Pipeline => "pipeline",
            Self::Status => "status",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnRule {
    pub(crate) headers: &'static [&'static str],
    pub(crate) target: &'static str,
    pub(crate) transform: Transform,
}

const fn rule(
    headers: &'static [&'static str],
    target: &'static str,
    transform: Transform,
) -> ColumnRule {
    ColumnRule {
        headers,
        target,
        transform,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKind {
    Import,
    Activities,
    Notes,
}

impl StepKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Activities => "activities",
            Self::Notes => "notes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceStep {
    pub(crate) key: &'static str,
    pub(crate) kind: StepKind,
    pub(crate) record_type: &'static str,
    pub(crate) file_hint: &'static str,
    pub(crate) duplicate_key: DuplicateKey,
    pub(crate) links: &'static [&'static str],
    pub(crate) columns: &'static [ColumnRule],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceTool {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
    pub(crate) date_order: DateOrder,
    pub(crate) decimal: DecimalStyle,
    pub(crate) steps: &'static [SourceStep],
}

impl SourceTool {
    pub(crate) fn step(&self, key: &str) -> Option<&'static SourceStep> {
        self.steps.iter().find(|step| step.key == key)
    }
}

pub(crate) const COMPANIES_STEP: &str = "organizations";
pub(crate) const CONTACTS_STEP: &str = "people";
pub(crate) const DEALS_STEP: &str = "deals";
pub(crate) const ACTIVITIES_STEP: &str = "activities";
pub(crate) const NOTES_STEP: &str = "notes";

pub(crate) const TITLE_COLUMN: &str = "title";
pub(crate) const ACTIVITY_TYPE_COLUMN: &str = "activity_type";
pub(crate) const DUE_AT_COLUMN: &str = "due_at";
pub(crate) const DONE_COLUMN: &str = "done";
pub(crate) const OWNER_COLUMN: &str = "owner_id";
pub(crate) const DESCRIPTION_COLUMN: &str = "description";
pub(crate) const BODY_COLUMN: &str = "body";
pub(crate) const DEAL_COLUMN: &str = "deal";
pub(crate) const CONTACT_COLUMN: &str = "contact";
pub(crate) const CONTACT_EMAIL_COLUMN: &str = "contact_email";
pub(crate) const COMPANY_COLUMN: &str = "company";

const PIPEDRIVE_COMPANIES: &[ColumnRule] = &[
    rule(
        &["Organization - Name", "Organização - Nome", "Name", "Nome"],
        "name",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Owner",
            "Organização - Proprietário",
            "Owner",
            "Proprietário",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &[
            "Organization - Address",
            "Organização - Endereço",
            "Address",
            "Endereço",
        ],
        "address_street",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Address - City/town/village/locality",
            "Organização - Endereço - Cidade",
            "Organization - City",
            "Organização - Cidade",
        ],
        "address_city",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Address - State/county",
            "Organização - Endereço - Estado",
            "Organization - State",
            "Organização - Estado",
        ],
        "address_state",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Address - ZIP/Postal code",
            "Organização - Endereço - CEP",
            "Organization - Postal code",
            "Organização - CEP",
        ],
        "address_postal_code",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Website",
            "Organização - Site",
            "Website",
            "Site",
        ],
        "website",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Industry",
            "Organização - Setor",
            "Industry",
            "Setor",
        ],
        "industry",
        Transform::Keep,
    ),
    rule(
        &[
            "Organization - Number of employees",
            "Organização - Número de funcionários",
        ],
        "size_range",
        Transform::EmployeeCount,
    ),
];

const PIPEDRIVE_CONTACTS: &[ColumnRule] = &[
    rule(
        &["Person - Name", "Pessoa - Nome", "Name", "Nome"],
        "name",
        Transform::Keep,
    ),
    rule(
        &[
            "Person - First name",
            "Pessoa - Primeiro nome",
            "First name",
            "Primeiro nome",
        ],
        "first_name",
        Transform::Keep,
    ),
    rule(
        &[
            "Person - Last name",
            "Pessoa - Sobrenome",
            "Last name",
            "Sobrenome",
        ],
        "last_name",
        Transform::Keep,
    ),
    rule(
        &[
            "Person - Email",
            "Person - Email - Work",
            "Pessoa - E-mail",
            "Pessoa - E-mail - Trabalho",
            "Email",
            "E-mail",
        ],
        "email",
        Transform::Email,
    ),
    rule(
        &[
            "Person - Phone",
            "Person - Phone - Work",
            "Pessoa - Telefone",
            "Pessoa - Telefone - Trabalho",
            "Phone",
            "Telefone",
        ],
        "phone",
        Transform::Phone,
    ),
    rule(
        &[
            "Person - Phone - Mobile",
            "Pessoa - Telefone - Celular",
            "Mobile",
            "Celular",
        ],
        "mobile",
        Transform::Phone,
    ),
    rule(
        &[
            "Person - Organization",
            "Pessoa - Organização",
            "Organization",
            "Organização",
        ],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &[
            "Person - Owner",
            "Pessoa - Proprietário",
            "Owner",
            "Proprietário",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Person - Job title", "Pessoa - Cargo", "Job title", "Cargo"],
        "job_title",
        Transform::Keep,
    ),
];

const PIPEDRIVE_DEALS: &[ColumnRule] = &[
    rule(
        &["Deal - Title", "Negócio - Título", "Title", "Título"],
        "title",
        Transform::Keep,
    ),
    rule(
        &["Deal - Value", "Negócio - Valor", "Value", "Valor"],
        "amount",
        Transform::Money,
    ),
    rule(
        &["Deal - Currency", "Negócio - Moeda", "Currency", "Moeda"],
        "currency",
        Transform::Keep,
    ),
    rule(
        &["Deal - Pipeline", "Negócio - Funil", "Pipeline", "Funil"],
        "pipeline_id",
        Transform::Pipeline,
    ),
    rule(
        &["Deal - Stage", "Negócio - Etapa", "Stage", "Etapa"],
        "stage_id",
        Transform::Stage,
    ),
    rule(
        &[
            "Deal - Organization",
            "Negócio - Organização",
            "Organization",
            "Organização",
        ],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &[
            "Deal - Contact person",
            "Negócio - Pessoa de contato",
            "Contact person",
            "Pessoa de contato",
        ],
        "contact_id",
        Transform::Keep,
    ),
    rule(
        &[
            "Deal - Owner",
            "Negócio - Proprietário",
            "Owner",
            "Proprietário",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &[
            "Deal - Expected close date",
            "Negócio - Data de fechamento esperada",
            "Expected close date",
            "Data de fechamento esperada",
        ],
        "expected_close_date",
        Transform::Date,
    ),
    rule(
        &[
            "Deal - Probability",
            "Negócio - Probabilidade",
            "Probability",
            "Probabilidade",
        ],
        "probability",
        Transform::Percent,
    ),
    rule(
        &[
            "Deal - Source channel",
            "Negócio - Canal de origem",
            "Source channel",
            "Canal de origem",
        ],
        "source",
        Transform::Keep,
    ),
    rule(
        &["Deal - Status", "Negócio - Status", "Status"],
        "status",
        Transform::Status,
    ),
    rule(
        &[
            "Deal - Lost reason",
            "Negócio - Motivo da perda",
            "Lost reason",
            "Motivo da perda",
        ],
        "lost_reason_id",
        Transform::Keep,
    ),
];

const HUBSPOT_COMPANIES: &[ColumnRule] = &[
    rule(
        &["Company name", "Name", "Nome da empresa", "Nome"],
        "name",
        Transform::Keep,
    ),
    rule(
        &[
            "Company Domain Name",
            "Website URL",
            "Nome do domínio da empresa",
            "URL do site",
        ],
        "domain",
        Transform::Keep,
    ),
    rule(
        &["Phone Number", "Número de telefone"],
        "phone",
        Transform::Phone,
    ),
    rule(
        &["Street Address", "Endereço"],
        "address_street",
        Transform::Keep,
    ),
    rule(&["City", "Cidade"], "address_city", Transform::Keep),
    rule(
        &["State/Region", "Estado/Região"],
        "address_state",
        Transform::Keep,
    ),
    rule(
        &["Postal Code", "CEP", "Código postal"],
        "address_postal_code",
        Transform::Keep,
    ),
    rule(&["Industry", "Setor"], "industry", Transform::Keep),
    rule(
        &["Number of Employees", "Número de funcionários"],
        "size_range",
        Transform::EmployeeCount,
    ),
    rule(
        &[
            "Company owner",
            "HubSpot Owner",
            "Proprietário da empresa",
            "Proprietário do HubSpot",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Description", "Descrição"],
        "description",
        Transform::Keep,
    ),
];

const HUBSPOT_CONTACTS: &[ColumnRule] = &[
    rule(&["First Name", "Nome"], "first_name", Transform::Keep),
    rule(&["Last Name", "Sobrenome"], "last_name", Transform::Keep),
    rule(&["Email", "E-mail"], "email", Transform::Email),
    rule(
        &["Phone Number", "Número de telefone"],
        "phone",
        Transform::Phone,
    ),
    rule(
        &["Mobile Phone Number", "Número de celular"],
        "mobile",
        Transform::Phone,
    ),
    rule(&["Job Title", "Cargo"], "job_title", Transform::Keep),
    rule(
        &[
            "Associated Company",
            "Company Name",
            "Empresa associada",
            "Nome da empresa",
        ],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &[
            "Contact owner",
            "HubSpot Owner",
            "Proprietário do contato",
            "Proprietário do HubSpot",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Street Address", "Endereço"],
        "address_street",
        Transform::Keep,
    ),
    rule(&["City", "Cidade"], "address_city", Transform::Keep),
    rule(
        &["State/Region", "Estado/Região"],
        "address_state",
        Transform::Keep,
    ),
    rule(
        &["Postal Code", "CEP", "Código postal"],
        "address_postal_code",
        Transform::Keep,
    ),
    rule(
        &["Original Source", "Fonte original"],
        "source",
        Transform::Keep,
    ),
];

const HUBSPOT_DEALS: &[ColumnRule] = &[
    rule(&["Deal Name", "Nome do negócio"], "title", Transform::Keep),
    rule(&["Amount", "Valor"], "amount", Transform::Money),
    rule(
        &["Close Date", "Data de fechamento"],
        "expected_close_date",
        Transform::Date,
    ),
    rule(&["Pipeline"], "pipeline_id", Transform::Pipeline),
    rule(
        &["Deal Stage", "Etapa do negócio"],
        "stage_id",
        Transform::Stage,
    ),
    rule(
        &[
            "Deal owner",
            "HubSpot Owner",
            "Proprietário do negócio",
            "Proprietário do HubSpot",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Associated Company", "Empresa associada"],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &["Associated Contact", "Contato associado"],
        "contact_id",
        Transform::Keep,
    ),
    rule(
        &["Deal probability", "Probabilidade do negócio"],
        "probability",
        Transform::Percent,
    ),
    rule(&["Currency", "Moeda"], "currency", Transform::Keep),
    rule(
        &["Closed Lost Reason", "Motivo da perda"],
        "lost_reason_id",
        Transform::Keep,
    ),
];

const RD_COMPANIES: &[ColumnRule] = &[
    rule(
        &["Nome", "Nome da empresa", "Empresa"],
        "name",
        Transform::Keep,
    ),
    rule(&["Site", "URL", "Website"], "website", Transform::Keep),
    rule(&["Telefone"], "phone", Transform::Phone),
    rule(&["E-mail", "Email"], "email", Transform::Email),
    rule(&["Segmento", "Segmentos"], "industry", Transform::Keep),
    rule(&["Endereço"], "address_street", Transform::Keep),
    rule(&["Cidade"], "address_city", Transform::Keep),
    rule(&["Estado", "UF"], "address_state", Transform::Keep),
    rule(&["CEP"], "address_postal_code", Transform::Keep),
    rule(
        &[
            "Responsável",
            "Responsável pela empresa",
            "Usuário responsável",
        ],
        "owner_id",
        Transform::Owner,
    ),
    rule(&["Resumo", "Descrição"], "description", Transform::Keep),
];

const RD_CONTACTS: &[ColumnRule] = &[
    rule(&["Nome", "Nome do contato"], "name", Transform::Keep),
    rule(&["E-mail", "Email"], "email", Transform::Email),
    rule(&["Telefone"], "phone", Transform::Phone),
    rule(&["Celular", "WhatsApp"], "mobile", Transform::Phone),
    rule(&["Cargo"], "job_title", Transform::Keep),
    rule(
        &["Empresa", "Nome da empresa"],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &["Responsável", "Usuário responsável"],
        "owner_id",
        Transform::Owner,
    ),
    rule(&["Fonte", "Origem"], "source", Transform::Keep),
];

const RD_DEALS: &[ColumnRule] = &[
    rule(
        &["Nome", "Nome da negociação", "Negociação"],
        "title",
        Transform::Keep,
    ),
    rule(
        &[
            "Valor Total",
            "Valor total",
            "Valor Único",
            "Valor único",
            "Valor",
        ],
        "amount",
        Transform::Money,
    ),
    rule(
        &["Funil", "Funil de vendas"],
        "pipeline_id",
        Transform::Pipeline,
    ),
    rule(&["Etapa", "Etapa do funil"], "stage_id", Transform::Stage),
    rule(
        &["Empresa", "Nome da empresa"],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &["Contato", "Contatos", "Nome do contato"],
        "contact_id",
        Transform::Keep,
    ),
    rule(
        &["Responsável", "Usuário responsável"],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Previsão de fechamento", "Data de previsão de fechamento"],
        "expected_close_date",
        Transform::Date,
    ),
    rule(&["Fonte", "Origem"], "source", Transform::Keep),
    rule(
        &["Status", "Estado", "Situação"],
        "status",
        Transform::Status,
    ),
    rule(
        &["Motivo de Perda", "Motivo de perda", "Motivo da perda"],
        "lost_reason_id",
        Transform::Keep,
    ),
];

const AGENDOR_COMPANIES: &[ColumnRule] = &[
    rule(
        &["Nome", "Nome fantasia", "Empresa"],
        "name",
        Transform::Keep,
    ),
    rule(&["Website", "Site"], "website", Transform::Keep),
    rule(
        &["Telefone", "Telefone comercial"],
        "phone",
        Transform::Phone,
    ),
    rule(&["E-mail", "Email"], "email", Transform::Email),
    rule(&["Setor", "Segmento"], "industry", Transform::Keep),
    rule(&["Endereço", "Rua"], "address_street", Transform::Keep),
    rule(&["Número"], "address_number", Transform::Keep),
    rule(&["Complemento"], "address_complement", Transform::Keep),
    rule(&["Bairro"], "address_district", Transform::Keep),
    rule(&["Cidade"], "address_city", Transform::Keep),
    rule(&["Estado", "UF"], "address_state", Transform::Keep),
    rule(&["CEP"], "address_postal_code", Transform::Keep),
    rule(
        &["Responsável", "Usuário responsável"],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &["Descrição", "Observações"],
        "description",
        Transform::Keep,
    ),
    rule(&["Origem", "Canal de origem"], "source", Transform::Keep),
];

const AGENDOR_CONTACTS: &[ColumnRule] = &[
    rule(&["Nome", "Nome da pessoa"], "name", Transform::Keep),
    rule(&["Cargo"], "job_title", Transform::Keep),
    rule(
        &["Empresa", "Nome da empresa"],
        "company_id",
        Transform::Keep,
    ),
    rule(&["E-mail", "Email"], "email", Transform::Email),
    rule(
        &["Telefone", "Telefone comercial"],
        "phone",
        Transform::Phone,
    ),
    rule(&["Celular", "WhatsApp"], "mobile", Transform::Phone),
    rule(
        &["Responsável", "Usuário responsável"],
        "owner_id",
        Transform::Owner,
    ),
    rule(&["Cidade"], "address_city", Transform::Keep),
    rule(&["Estado", "UF"], "address_state", Transform::Keep),
    rule(&["Origem", "Canal de origem"], "source", Transform::Keep),
    rule(&["Descrição", "Observações"], "notes", Transform::Keep),
];

const AGENDOR_DEALS: &[ColumnRule] = &[
    rule(
        &["Título", "Nome do negócio", "Negócio"],
        "title",
        Transform::Keep,
    ),
    rule(&["Valor", "Valor total"], "amount", Transform::Money),
    rule(&["Funil"], "pipeline_id", Transform::Pipeline),
    rule(&["Etapa", "Etapa do funil"], "stage_id", Transform::Stage),
    rule(
        &["Empresa", "Nome da empresa"],
        "company_id",
        Transform::Keep,
    ),
    rule(
        &["Pessoa", "Contato", "Nome da pessoa"],
        "contact_id",
        Transform::Keep,
    ),
    rule(
        &["Responsável", "Usuário responsável"],
        "owner_id",
        Transform::Owner,
    ),
    rule(
        &[
            "Data de previsão",
            "Previsão de fechamento",
            "Data prevista",
        ],
        "expected_close_date",
        Transform::Date,
    ),
    rule(&["Origem", "Canal de origem"], "source", Transform::Keep),
    rule(&["Status", "Situação"], "status", Transform::Status),
    rule(
        &["Motivo de perda", "Motivo da perda", "Motivo de Perda"],
        "lost_reason_id",
        Transform::Keep,
    ),
];

const COMPANY_LINKS: &[&str] = &[];
const CONTACT_LINKS: &[&str] = &[COMPANIES_STEP];
const DEAL_LINKS: &[&str] = &[COMPANIES_STEP, CONTACTS_STEP];
const HISTORY_LINKS: &[&str] = &[COMPANIES_STEP, CONTACTS_STEP, DEALS_STEP];

struct StepRules {
    organizations: &'static [ColumnRule],
    people: &'static [ColumnRule],
    deals: &'static [ColumnRule],
    activities: &'static [ColumnRule],
    notes: &'static [ColumnRule],
}

const fn steps(
    rules: StepRules,
    hints: [&'static str; 5],
    company_key: DuplicateKey,
) -> [SourceStep; 5] {
    [
        SourceStep {
            key: COMPANIES_STEP,
            kind: StepKind::Import,
            record_type: "organizations.organization",
            file_hint: hints[0],
            duplicate_key: company_key,
            links: COMPANY_LINKS,
            columns: rules.organizations,
        },
        SourceStep {
            key: CONTACTS_STEP,
            kind: StepKind::Import,
            record_type: "people.person",
            file_hint: hints[1],
            duplicate_key: DuplicateKey::Email,
            links: CONTACT_LINKS,
            columns: rules.people,
        },
        SourceStep {
            key: DEALS_STEP,
            kind: StepKind::Import,
            record_type: "deals.deal",
            file_hint: hints[2],
            duplicate_key: DuplicateKey::None,
            links: DEAL_LINKS,
            columns: rules.deals,
        },
        SourceStep {
            key: ACTIVITIES_STEP,
            kind: StepKind::Activities,
            record_type: "activities.activity",
            file_hint: hints[3],
            duplicate_key: DuplicateKey::None,
            links: HISTORY_LINKS,
            columns: rules.activities,
        },
        SourceStep {
            key: NOTES_STEP,
            kind: StepKind::Notes,
            record_type: "notes.note",
            file_hint: hints[4],
            duplicate_key: DuplicateKey::None,
            links: HISTORY_LINKS,
            columns: rules.notes,
        },
    ]
}

const PIPEDRIVE_ACTIVITIES: &[ColumnRule] = &[
    rule(
        &[
            "Activity - Subject",
            "Atividade - Assunto",
            "Subject",
            "Assunto",
        ],
        TITLE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Activity - Type", "Atividade - Tipo", "Type", "Tipo"],
        ACTIVITY_TYPE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &[
            "Activity - Due date",
            "Atividade - Data de vencimento",
            "Due date",
            "Data de vencimento",
        ],
        DUE_AT_COLUMN,
        Transform::DateTime,
    ),
    rule(
        &[
            "Activity - Done",
            "Atividade - Concluída",
            "Done",
            "Concluída",
        ],
        DONE_COLUMN,
        Transform::Done,
    ),
    rule(
        &[
            "Activity - Assigned to user",
            "Atividade - Atribuído ao usuário",
            "Assigned to user",
            "Atribuído ao usuário",
            "Owner",
            "Proprietário",
        ],
        OWNER_COLUMN,
        Transform::Owner,
    ),
    rule(
        &[
            "Activity - Note",
            "Atividade - Anotação",
            "Note",
            "Anotação",
        ],
        DESCRIPTION_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Activity - Deal", "Atividade - Negócio", "Deal", "Negócio"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &[
            "Activity - Contact person",
            "Atividade - Pessoa de contato",
            "Contact person",
            "Pessoa de contato",
        ],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &[
            "Activity - Organization",
            "Atividade - Organização",
            "Organization",
            "Organização",
        ],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const PIPEDRIVE_NOTES: &[ColumnRule] = &[
    rule(
        &[
            "Note - Content",
            "Anotação - Conteúdo",
            "Content",
            "Conteúdo",
        ],
        BODY_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Note - Deal", "Anotação - Negócio", "Deal", "Negócio"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Note - Person", "Anotação - Pessoa", "Person", "Pessoa"],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &[
            "Note - Organization",
            "Anotação - Organização",
            "Organization",
            "Organização",
        ],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const HUBSPOT_ACTIVITIES: &[ColumnRule] = &[
    rule(
        &["Task Title", "Title", "Título da tarefa", "Título"],
        TITLE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Task Type", "Tipo de tarefa"],
        ACTIVITY_TYPE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Due Date", "Data de vencimento"],
        DUE_AT_COLUMN,
        Transform::DateTime,
    ),
    rule(
        &["Task Status", "Status", "Status da tarefa"],
        DONE_COLUMN,
        Transform::Done,
    ),
    rule(
        &[
            "Assigned to",
            "Task owner",
            "Atribuída a",
            "Proprietário da tarefa",
        ],
        OWNER_COLUMN,
        Transform::Owner,
    ),
    rule(
        &["Notes", "Task notes", "Body", "Notas"],
        DESCRIPTION_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Deal", "Negócio associado"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Contact", "Contato associado"],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Company", "Empresa associada"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const HUBSPOT_NOTES: &[ColumnRule] = &[
    rule(
        &["Note body", "Body", "Corpo da nota", "Nota"],
        BODY_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Deal", "Negócio associado"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Contact", "Contato associado"],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Associated Company", "Empresa associada"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const RD_ACTIVITIES: &[ColumnRule] = &[
    rule(
        &["Assunto", "Tarefa", "Título"],
        TITLE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Tipo", "Tipo da tarefa"],
        ACTIVITY_TYPE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Data", "Data da tarefa", "Data agendada"],
        DUE_AT_COLUMN,
        Transform::DateTime,
    ),
    rule(&["Concluída", "Status"], DONE_COLUMN, Transform::Done),
    rule(
        &["Responsável", "Usuário responsável"],
        OWNER_COLUMN,
        Transform::Owner,
    ),
    rule(
        &["Descrição", "Observações"],
        DESCRIPTION_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Negociação", "Nome da negociação"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Contato", "Nome do contato"],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Empresa", "Nome da empresa"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const RD_NOTES: &[ColumnRule] = &[
    rule(
        &["Anotação", "Texto", "Conteúdo"],
        BODY_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Negociação", "Nome da negociação"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Contato", "Nome do contato"],
        CONTACT_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Empresa", "Nome da empresa"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const AGENDOR_ACTIVITIES: &[ColumnRule] = &[
    rule(
        &["Assunto", "Título", "Tarefa"],
        TITLE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Tipo", "Tipo da tarefa"],
        ACTIVITY_TYPE_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Data", "Data agendada", "Data da tarefa"],
        DUE_AT_COLUMN,
        Transform::DateTime,
    ),
    rule(&["Finalizada", "Status"], DONE_COLUMN, Transform::Done),
    rule(
        &["Responsável", "Usuário responsável"],
        OWNER_COLUMN,
        Transform::Owner,
    ),
    rule(&["Descrição", "Texto"], DESCRIPTION_COLUMN, Transform::Keep),
    rule(
        &["Negócio", "Nome do negócio"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(&["Pessoa", "Contato"], CONTACT_COLUMN, Transform::Keep),
    rule(
        &["Empresa", "Nome da empresa"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const AGENDOR_NOTES: &[ColumnRule] = &[
    rule(
        &["Comentário", "Texto", "Histórico"],
        BODY_COLUMN,
        Transform::Keep,
    ),
    rule(
        &["Negócio", "Nome do negócio"],
        DEAL_COLUMN,
        Transform::Keep,
    ),
    rule(&["Pessoa", "Contato"], CONTACT_COLUMN, Transform::Keep),
    rule(
        &["Empresa", "Nome da empresa"],
        COMPANY_COLUMN,
        Transform::Keep,
    ),
];

const PIPEDRIVE_STEPS: [SourceStep; 5] = steps(
    StepRules {
        organizations: PIPEDRIVE_COMPANIES,
        people: PIPEDRIVE_CONTACTS,
        deals: PIPEDRIVE_DEALS,
        activities: PIPEDRIVE_ACTIVITIES,
        notes: PIPEDRIVE_NOTES,
    },
    [
        "organizations.csv",
        "people.csv",
        "deals.csv",
        "activities.csv",
        "notes.csv",
    ],
    DuplicateKey::Name,
);
const HUBSPOT_STEPS: [SourceStep; 5] = steps(
    StepRules {
        organizations: HUBSPOT_COMPANIES,
        people: HUBSPOT_CONTACTS,
        deals: HUBSPOT_DEALS,
        activities: HUBSPOT_ACTIVITIES,
        notes: HUBSPOT_NOTES,
    },
    [
        "organizations.csv",
        "people.csv",
        "deals.csv",
        "tasks.csv",
        "notes.csv",
    ],
    DuplicateKey::Domain,
);
const RD_STEPS: [SourceStep; 5] = steps(
    StepRules {
        organizations: RD_COMPANIES,
        people: RD_CONTACTS,
        deals: RD_DEALS,
        activities: RD_ACTIVITIES,
        notes: RD_NOTES,
    },
    [
        "empresas.csv",
        "contatos.csv",
        "negociacoes.csv",
        "tarefas.csv",
        "anotacoes.csv",
    ],
    DuplicateKey::Name,
);
const AGENDOR_STEPS: [SourceStep; 5] = steps(
    StepRules {
        organizations: AGENDOR_COMPANIES,
        people: AGENDOR_CONTACTS,
        deals: AGENDOR_DEALS,
        activities: AGENDOR_ACTIVITIES,
        notes: AGENDOR_NOTES,
    },
    [
        "empresas.csv",
        "pessoas.csv",
        "negocios.csv",
        "tarefas.csv",
        "comentarios.csv",
    ],
    DuplicateKey::Name,
);

pub(crate) const SOURCE_TOOLS: &[SourceTool] = &[
    SourceTool {
        key: "pipedrive",
        name: "Pipedrive",
        date_order: DateOrder::MonthFirst,
        decimal: DecimalStyle::Dot,
        steps: &PIPEDRIVE_STEPS,
    },
    SourceTool {
        key: "hubspot",
        name: "HubSpot",
        date_order: DateOrder::MonthFirst,
        decimal: DecimalStyle::Dot,
        steps: &HUBSPOT_STEPS,
    },
    SourceTool {
        key: "rd_station",
        name: "RD Station CRM",
        date_order: DateOrder::DayFirst,
        decimal: DecimalStyle::Comma,
        steps: &RD_STEPS,
    },
    SourceTool {
        key: "agendor",
        name: "Agendor",
        date_order: DateOrder::DayFirst,
        decimal: DecimalStyle::Comma,
        steps: &AGENDOR_STEPS,
    },
];

pub(crate) fn source_tool(key: &str) -> Option<&'static SourceTool> {
    SOURCE_TOOLS.iter().find(|tool| tool.key == key)
}
