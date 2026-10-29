use chrono::{DateTime, Duration, Utc};

pub(crate) const DEMO_SEED: u64 = 0x0b17_2026_5eed_0008;
pub(crate) const COMPANY_COUNT: usize = 20;
pub(crate) const CONTACTS_PER_COMPANY: usize = 3;
pub(crate) const DEAL_COUNT: usize = 40;

const COMPANY_NAMES: [&str; COMPANY_COUNT] = [
    "Aurora Logística",
    "Bússola Engenharia",
    "Cerrado Alimentos",
    "Delta Saúde Integrada",
    "Estrela Educação",
    "Farol Contabilidade",
    "Galáxia Software",
    "Horizonte Imóveis",
    "Ipê Construtora",
    "Jangada Turismo",
    "Lume Energia Solar",
    "Maré Pescados",
    "Nuvem Telecom",
    "Orquídea Cosméticos",
    "Pampa Agronegócio",
    "Quasar Consultoria",
    "Rio Claro Transportes",
    "Serra Verde Móveis",
    "Trilha Esportes",
    "Vértice Marketing",
];

const INDUSTRIES: [&str; 10] = [
    "Logística",
    "Engenharia",
    "Alimentos",
    "Saúde",
    "Educação",
    "Serviços financeiros",
    "Tecnologia",
    "Imobiliário",
    "Construção",
    "Turismo",
];

const CITIES: [(&str, &str); 8] = [
    ("São Paulo", "SP"),
    ("Rio de Janeiro", "RJ"),
    ("Belo Horizonte", "MG"),
    ("Curitiba", "PR"),
    ("Porto Alegre", "RS"),
    ("Recife", "PE"),
    ("Salvador", "BA"),
    ("Goiânia", "GO"),
];

const SIZE_RANGES: [&str; 4] = ["size_1_10", "size_11_50", "size_51_200", "size_201_500"];

const FIRST_NAMES: [&str; 20] = [
    "Ana", "Bruno", "Camila", "Diego", "Elisa", "Felipe", "Gabriela", "Henrique", "Isabela",
    "João", "Larissa", "Marcos", "Natália", "Otávio", "Paula", "Rafael", "Sofia", "Thiago",
    "Vanessa", "Yuri",
];

const LAST_NAMES: [&str; 16] = [
    "Almeida", "Barbosa", "Cardoso", "Duarte", "Esteves", "Ferreira", "Gomes", "Lima", "Martins",
    "Nogueira", "Oliveira", "Pereira", "Ribeiro", "Souza", "Teixeira", "Vieira",
];

const JOB_TITLES: [&str; 8] = [
    "Diretora comercial",
    "Gerente de compras",
    "Coordenador financeiro",
    "Analista de operações",
    "CEO",
    "Gerente de TI",
    "Head de marketing",
    "Supervisor de vendas",
];

const SOURCES: [&str; 5] = [
    "Indicação",
    "Site",
    "Evento",
    "LinkedIn",
    "Prospecção ativa",
];

const PRODUCTS: [&str; 8] = [
    "Plano anual",
    "Implantação",
    "Consultoria",
    "Licenças adicionais",
    "Treinamento da equipe",
    "Renovação de contrato",
    "Projeto piloto",
    "Suporte premium",
];

const TIERS: [&str; 3] = ["gold", "silver", "bronze"];
const CHANNELS: [&str; 3] = ["email", "phone", "whatsapp"];
const CONTRACT_MONTHS: [i64; 4] = [6, 12, 24, 36];

const NOTE_BODIES: [&str; 6] = [
    "Reunião produtiva. O cliente quer **reduzir custos operacionais** e pediu uma proposta até o fim do mês.",
    "Pontos levantados:\n\n- prazo de implantação\n- integração com o ERP atual\n- treinamento da equipe",
    "Contato prefere conversar pela manhã. Decisão final passa pela diretoria financeira.",
    "Concorrente apresentou preço menor, mas sem suporte local. Reforçar nosso diferencial de atendimento.",
    "Cliente elogiou o atendimento e pode indicar outras empresas do grupo.",
    "Enviado material com estudos de caso do setor. Aguardando retorno.",
];

const TASK_TITLES: [&str; 8] = [
    "Ligar para alinhar a proposta",
    "Enviar proposta revisada",
    "Agendar demonstração",
    "Follow-up por e-mail",
    "Visitar o escritório do cliente",
    "Confirmar dados para o contrato",
    "Apresentar estudo de caso",
    "Revisar objeções levantadas",
];

