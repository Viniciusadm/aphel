use crate::error::BlockError;

const MAX_SLUG_CHARS: usize = 80;
const RESERVED: &[&str] = &[
    "account", "api", "book", "conta", "de", "en", "es", "fr", "it", "p", "pt", "pt-br", "quote",
    "robots", "sitemap",
];

fn plain(character: char) -> Option<char> {
    let mapped = match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other if other.is_ascii_alphanumeric() => other.to_ascii_lowercase(),
        _ => return None,
    };
    Some(mapped)
}

pub fn suggest(title: &str, fallback: &str) -> String {
    let mut slug = String::new();
    for character in title.chars() {
        match plain(character) {
            Some(letter) => slug.push(letter),
            None if !slug.ends_with('-') && !slug.is_empty() => slug.push('-'),
            None => {}
        }
    }
    let slug: String = slug.chars().take(MAX_SLUG_CHARS).collect();
    let slug = slug.trim_matches('-').to_owned();
    match slug.is_empty() || RESERVED.contains(&slug.as_str()) {
        true => format!("{fallback}-{slug}")
            .trim_end_matches('-')
            .to_owned(),
        false => slug,
    }
}

pub fn check(slug: &str) -> Result<String, BlockError> {
    let slug = slug.trim().trim_matches('/').to_ascii_lowercase();
    let well_formed = !slug.is_empty()
        && slug.chars().count() <= MAX_SLUG_CHARS
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if !well_formed {
        return Err(BlockError::new(
            "slug_invalid",
            "slug",
            "the address uses lowercase letters, digits and hyphens, such as about-us",
        ));
    }
    if RESERVED.contains(&slug.as_str()) {
        return Err(BlockError::new(
            "slug_reserved",
            "slug",
            format!("/{slug} is used by the system; choose another address"),
        ));
    }
    Ok(slug)
}
