use chrono::{DateTime, Duration, Utc};

pub(crate) const DEMO_SEED: u64 = 0x0b17_2026_5eed_0008;

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
    pub(crate) deal_open: Vec<bool>,
    pub(crate) tasks: Vec<DemoActivity>,
    pub(crate) notes: Vec<DemoNote>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DemoAnchors {
    pub(crate) company_names: Vec<String>,
    pub(crate) company_created: Vec<DateTime<Utc>>,
    pub(crate) contact_created: Vec<DateTime<Utc>>,
    pub(crate) deals: Vec<DemoDealAnchor>,
}

#[derive(Debug, Clone)]
pub(crate) struct DemoDealAnchor {
    pub(crate) open: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) last_change_at: DateTime<Utc>,
}

pub(crate) fn generate(now: DateTime<Utc>, anchors: &DemoAnchors) -> DemoDataset {
    let mut random = DemoRandom::new(DEMO_SEED);

    let company_total = anchors.company_names.len();

    let mut tasks = Vec::new();
    for (index, deal) in anchors.deals.iter().enumerate() {
        let open = deal.open;
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
    for (index, deal) in anchors
        .deals
        .iter()
        .enumerate()
        .filter(|(index, _)| index % 2 == 0)
    {
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
        deal_open: anchors.deals.iter().map(|deal| deal.open).collect(),
        tasks,
        notes,
    }
}