const ACTIVITY_KEYS: [&str; 5] = ["call", "email", "meeting", "task", "visit"];

#[derive(Debug, Clone)]
pub(crate) struct DemoRandom(u64);

impl DemoRandom {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound.max(1) as u64) as usize
    }

    pub(crate) fn between(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next_u64() % (high - low + 1).max(1) as u64) as i64
    }

    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoCompany {
    pub(crate) name: String,
    pub(crate) domain: String,
    pub(crate) industry: String,
    pub(crate) size_range: String,
    pub(crate) phone: String,
    pub(crate) city: String,
    pub(crate) state: String,
    pub(crate) source: String,
    pub(crate) tier: String,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoContact {
    pub(crate) company_index: usize,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) email: String,
    pub(crate) mobile: String,
    pub(crate) job_title: String,
    pub(crate) source: String,
    pub(crate) preferred_channel: String,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoDealOutcome {
    Open {
        stage_index: usize,
    },
    Won {
        reached_stage: usize,
    },
    Lost {
        reached_stage: usize,
        reason_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoDeal {
    pub(crate) title: String,
    pub(crate) company_index: usize,
    pub(crate) contact_index: usize,
    pub(crate) amount: i64,
    pub(crate) contract_months: i64,
    pub(crate) expected_close_in_days: i64,
    pub(crate) outcome: DemoDealOutcome,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) last_change_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoTarget {
    Company(usize),
    Contact(usize),
    Deal(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoActivity {
    pub(crate) title: String,
    pub(crate) type_key: String,
    pub(crate) target: DemoTarget,
    pub(crate) due_at: DateTime<Utc>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) priority: String,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoNote {
    pub(crate) target: DemoTarget,
    pub(crate) body: String,
    pub(crate) pinned: bool,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemoDataset {
    pub(crate) companies: Vec<DemoCompany>,
    pub(crate) contacts: Vec<DemoContact>,
    pub(crate) deals: Vec<DemoDeal>,
    pub(crate) activities: Vec<DemoActivity>,
    pub(crate) notes: Vec<DemoNote>,
}

pub(crate) fn generate(
    now: DateTime<Utc>,
    open_stage_count: usize,
    lost_reason_count: usize,
) -> DemoDataset {
    let mut random = DemoRandom::new(DEMO_SEED);
    let days_ago = |random: &mut DemoRandom, low: i64, high: i64| {
        now - Duration::days(random.between(low, high)) - Duration::minutes(random.between(0, 600))
    };

    let companies: Vec<DemoCompany> = COMPANY_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let (city, state) = CITIES[index % CITIES.len()];
            DemoCompany {
                name: (*name).to_owned(),
                domain: format!("{}.example.com", slugify(name)),
                industry: INDUSTRIES[index % INDUSTRIES.len()].to_owned(),
                size_range: random.pick(&SIZE_RANGES).to_string(),
                phone: format!(
                    "+55 11 3{:03}-{:04}",
                    random.between(100, 999),
                    random.between(1000, 9999)
                ),
                city: city.to_owned(),
                state: state.to_owned(),
                source: random.pick(&SOURCES).to_string(),
                tier: random.pick(&TIERS).to_string(),
                created_at: days_ago(&mut random, 90, 180),
            }
        })
        .collect();

    let mut contacts = Vec::new();
    for company_index in 0..companies.len() {
        for slot in 0..CONTACTS_PER_COMPANY {
            let position = company_index * CONTACTS_PER_COMPANY + slot;
            let first_name = FIRST_NAMES[position % FIRST_NAMES.len()];
            let last_name = LAST_NAMES[(position * 7 + company_index) % LAST_NAMES.len()];
            contacts.push(DemoContact {
                company_index,
                first_name: first_name.to_owned(),
                last_name: last_name.to_owned(),
                email: format!(
                    "{}.{}@{}",
                    slugify(first_name),
                    slugify(last_name),
                    companies[company_index].domain
                ),
                mobile: format!(
                    "+55 11 9{:04}-{:04}",
                    random.between(7000, 9999),
                    random.between(1000, 9999)
                ),
                job_title: JOB_TITLES[position % JOB_TITLES.len()].to_owned(),
                source: random.pick(&SOURCES).to_string(),
                preferred_channel: random.pick(&CHANNELS).to_string(),
                created_at: days_ago(&mut random, 60, 89),
            });
        }
    }

    let open_stage_count = open_stage_count.max(1);
    let lost_reason_count = lost_reason_count.max(1);
    let deals: Vec<DemoDeal> = (0..DEAL_COUNT)
        .map(|index| {
            let company_index = index % companies.len();
            let contact_index = company_index * CONTACTS_PER_COMPANY + index % CONTACTS_PER_COMPANY;
            let outcome = match index % 10 {
                0 | 1 => DemoDealOutcome::Won {
                    reached_stage: open_stage_count - 1,
                },
                2 => DemoDealOutcome::Won {
                    reached_stage: random.below(open_stage_count),
                },
                3 | 4 => DemoDealOutcome::Lost {
                    reached_stage: random.below(open_stage_count),
                    reason_index: random.below(lost_reason_count),
                },
                slot => DemoDealOutcome::Open {
                    stage_index: (slot - 5 + index / 10) % open_stage_count,
                },
            };
            let created_at = days_ago(&mut random, 10, 55);
            let last_change_at = created_at
                + Duration::hours(random.between(24, ((now - created_at).num_hours() - 1).max(25)));
            let expected_close_in_days = match outcome {
                DemoDealOutcome::Open { .. } => random.between(-5, 60),
                _ => -(now - last_change_at).num_days(),
            };
            DemoDeal {
                title: format!(
                    "{} - {}",
                    random.pick(&PRODUCTS),
                    companies[company_index].name
                ),
                company_index,
                contact_index,
                amount: random.between(5, 250) * 1000 + random.between(0, 9) * 100,
                contract_months: *random.pick(&CONTRACT_MONTHS),
                expected_close_in_days,
                outcome,
                created_at,
                last_change_at: last_change_at.min(now),
            }
        })
        .collect();

    let mut activities = Vec::new();
    for (index, deal) in deals.iter().enumerate() {
        let open = matches!(deal.outcome, DemoDealOutcome::Open { .. });
        let completed_due = deal.created_at + Duration::days(random.between(1, 5));
        activities.push(DemoActivity {
            title: random.pick(&TASK_TITLES).to_string(),
            type_key: random.pick(&ACTIVITY_KEYS).to_string(),
            target: DemoTarget::Deal(index),
            due_at: completed_due.min(now),
            completed_at: Some(completed_due.min(now)),
            priority: "normal".to_owned(),
            created_at: deal.created_at,
        });
        if open {
            let due_at = match index % 4 {
                0 => now - Duration::days(random.between(1, 10)),
                1 => now + Duration::hours(random.between(2, 8)),
                _ => now + Duration::days(random.between(1, 14)),
            };
            activities.push(DemoActivity {
                title: random.pick(&TASK_TITLES).to_string(),
                type_key: random.pick(&ACTIVITY_KEYS).to_string(),
                target: DemoTarget::Deal(index),
                due_at,
                completed_at: None,
                priority: if index % 3 == 0 { "high" } else { "normal" }.to_owned(),
                created_at: deal.last_change_at,
            });
        }
    }
    for contact_index in (0..contacts.len()).step_by(6) {
        let overdue = contact_index % 12 == 0;
        activities.push(DemoActivity {
            title: "Retomar contato".to_owned(),
            type_key: "call".to_owned(),
            target: DemoTarget::Contact(contact_index),
            due_at: if overdue {
                now - Duration::days(random.between(2, 7))
            } else {
                now + Duration::days(random.between(3, 20))
            },
            completed_at: None,
            priority: "low".to_owned(),
            created_at: contacts[contact_index].created_at,
        });
    }

    let mut notes = Vec::new();
    for (index, deal) in deals.iter().enumerate().filter(|(index, _)| index % 2 == 0) {
        notes.push(DemoNote {
            target: DemoTarget::Deal(index),
            body: random.pick(&NOTE_BODIES).to_string(),
            pinned: index % 8 == 0,
            created_at: deal.created_at + Duration::hours(random.between(2, 48)),
        });
    }
    for company_index in (0..companies.len()).step_by(4) {
        notes.push(DemoNote {
            target: DemoTarget::Company(company_index),
            body: random.pick(&NOTE_BODIES).to_string(),
            pinned: false,
            created_at: companies[company_index].created_at + Duration::days(1),
        });
    }

    DemoDataset {
        companies,
        contacts,
        deals,
        activities,
        notes,
    }
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        let folded = match character {
            'á' | 'à' | 'â' | 'ã' | 'Á' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ü' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            other => other,
        };
        if folded.is_ascii_alphanumeric() {
            slug.push(folded.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}
