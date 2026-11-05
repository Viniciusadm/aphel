use chrono::{DateTime, Duration, Utc};

pub(crate) const DEMO_SEED: u64 = 0x0b17_2026_5eed_0008;
pub(crate) const DEAL_COUNT: usize = 40;

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
    pub(crate) deals: Vec<DemoDeal>,
    pub(crate) tasks: Vec<DemoActivity>,
    pub(crate) notes: Vec<DemoNote>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DemoAnchors {
    pub(crate) company_names: Vec<String>,
    pub(crate) company_created: Vec<DateTime<Utc>>,
    pub(crate) contact_created: Vec<DateTime<Utc>>,
    pub(crate) contacts_by_company: Vec<Vec<usize>>,
}

pub(crate) fn generate(
    now: DateTime<Utc>,
    open_stage_count: usize,
    lost_reason_count: usize,
    anchors: &DemoAnchors,
) -> DemoDataset {
    let mut random = DemoRandom::new(DEMO_SEED);
    let days_ago = |random: &mut DemoRandom, low: i64, high: i64| {
        now - Duration::days(random.between(low, high)) - Duration::minutes(random.between(0, 600))
    };

    let company_total = anchors.company_names.len();
    let usable = company_total > 0 && anchors.contacts_by_company.iter().all(|c| !c.is_empty());
    let deal_total = if usable { DEAL_COUNT } else { 0 };

    let open_stage_count = open_stage_count.max(1);
    let lost_reason_count = lost_reason_count.max(1);
    let deals: Vec<DemoDeal> = (0..deal_total)
        .map(|index| {
            let company_index = index % company_total;
            let contacts = &anchors.contacts_by_company[company_index];
            let contact_index = contacts[index % contacts.len()];
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
                    anchors.company_names[company_index]
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

    let mut tasks = Vec::new();
    for (index, deal) in deals.iter().enumerate() {
        let open = matches!(deal.outcome, DemoDealOutcome::Open { .. });
        let completed_due = deal.created_at + Duration::days(random.between(1, 5));
        tasks.push(DemoActivity {
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
            tasks.push(DemoActivity {
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
    for contact_index in (0..anchors.contact_created.len()).step_by(6) {
        let overdue = contact_index % 12 == 0;
        tasks.push(DemoActivity {
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
            created_at: anchors.contact_created[contact_index],
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
    for company_index in (0..company_total).step_by(4) {
        notes.push(DemoNote {
            target: DemoTarget::Company(company_index),
            body: random.pick(&NOTE_BODIES).to_string(),
            pinned: false,
            created_at: anchors.company_created[company_index] + Duration::days(1),
        });
    }

    DemoDataset {
        deals,
        tasks,
        notes,
    }
}
