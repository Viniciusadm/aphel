#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MailLocale {
    #[default]
    PtBr,
    En,
}

impl MailLocale {
    pub fn parse(tag: &str) -> Option<Self> {
        let tag = tag.trim().to_ascii_lowercase();
        if tag == "en" || tag.starts_with("en-") {
            Some(Self::En)
        } else if tag == "pt" || tag.starts_with("pt-") {
            Some(Self::PtBr)
        } else {
            None
        }
    }

    pub fn resolve(preferred: Option<&str>, fallback: Option<&str>) -> Self {
        preferred
            .and_then(Self::parse)
            .or_else(|| fallback.and_then(Self::parse))
            .unwrap_or_default()
    }

    pub fn html_lang(&self) -> &'static str {
        match self {
            Self::PtBr => "pt-BR",
            Self::En => "en",
        }
    }

    pub fn pick<'a>(&self, pt_br: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::PtBr => pt_br,
            Self::En => en,
        }
    }

    pub fn greeting(&self, display_name: &str) -> String {
        let name = display_name.trim();
        match (self, name.is_empty()) {
            (Self::PtBr, true) => "Olá,".to_owned(),
            (Self::PtBr, false) => format!("Olá, {name},"),
            (Self::En, true) => "Hi,".to_owned(),
            (Self::En, false) => format!("Hi {name},"),
        }
    }

    pub fn signature(&self) -> &'static str {
        self.pick("— Equipe Aphel", "— The Aphel team")
    }

    pub fn date_format(&self) -> &'static str {
        self.pick("%d/%m/%Y", "%b %-d, %Y")
    }
}
