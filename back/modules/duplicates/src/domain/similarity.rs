pub(crate) const NAME_SIMILARITY_THRESHOLD: f64 = 0.85;

const COMPANY_SUFFIXES: &[&str] = &[
    "ltda", "me", "mei", "epp", "eireli", "sa", "s a", "cia", "inc", "llc", "ltd", "corp",
];

pub(crate) use ::search::application::text::normalize_name;

pub(crate) fn normalize_company_name(value: &str) -> String {
    let mut name = normalize_name(value);
    loop {
        let trimmed = COMPANY_SUFFIXES.iter().find_map(|suffix| {
            name.strip_suffix(suffix)
                .filter(|rest| rest.ends_with(' '))
                .map(|rest| rest.trim_end().to_owned())
        });
        match trimmed {
            Some(rest) if !rest.is_empty() => name = rest,
            _ => return name,
        }
    }
}

fn levenshtein(left: &[char], right: &[char]) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (i, left_char) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

pub(crate) fn similarity(left: &str, right: &str) -> f64 {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let longest = left.len().max(right.len());
    if longest == 0 {
        return 0.0;
    }
    1.0 - levenshtein(&left, &right) as f64 / longest as f64
}

fn sorted_tokens(value: &str) -> String {
    let mut tokens: Vec<&str> = value.split_whitespace().collect();
    tokens.sort_unstable();
    tokens.join(" ")
}

pub(crate) fn normalized_names_similar(left: &str, right: &str) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }
    similarity(left, right) >= NAME_SIMILARITY_THRESHOLD
        || similarity(&sorted_tokens(left), &sorted_tokens(right)) >= NAME_SIMILARITY_THRESHOLD
}
