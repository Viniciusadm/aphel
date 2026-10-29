use crate::application::mail::{EmailMessage, MailLocale};

pub fn email_verification(
    to: &str,
    display_name: &str,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    match locale {
        MailLocale::PtBr => render(
            to,
            "Confirme seu e-mail no Aphel",
            display_name,
            &[
                "Falta pouco para começar a usar o Aphel.",
                "Confirme que este endereço de e-mail é seu clicando no botão abaixo. O link vale por 48 horas.",
            ],
            "Confirmar e-mail",
            link,
            "Se você não criou uma conta no Aphel, ignore esta mensagem.",
            locale,
        ),
        MailLocale::En => render(
            to,
            "Confirm your email on Aphel",
            display_name,
            &[
                "You are almost ready to start using Aphel.",
                "Confirm that this email address is yours by clicking the button below. The link is valid for 48 hours.",
            ],
            "Confirm email",
            link,
            "If you did not create an Aphel account, ignore this message.",
            locale,
        ),
    }
}

pub fn password_reset(
    to: &str,
    display_name: &str,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    match locale {
        MailLocale::PtBr => render(
            to,
            "Redefinição de senha do Aphel",
            display_name,
            &[
                "Recebemos um pedido para redefinir a senha da sua conta.",
                "Clique no botão abaixo para escolher uma nova senha. O link vale por 1 hora e só pode ser usado uma vez.",
            ],
            "Redefinir senha",
            link,
            "Se você não pediu a redefinição, ignore esta mensagem; sua senha continua a mesma.",
            locale,
        ),
        MailLocale::En => render(
            to,
            "Reset your Aphel password",
            display_name,
            &[
                "We received a request to reset the password of your account.",
                "Click the button below to choose a new password. The link is valid for 1 hour and can be used only once.",
            ],
            "Reset password",
            link,
            "If you did not ask for a reset, ignore this message; your password stays the same.",
            locale,
        ),
    }
}

