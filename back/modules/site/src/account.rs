use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use aphel_contract_site::SiteCustomer;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::rate_limit::SharedRateLimiter;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::mail::EmailMessage;
use axum::body::Bytes;
use axum::http::request::Parts;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use minijinja::Value;
use serde_json::json;

use crate::customers::{self, SignUpError, TokenPurpose};
use crate::people_link;
use crate::runtime::SiteRuntime;
use crate::session;

pub(crate) const ACCOUNT_TEMPLATE: &str = "aphel/account.html";
const POSTS_PER_ADDRESS: u32 = 30;
const LOGINS_PER_EMAIL: u32 = 10;
const RATE_LIMITED: &str = "muitas tentativas em sequência; aguarde um pouco e tente novamente";

static ACCOUNT_POSTS: LazyLock<SharedRateLimiter> = LazyLock::new(|| {
    SharedRateLimiter::new(
        "site.account_per_address",
        POSTS_PER_ADDRESS,
        Duration::from_secs(60),
    )
});
static LOGIN_ATTEMPTS: LazyLock<SharedRateLimiter> = LazyLock::new(|| {
    SharedRateLimiter::new(
        "site.login_per_email",
        LOGINS_PER_EMAIL,
        Duration::from_secs(900),
    )
});

#[derive(Clone, Copy)]
pub(crate) struct AccountPaths {
    pub root: &'static str,
    login: &'static str,
    signup: &'static str,
    logout: &'static str,
    password: &'static str,
    link: &'static str,
    verify: &'static str,
    close: &'static str,
}

pub(crate) fn paths(locale: &str) -> AccountPaths {
    if locale.starts_with("pt") {
        AccountPaths {
            root: "conta",
            login: "entrar",
            signup: "criar",
            logout: "sair",
            password: "senha",
            link: "link",
            verify: "verificar",
            close: "encerrar",
        }
    } else {
        AccountPaths {
            root: "account",
            login: "login",
            signup: "signup",
            logout: "logout",
            password: "password",
            link: "link",
            verify: "verify",
            close: "close",
        }
    }
}

pub(crate) fn segments<'a>(paths: &AccountPaths, path: &'a str) -> Option<Vec<&'a str>> {
    let rest = path.trim_matches('/');
    let mut parts = rest.split('/');
    (parts.next() == Some(paths.root)).then(|| parts.filter(|part| !part.is_empty()).collect())
}

pub(crate) fn urls(prefix: &str, paths: &AccountPaths) -> serde_json::Value {
    let root = format!("{prefix}/{}", paths.root);
    json!({
        "home": root,
        "login": format!("{root}/{}", paths.login),
        "signup": format!("{root}/{}", paths.signup),
        "logout": format!("{root}/{}", paths.logout),
        "password": format!("{root}/{}", paths.password),
        "close": format!("{root}/{}", paths.close),
    })
}

