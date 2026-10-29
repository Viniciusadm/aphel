use std::collections::BTreeSet;

use crate::ImportTarget;

const SYNONYMS: &[(&str, &[&str])] = &[
    (
        "name",
        &["nome completo", "full name", "contato", "nome do contato"],
    ),
    ("first_name", &["nome", "primeiro nome", "first name"]),
    ("last_name", &["sobrenome", "ultimo nome", "last name"]),
    (
        "email",
        &[
            "e mail",
            "email",
            "mail",
            "endereco de e mail",
            "endereco de email",
            "correio eletronico",
        ],
    ),
    (
        "phone",
        &["telefone", "fone", "tel", "telefone fixo", "phone"],
    ),
    (
        "mobile",
        &["celular", "cel", "whatsapp", "telefone celular", "mobile"],
    ),
    ("job_title", &["cargo", "funcao", "job title"]),
    (
        "company_id",
        &[
            "empresa",
            "companhia",
            "organizacao",
            "company",
            "razao social",
            "nome da empresa",
            "cliente",
        ],
    ),
    (
        "owner_id",
        &[
            "responsavel",
            "dono",
            "proprietario",
            "vendedor",
            "owner",
            "email do responsavel",
            "e mail do responsavel",
        ],
    ),
    ("domain", &["dominio", "domain"]),
    (
        "website",
        &["site", "website", "url", "pagina", "home page"],
    ),
    ("industry", &["segmento", "setor", "ramo", "industria"]),
    ("size_range", &["porte", "tamanho", "funcionarios"]),
    (
        "address_street",
        &["endereco", "logradouro", "rua", "avenida"],
    ),
    ("address_number", &["numero", "n", "no"]),
    ("address_complement", &["complemento"]),
    ("address_district", &["bairro"]),
    ("address_city", &["cidade", "municipio"]),
    ("address_state", &["estado", "uf"]),
    ("address_postal_code", &["cep", "codigo postal"]),
    ("address_country", &["pais"]),
    ("source", &["origem", "fonte", "canal"]),
    (
        "notes",
        &["observacoes", "observacao", "obs", "notas", "comentarios"],
    ),
    (
        "title",
        &[
            "titulo",
            "oportunidade",
            "negocio",
            "nome da oportunidade",
            "deal",
        ],
    ),
    ("amount", &["valor", "valor total", "preco", "montante"]),
    ("stage_id", &["etapa", "fase", "estagio"]),
    ("pipeline_id", &["funil", "pipeline"]),
    (
        "contact_id",
        &["contato", "email do contato", "e mail do contato"],
    ),
    (
        "expected_close_date",
        &["previsao de fechamento", "data de fechamento", "fechamento"],
    ),
    ("probability", &["probabilidade"]),
    ("description", &["descricao"]),
    ("currency", &["moeda"]),
];

type TargetMatcher = fn(&ImportTarget, &str) -> bool;

pub fn normalize_label(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        let folded = fold_accent(character);
        if folded.is_ascii_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            pending_space = false;
            normalized.push(folded);
        } else {
            pending_space = true;
        }
    }
    normalized
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

fn synonyms(key: &str) -> &'static [&'static str] {
    SYNONYMS
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, synonyms)| *synonyms)
        .unwrap_or(&[])
}

pub(crate) fn suggest_mapping(headers: &[String], targets: &[ImportTarget]) -> Vec<Option<String>> {
    let normalized: Vec<String> = headers
        .iter()
        .map(|header| normalize_label(header))
        .collect();
    let mut used = BTreeSet::new();
    let mut suggestions: Vec<Option<String>> = vec![None; headers.len()];

    let passes: [TargetMatcher; 3] = [
        |target, header| normalize_label(&target.label) == header,
        |target, header| normalize_label(&target.key) == header,
        |target, header| synonyms(&target.key).contains(&header),
    ];
    for matches in passes {
        for (index, header) in normalized.iter().enumerate() {
            if suggestions[index].is_some() || header.is_empty() {
                continue;
            }
            if let Some(target) = targets
                .iter()
                .find(|target| !used.contains(&target.key) && matches(target, header))
            {
                used.insert(target.key.clone());
                suggestions[index] = Some(target.key.clone());
            }
        }
    }
    suggestions
}
