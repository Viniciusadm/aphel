const FIRST_WEIGHTS: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const SECOND_WEIGHTS: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

fn char_value(character: char) -> Option<u32> {
    match character {
        '0'..='9' | 'A'..='Z' => Some(character as u32 - '0' as u32),
        _ => None,
    }
}

fn check_digit(values: &[u32], weights: &[u32]) -> u32 {
    let sum: u32 = values
        .iter()
        .zip(weights)
        .map(|(value, weight)| value * weight)
        .sum();
    let remainder = sum % 11;
    if remainder < 2 { 0 } else { 11 - remainder }
}

pub(crate) fn normalize(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|character| !matches!(character, '.' | '/' | '-' | ' '))
        .map(|character| character.to_ascii_uppercase())
        .collect();
    if cleaned.len() != 14 {
        return None;
    }
    let characters: Vec<char> = cleaned.chars().collect();
    if !characters[12..].iter().all(char::is_ascii_digit) {
        return None;
    }
    let values: Vec<u32> = characters
        .iter()
        .map(|character| char_value(*character))
        .collect::<Option<Vec<u32>>>()?;
    if characters
        .iter()
        .all(|character| *character == characters[0])
    {
        return None;
    }
    let first = check_digit(&values[..12], &FIRST_WEIGHTS);
    let second = check_digit(&values[..13], &SECOND_WEIGHTS);
    (values[12] == first && values[13] == second).then_some(cleaned)
}

pub(crate) fn format(cnpj: &str) -> String {
    if cnpj.len() != 14 {
        return cnpj.to_owned();
    }
    format!(
        "{}.{}.{}/{}-{}",
        &cnpj[0..2],
        &cnpj[2..5],
        &cnpj[5..8],
        &cnpj[8..12],
        &cnpj[12..14]
    )
}