fn texts(locale: &str) -> serde_json::Value {
    if locale.starts_with("pt") {
        texts_of(&[
            ("login_title", "Entrar"),
            ("email", "E-mail"),
            ("password", "Senha"),
            ("new_password", "Nova senha"),
            ("password_hint", "Pelo menos 8 caracteres."),
            ("login_submit", "Entrar"),
            ("forgot_link", "Esqueci a senha"),
            ("link_title", "Prefere entrar sem senha?"),
            ("link_submit", "Receber link de acesso por e-mail"),
            ("no_account", "Ainda não tem conta?"),
            ("signup_link", "Criar conta"),
            ("signup_title", "Criar conta"),
            ("name", "Nome"),
            ("signup_submit", "Criar conta"),
            ("has_account", "Já tem conta?"),
            ("login_link", "Entrar"),
            ("forgot_title", "Recuperar senha"),
            (
                "forgot_body",
                "Informe o e-mail da sua conta. Enviaremos um link para criar uma nova senha.",
            ),
            ("forgot_submit", "Enviar link"),
            ("reset_title", "Criar nova senha"),
            ("reset_submit", "Salvar nova senha"),
            ("magic_title", "Entrar na sua conta"),
            ("magic_submit", "Entrar agora"),
            ("home_title", "Minha conta"),
            ("unverified", "e-mail ainda não confirmado"),
            ("profile_title", "Seus dados"),
            ("save", "Salvar"),
            ("logout", "Sair"),
            ("close_title", "Encerrar conta"),
            (
                "close_body",
                "Seus dados de cadastro são apagados e você sai de todos os aparelhos. Pedidos e outros registros guardados pela empresa seguem as regras dela.",
            ),
            ("close_confirm", "Entendo que isso não pode ser desfeito"),
            ("close_submit", "Encerrar minha conta"),
            ("invalid_login", "E-mail ou senha incorretos."),
            (
                "email_taken",
                "Já existe uma conta com esse e-mail. Entre ou recupere a senha.",
            ),
            ("invalid_name", "Informe seu nome."),
            ("invalid_email", "Informe um e-mail válido."),
            (
                "invalid_password",
                "A senha precisa ter pelo menos 8 caracteres.",
            ),
            (
                "link_sent",
                "Se houver uma conta com esse e-mail, enviamos um link de acesso. Ele vale por 20 minutos.",
            ),
            (
                "reset_sent",
                "Se houver uma conta com esse e-mail, enviamos um link para criar uma nova senha. Ele vale por 1 hora.",
            ),
            (
                "mail_failed",
                "Não foi possível enviar o e-mail agora. Tente de novo mais tarde.",
            ),
            (
                "invalid_token",
                "Este link não é mais válido. Peça um novo.",
            ),
            ("verified", "E-mail confirmado. Obrigado!"),
            ("saved", "Dados salvos."),
            ("closed", "Sua conta foi encerrada."),
            (
                "forbidden",
                "Não foi possível confirmar de onde veio o envio. Recarregue a página e tente de novo.",
            ),
            ("mail_verify_subject", "Confirme seu e-mail"),
            (
                "mail_verify_body",
                "Para confirmar seu e-mail, abra o link abaixo.",
            ),
            ("mail_link_subject", "Seu link de acesso"),
            (
                "mail_link_body",
                "Para entrar na sua conta, abra o link abaixo. Ele vale por 20 minutos.",
            ),
            ("mail_reset_subject", "Criar nova senha"),
            (
                "mail_reset_body",
                "Para criar uma nova senha, abra o link abaixo. Ele vale por 1 hora. Se não foi você, ignore este e-mail.",
            ),
        ])
    } else {
        texts_of(&[
            ("login_title", "Sign in"),
            ("email", "Email"),
            ("password", "Password"),
            ("new_password", "New password"),
            ("password_hint", "At least 8 characters."),
            ("login_submit", "Sign in"),
            ("forgot_link", "Forgot your password?"),
            ("link_title", "Rather sign in without a password?"),
            ("link_submit", "Email me a sign-in link"),
            ("no_account", "No account yet?"),
            ("signup_link", "Create an account"),
            ("signup_title", "Create an account"),
            ("name", "Name"),
            ("signup_submit", "Create account"),
            ("has_account", "Already have an account?"),
            ("login_link", "Sign in"),
            ("forgot_title", "Reset your password"),
            (
                "forgot_body",
                "Enter the email of your account. We will send a link to create a new password.",
            ),
            ("forgot_submit", "Send link"),
            ("reset_title", "Create a new password"),
            ("reset_submit", "Save new password"),
            ("magic_title", "Sign in to your account"),
            ("magic_submit", "Sign in now"),
            ("home_title", "My account"),
            ("unverified", "email not confirmed yet"),
            ("profile_title", "Your details"),
            ("save", "Save"),
            ("logout", "Sign out"),
            ("close_title", "Close account"),
            (
                "close_body",
                "Your sign-up details are erased and you are signed out everywhere. Orders and other records the company keeps follow its own rules.",
            ),
            ("close_confirm", "I understand this cannot be undone"),
            ("close_submit", "Close my account"),
            ("invalid_login", "Wrong email or password."),
            (
                "email_taken",
                "An account with this email already exists. Sign in or reset the password.",
            ),
            ("invalid_name", "Enter your name."),
            ("invalid_email", "Enter a valid email."),
            (
                "invalid_password",
                "The password needs at least 8 characters.",
            ),
            (
                "link_sent",
                "If an account uses this email, we sent a sign-in link. It works for 20 minutes.",
            ),
            (
                "reset_sent",
                "If an account uses this email, we sent a link to create a new password. It works for 1 hour.",
            ),
            (
                "mail_failed",
                "The email could not be sent right now. Try again later.",
            ),
            (
                "invalid_token",
                "This link is no longer valid. Ask for a new one.",
            ),
            ("verified", "Email confirmed. Thank you!"),
            ("saved", "Details saved."),
            ("closed", "Your account was closed."),
            (
                "forbidden",
                "We could not confirm where the form came from. Reload the page and try again.",
            ),
            ("mail_verify_subject", "Confirm your email"),
            (
                "mail_verify_body",
                "To confirm your email, open the link below.",
            ),
            ("mail_link_subject", "Your sign-in link"),
            (
                "mail_link_body",
                "To sign in, open the link below. It works for 20 minutes.",
            ),
            ("mail_reset_subject", "Create a new password"),
            (
                "mail_reset_body",
                "To create a new password, open the link below. It works for 1 hour. If it was not you, ignore this email.",
            ),
        ])
    }
}