pub fn invitation(
    to: &str,
    inviter_name: &str,
    organization_name: &str,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    match locale {
        MailLocale::PtBr => {
            let invited_line = format!(
                "{inviter_name} convidou você para participar de {organization_name} no Aphel."
            );
            render(
                to,
                &format!("Convite para {organization_name} no Aphel"),
                "",
                &[
                    &invited_line,
                    "Clique no botão abaixo para aceitar o convite. O link vale por 7 dias.",
                ],
                "Aceitar convite",
                link,
                "Se você não esperava este convite, ignore esta mensagem.",
                locale,
            )
        }
        MailLocale::En => {
            let invited_line =
                format!("{inviter_name} invited you to join {organization_name} on Aphel.");
            render(
                to,
                &format!("Invitation to {organization_name} on Aphel"),
                "",
                &[
                    &invited_line,
                    "Click the button below to accept the invitation. The link is valid for 7 days.",
                ],
                "Accept invitation",
                link,
                "If you were not expecting this invitation, ignore this message.",
                locale,
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityNoticeDetails {
    pub moment: String,
    pub device: String,
    pub ip: String,
}

impl SecurityNoticeDetails {
    fn summary(&self, locale: MailLocale) -> String {
        match locale {
            MailLocale::PtBr => format!(
                "Quando: {}. Dispositivo: {}. IP: {}.",
                self.moment, self.device, self.ip
            ),
            MailLocale::En => format!(
                "When: {}. Device: {}. IP: {}.",
                self.moment, self.device, self.ip
            ),
        }
    }
}

fn security_action_label(locale: MailLocale) -> &'static str {
    locale.pick("Revisar a segurança da conta", "Review account security")
}

fn security_footer(locale: MailLocale) -> &'static str {
    locale.pick(
        "Este é um aviso automático de segurança do Aphel e não pode ser desativado.",
        "This is an automatic Aphel security notice and cannot be turned off.",
    )
}

#[allow(clippy::too_many_arguments)]
fn security_notice(
    to: &str,
    display_name: &str,
    details: &SecurityNoticeDetails,
    link: &str,
    locale: MailLocale,
    subject: &str,
    opening: &str,
    closing: &str,
) -> EmailMessage {
    let summary = details.summary(locale);
    render(
        to,
        subject,
        display_name,
        &[opening, &summary, closing],
        security_action_label(locale),
        link,
        security_footer(locale),
        locale,
    )
}

pub fn security_new_device(
    to: &str,
    display_name: &str,
    details: &SecurityNoticeDetails,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    security_notice(
        to,
        display_name,
        details,
        link,
        locale,
        locale.pick(
            "Novo acesso à sua conta do Aphel",
            "New sign-in to your Aphel account",
        ),
        locale.pick(
            "Sua conta do Aphel foi acessada a partir de um dispositivo ou rede que não reconhecemos.",
            "Your Aphel account was accessed from a device or network we do not recognize.",
        ),
        locale.pick(
            "Se foi você, nada precisa ser feito. Se não reconhece este acesso, troque sua senha e encerre as sessões que não são suas.",
            "If it was you, there is nothing to do. If you do not recognize this sign-in, change your password and end the sessions that are not yours.",
        ),
    )
}

pub fn security_password_changed(
    to: &str,
    display_name: &str,
    details: &SecurityNoticeDetails,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    security_notice(
        to,
        display_name,
        details,
        link,
        locale,
        locale.pick(
            "Sua senha do Aphel foi alterada",
            "Your Aphel password was changed",
        ),
        locale.pick(
            "A senha da sua conta do Aphel acabou de ser alterada.",
            "The password of your Aphel account was just changed.",
        ),
        locale.pick(
            "Se foi você, nada precisa ser feito. Se não reconhece esta alteração, redefina sua senha imediatamente pela opção \"Esqueci minha senha\" e fale com o administrador da sua organização.",
            "If it was you, there is nothing to do. If you do not recognize this change, reset your password right away with \"Forgot my password\" and contact the administrator of your organization.",
        ),
    )
}

pub fn security_two_factor_enabled(
    to: &str,
    display_name: &str,
    details: &SecurityNoticeDetails,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    security_notice(
        to,
        display_name,
        details,
        link,
        locale,
        locale.pick(
            "Verificação em duas etapas ativada no Aphel",
            "Two-step verification turned on in Aphel",
        ),
        locale.pick(
            "A verificação em duas etapas foi ativada na sua conta do Aphel. A partir de agora, além da senha, será pedido um código do seu aplicativo autenticador.",
            "Two-step verification was turned on for your Aphel account. From now on, besides the password, you will be asked for a code from your authenticator app.",
        ),
        locale.pick(
            "Se não foi você, troque sua senha e encerre as sessões que não são suas.",
            "If it was not you, change your password and end the sessions that are not yours.",
        ),
    )
}

pub fn security_two_factor_disabled(
    to: &str,
    display_name: &str,
    details: &SecurityNoticeDetails,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    security_notice(
        to,
        display_name,
        details,
        link,
        locale,
        locale.pick(
            "Verificação em duas etapas desativada no Aphel",
            "Two-step verification turned off in Aphel",
        ),
        locale.pick(
            "A verificação em duas etapas foi desativada na sua conta do Aphel. O acesso agora pede apenas a senha.",
            "Two-step verification was turned off for your Aphel account. Signing in now asks only for the password.",
        ),
        locale.pick(
            "Se não foi você, troque sua senha, encerre as sessões que não são suas e ative a verificação em duas etapas de novo.",
            "If it was not you, change your password, end the sessions that are not yours and turn two-step verification on again.",
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn render(
    to: &str,
    subject: &str,
    display_name: &str,
    paragraphs: &[&str],
    action_label: &str,
    link: &str,
    footer: &str,
    locale: MailLocale,
) -> EmailMessage {
    let greeting = locale.greeting(display_name);
    let signature = locale.signature();
    let fallback_hint = locale.pick(
        "Se o botão não funcionar, copie e cole este endereço no navegador:",
        "If the button does not work, copy and paste this address into your browser:",
    );

    let mut text = format!("{greeting}\n\n");
    for paragraph in paragraphs {
        text.push_str(paragraph);
        text.push_str("\n\n");
    }
    text.push_str(&format!(
        "{action_label}: {link}\n\n{footer}\n\n{signature}\n"
    ));

    let html_paragraphs: String = paragraphs
        .iter()
        .map(|paragraph| {
            format!(
                "<p style=\"margin:0 0 16px\">{}</p>",
                escape_html(paragraph)
            )
        })
        .collect();
    let html = format!(
        "<!doctype html><html lang=\"{lang}\"><body style=\"margin:0;padding:24px;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;color:#18181b\">\
<div style=\"max-width:520px;margin:0 auto;background:#ffffff;border-radius:8px;padding:32px\">\
<p style=\"margin:0 0 16px\">{greeting}</p>{html_paragraphs}\
<p style=\"margin:24px 0\"><a href=\"{link}\" style=\"background:#4f46e5;color:#ffffff;padding:12px 20px;border-radius:6px;text-decoration:none;display:inline-block\">{action}</a></p>\
<p style=\"margin:0 0 16px;font-size:13px;color:#52525b\">{hint}<br>{link}</p>\
<p style=\"margin:0;font-size:13px;color:#52525b\">{footer}</p>\
</div></body></html>",
        lang = locale.html_lang(),
        greeting = escape_html(&greeting),
        link = escape_html(link),
        action = escape_html(action_label),
        hint = escape_html(fallback_hint),
        footer = escape_html(footer),
    );

    EmailMessage {
        calendar: None,
        to: to.to_owned(),
        subject: subject.to_owned(),
        html,
        text,
    }
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