fn texts_of(pairs: &[(&str, &str)]) -> serde_json::Value {
    serde_json::Value::Object(
        pairs
            .iter()
            .map(|(key, value)| {
                (
                    (*key).to_owned(),
                    serde_json::Value::String((*value).to_owned()),
                )
            })
            .collect(),
    )
}

pub(crate) struct AccountRequest<'a> {
    pub state: &'a AppState,
    pub runtime: &'a SiteRuntime,
    pub site: serde_json::Value,
    pub locale: String,
    pub prefix: String,
    pub base_url: String,
    pub parts: Parts,
    pub body: Bytes,
    pub customer: Option<SiteCustomer>,
    pub client: Option<String>,
    pub links: Vec<serde_json::Value>,
}

struct View {
    name: &'static str,
    status: StatusCode,
    notice: Option<String>,
    error: Option<String>,
    heading: Option<String>,
    values: serde_json::Value,
}

impl View {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            status: StatusCode::OK,
            notice: None,
            error: None,
            heading: None,
            values: json!({}),
        }
    }

    fn message(heading: String) -> Self {
        Self {
            heading: Some(heading),
            ..Self::new("message")
        }
    }
}

fn text(texts: &serde_json::Value, key: &str) -> String {
    texts[key].as_str().unwrap_or_default().to_owned()
}

fn form(body: &Bytes) -> HashMap<String, String> {
    form_urlencoded::parse(body).into_owned().collect()
}

fn same_origin(parts: &Parts) -> bool {
    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_ascii_lowercase);
    let source = parts
        .headers
        .get(header::ORIGIN)
        .or_else(|| parts.headers.get(header::REFERER))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once("://").map(|(_, rest)| rest))
        .and_then(|rest| rest.split('/').next())
        .map(str::to_ascii_lowercase);
    host.is_some() && host == source
}

fn redirect(location: &str, cookie: Option<HeaderValue>) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(location) {
        headers.insert(header::LOCATION, value);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(cookie) = cookie {
        headers.insert(header::SET_COOKIE, cookie);
    }
    response
}

impl AccountRequest<'_> {
    fn home_url(&self, paths: &AccountPaths) -> String {
        format!("{}/{}", self.prefix, paths.root)
    }

    fn safe_next(&self, raw: Option<&str>, paths: &AccountPaths) -> String {
        let local = format!("{}/", self.prefix);
        raw.map(str::trim)
            .filter(|next| {
                next.starts_with(&local)
                    && !next.starts_with("//")
                    && !next.contains('\\')
                    && next.len() <= 512
            })
            .map(str::to_owned)
            .unwrap_or_else(|| self.home_url(paths))
    }

    fn query_next(&self) -> Option<String> {
        form_urlencoded::parse(self.parts.uri.query().unwrap_or_default().as_bytes())
            .find(|(name, _)| name == "next")
            .map(|(_, value)| value.into_owned())
    }

    fn render(&self, view: View, paths: &AccountPaths, next: &str) -> Response {
        let texts = texts(&self.locale);
        let values = json!({
            "site": self.site,
            "page": { "owner": "", "title": self.title(&view, &texts) },
            "customer": self.customer,
            "view": view.name,
            "texts": texts,
            "urls": urls(&self.prefix, paths),
            "next": next,
            "values": view.values,
            "notice": view.notice,
            "error": view.error,
            "heading": view.heading,
            "links": self.links,
        });
        let rendered = self
            .runtime
            .environment
            .get_template(ACCOUNT_TEMPLATE)
            .and_then(|template| template.render(Value::from_serialize(&values)));
        match rendered {
            Ok(html) => {
                let mut response = (view.status, html).into_response();
                let headers = response.headers_mut();
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/html; charset=utf-8"),
                );
                headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                if let Ok(csp) = HeaderValue::from_str(&self.runtime.content_security_policy(None))
                {
                    headers.insert(header::CONTENT_SECURITY_POLICY, csp);
                }
                headers.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                );
                headers.insert(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("same-origin"),
                );
                response
            }
            Err(error) => {
                tracing::warn!(error = %format!("{error:#}"), "could not render a site account page");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }

    fn title(&self, view: &View, texts: &serde_json::Value) -> String {
        let key = match view.name {
            "login" => "login_title",
            "signup" => "signup_title",
            "forgot" => "forgot_title",
            "reset" => "reset_title",
            "magic" => "magic_title",
            "home" => "home_title",
            _ => return view.heading.clone().unwrap_or_default(),
        };
        text(texts, key)
    }

    async fn send(&self, to: &str, subject_key: &str, body_key: &str, link: &str) -> bool {
        let texts = texts(&self.locale);
        let site_name = self.site["name"].as_str().unwrap_or_default();
        let body = text(&texts, body_key);
        let escape = |value: &str| {
            value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let message = EmailMessage {
            to: to.to_owned(),
            subject: format!("{} · {site_name}", text(&texts, subject_key)),
            text: format!("{body}\n\n{link}\n\n{site_name}"),
            html: format!(
                "<p>{}</p><p><a href=\"{}\">{}</a></p><p>{}</p>",
                escape(&body),
                escape(link),
                escape(link),
                escape(site_name)
            ),
            calendar: None,
        };
        match self.state.mailer().send(message).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, "could not send a site account email");
                false
            }
        }
    }

    fn absolute(&self, path: &str) -> String {
        format!("{}{path}", crate::handler::origin_of(&self.base_url))
    }

    async fn start_session(&self, customer_id: &str) -> Result<Option<HeaderValue>, AppError> {
        let agent = self
            .parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let token =
            customers::start_session(&self.state.db()?, customer_id, self.client.clone(), agent)
                .await?;
        Ok(session::set_cookie(self.state, &token))
    }
}

pub(crate) async fn handle(request: AccountRequest<'_>, segments: Vec<&str>) -> Response {
    let paths = paths(&request.locale);
    match serve(&request, &paths, &segments).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "site account request failed");
            let texts = texts(&request.locale);
            let mut view = View::message(text(&texts, "mail_failed"));
            view.status = StatusCode::INTERNAL_SERVER_ERROR;
            let next = request.home_url(&paths);
            request.render(view, &paths, &next)
        }
    }
}

async fn serve(
    request: &AccountRequest<'_>,
    paths: &AccountPaths,
    segments: &[&str],
) -> Result<Response, AppError> {
    let texts = texts(&request.locale);
    let pool = request.state.db()?;
    let method = request.parts.method.clone();
    let home = request.home_url(paths);
    let login_url = format!("{home}/{}", paths.login);
    if method == Method::POST {
        if !same_origin(&request.parts) {
            let mut view = View::message(text(&texts, "forbidden"));
            view.status = StatusCode::FORBIDDEN;
            return Ok(request.render(view, paths, &home));
        }
        let address = request.client.as_deref().unwrap_or("unknown");
        if ACCOUNT_POSTS.check(address, RATE_LIMITED).await.is_err() {
            return Ok((StatusCode::TOO_MANY_REQUESTS, RATE_LIMITED).into_response());
        }
    } else if method != Method::GET && method != Method::HEAD {
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
    }
    let fields = form(&request.body);
    let field = |name: &str| fields.get(name).map(String::as_str).unwrap_or_default();
    let posted_next = request.safe_next(fields.get("next").map(String::as_str), paths);
    let query_next = request.safe_next(request.query_next().as_deref(), paths);
    let is_post = method == Method::POST;
    match segments {
        [] if !is_post => Ok(match &request.customer {
            Some(_) => request.render(View::new("home"), paths, &home),
            None => redirect(&format!("{login_url}?next={}", urlencode(&home)), None),
        }),
        [] => {
            let Some(customer) = &request.customer else {
                return Ok(redirect(&login_url, None));
            };
            customers::rename(
                request.state,
                customer,
                field("name"),
                request.client.clone(),
            )
            .await?;
            Ok(redirect(&home, None))
        }
        [segment] if *segment == paths.login && !is_post => Ok(match request.customer {
            Some(_) => redirect(&query_next, None),
            None => request.render(View::new("login"), paths, &query_next),
        }),
        [segment] if *segment == paths.login => {
            let email = field("email").trim().to_owned();
            if field("mode") == "link" {
                let mut view = View::message(text(&texts, "link_sent"));
                if let Some(normalized) = customers::normalize_email(&email)
                    && let Some(customer) = customers::find_by_email(&pool, &normalized).await?
                {
                    let token =
                        customers::issue_token(&pool, &customer.id, TokenPurpose::Magic).await?;
                    let link = request.absolute(&format!(
                        "{home}/{}/{token}?next={}",
                        paths.link,
                        urlencode(&posted_next)
                    ));
                    if !request
                        .send(
                            &customer.email,
                            "mail_link_subject",
                            "mail_link_body",
                            &link,
                        )
                        .await
                    {
                        view = View::message(text(&texts, "mail_failed"));
                    }
                }
                return Ok(request.render(view, paths, &posted_next));
            }
            let key = email.to_ascii_lowercase();
            if LOGIN_ATTEMPTS.check(&key, RATE_LIMITED).await.is_err() {
                return Ok((StatusCode::TOO_MANY_REQUESTS, RATE_LIMITED).into_response());
            }
            match customers::authenticate(&pool, &email, field("password")).await? {
                Some(customer) => {
                    people_link::link(request.state, &customer.id).await;
                    let cookie = request.start_session(&customer.id).await?;
                    Ok(redirect(&posted_next, cookie))
                }
                None => {
                    let mut view = View::new("login");
                    view.status = StatusCode::UNPROCESSABLE_ENTITY;
                    view.error = Some(text(&texts, "invalid_login"));
                    view.values = json!({ "email": email });
                    Ok(request.render(view, paths, &posted_next))
                }
            }
        }
        [segment] if *segment == paths.signup && !is_post => {
            Ok(request.render(View::new("signup"), paths, &query_next))
        }
        [segment] if *segment == paths.signup => {
            let name = field("name").to_owned();
            let email = field("email").trim().to_owned();
            match customers::sign_up(
                request.state,
                &name,
                &email,
                Some(field("password")),
                request.client.clone(),
            )
            .await
            {
                Ok(customer) => {
                    let token =
                        customers::issue_token(&pool, &customer.id, TokenPurpose::Verify).await?;
                    let link = request.absolute(&format!("{home}/{}/{token}", paths.verify));
                    request
                        .send(
                            &customer.email,
                            "mail_verify_subject",
                            "mail_verify_body",
                            &link,
                        )
                        .await;
                    let cookie = request.start_session(&customer.id).await?;
                    Ok(redirect(&posted_next, cookie))
                }
                Err(SignUpError::Failed(error)) => Err(error),
                Err(problem) => {
                    let key = match problem {
                        SignUpError::EmailTaken => "email_taken",
                        SignUpError::Invalid("name") => "invalid_name",
                        SignUpError::Invalid("email") => "invalid_email",
                        _ => "invalid_password",
                    };
                    let mut view = View::new("signup");
                    view.status = StatusCode::UNPROCESSABLE_ENTITY;
                    view.error = Some(text(&texts, key));
                    view.values = json!({ "name": name, "email": email });
                    Ok(request.render(view, paths, &posted_next))
                }
            }
        }
        [segment] if *segment == paths.logout && is_post => {
            if let Some(token) = session::token(&request.parts.headers) {
                customers::end_session(&pool, &token).await?;
            }
            Ok(redirect(
                &format!("{}/", request.prefix),
                Some(session::clear_cookie()),
            ))
        }
        [segment] if *segment == paths.password && !is_post => {
            Ok(request.render(View::new("forgot"), paths, &home))
        }
        [segment] if *segment == paths.password => {
            let mut view = View::message(text(&texts, "reset_sent"));
            if let Some(email) = customers::normalize_email(field("email"))
                && let Some(customer) = customers::find_by_email(&pool, &email).await?
            {
                let token =
                    customers::issue_token(&pool, &customer.id, TokenPurpose::Reset).await?;
                let link = request.absolute(&format!("{home}/{}/{token}", paths.password));
                if !request
                    .send(
                        &customer.email,
                        "mail_reset_subject",
                        "mail_reset_body",
                        &link,
                    )
                    .await
                {
                    view = View::message(text(&texts, "mail_failed"));
                }
            }
            Ok(request.render(view, paths, &home))
        }
        [segment, token] if *segment == paths.password && !is_post => {
            let valid = customers::peek_token(&pool, token, TokenPurpose::Reset)
                .await?
                .is_some();
            Ok(request.render(
                if valid {
                    View::new("reset")
                } else {
                    View::message(text(&texts, "invalid_token"))
                },
                paths,
                &home,
            ))
        }
        [segment, token] if *segment == paths.password => {
            let password = field("password");
            if !customers::valid_password(password) {
                let mut view = View::new("reset");
                view.status = StatusCode::UNPROCESSABLE_ENTITY;
                view.error = Some(text(&texts, "invalid_password"));
                return Ok(request.render(view, paths, &home));
            }
            let Some(customer_id) =
                customers::redeem_token(&pool, token, TokenPurpose::Reset).await?
            else {
                return Ok(request.render(
                    View::message(text(&texts, "invalid_token")),
                    paths,
                    &home,
                ));
            };
            customers::set_password(
                request.state,
                &customer_id,
                password,
                request.client.clone(),
            )
            .await?;
            customers::mark_verified(&pool, &customer_id).await?;
            people_link::link(request.state, &customer_id).await;
            let cookie = request.start_session(&customer_id).await?;
            Ok(redirect(&home, cookie))
        }
        [segment, token] if *segment == paths.link && !is_post => {
            let valid = customers::peek_token(&pool, token, TokenPurpose::Magic)
                .await?
                .is_some();
            Ok(request.render(
                if valid {
                    View::new("magic")
                } else {
                    View::message(text(&texts, "invalid_token"))
                },
                paths,
                &query_next,
            ))
        }
        [segment, token] if *segment == paths.link => {
            let Some(customer_id) =
                customers::redeem_token(&pool, token, TokenPurpose::Magic).await?
            else {
                return Ok(request.render(
                    View::message(text(&texts, "invalid_token")),
                    paths,
                    &home,
                ));
            };
            customers::mark_verified(&pool, &customer_id).await?;
            people_link::link(request.state, &customer_id).await;
            let cookie = request.start_session(&customer_id).await?;
            Ok(redirect(&posted_next, cookie))
        }
        [segment, token] if *segment == paths.verify && !is_post => {
            match customers::redeem_token(&pool, token, TokenPurpose::Verify).await? {
                Some(customer_id) => {
                    customers::mark_verified(&pool, &customer_id).await?;
                    people_link::link(request.state, &customer_id).await;
                    Ok(request.render(View::message(text(&texts, "verified")), paths, &home))
                }
                None => {
                    Ok(request.render(View::message(text(&texts, "invalid_token")), paths, &home))
                }
            }
        }
        [segment] if *segment == paths.close && is_post => {
            let Some(customer) = &request.customer else {
                return Ok(redirect(&login_url, None));
            };
            if field("confirm") != "yes" {
                return Ok(redirect(&home, None));
            }
            let scope = customers::customer_scope(&customer.id);
            let ctx = aphel_kernel::Ctx::from_state(request.state, &scope).await?;
            customers::close(&ctx, &customer.id, request.client.clone()).await?;
            let mut response = request.render(View::message(text(&texts, "closed")), paths, &home);
            response
                .headers_mut()
                .insert(header::SET_COOKIE, session::clear_cookie());
            Ok(response)
        }
        _ => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

pub(crate) fn urlencode(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
