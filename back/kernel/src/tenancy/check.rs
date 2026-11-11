use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, StatusCode, header};
use futures::StreamExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::application::installer::InstallCommand;
use crate::config::AppConfig;
use crate::domain::Secret;
use crate::errors::AppError;
use crate::tenancy::{
    ControlPlane, OperatorActor, Tenant, TenantDirectory, TenantLocal, TenantRegistration,
    TenantRuntime, TenantRuntimes, begin_deletion, create_tenant, delete_tenant, resume_tenant,
    scope, suspend_tenant,
};

const HOST_DOMAIN: &str = "tenancy-check.invalid";
const MARKER_KEY: &str = "tenancy-check/marker";
const BODY_LIMIT: usize = 4 * 1024 * 1024;

static PROBE: TenantLocal<AtomicU64> = TenantLocal::new(AtomicU64::default);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IsolationReport {
    pub checks: Vec<CheckOutcome>,
}

impl IsolationReport {
    pub fn passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|check| check.passed)
    }

    fn record(&mut self, name: &'static str, passed: bool, detail: impl Into<String>) {
        self.checks.push(CheckOutcome {
            name,
            passed,
            detail: detail.into(),
        });
    }

    fn record_result(&mut self, name: &'static str, result: Result<String, String>) {
        match result {
            Ok(detail) => self.record(name, true, detail),
            Err(detail) => self.record(name, false, detail),
        }
    }
}

struct Side {
    tenant: Tenant,
    host: String,
    email: String,
    password: String,
}

struct Context {
    control: ControlPlane,
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
    router: Router,
    module: Option<&'static str>,
    marker: String,
}

pub async fn run_isolation_check(
    control: &ControlPlane,
    config: &AppConfig,
) -> Result<IsolationReport, AppError> {
    control.migrate().await?;
    let suffix = Uuid::now_v7().simple().to_string();
    let suffix = &suffix[suffix.len() - 8..];
    let slugs = [
        format!("isocheck-{suffix}-a"),
        format!("isocheck-{suffix}-b"),
    ];
    let module = crate::modules::app()
        .modules()
        .iter()
        .find(|entry| {
            !entry.manifest.required
                && entry.is_compatible()
                && entry.manifest.dependencies.is_empty()
        })
        .map(|entry| entry.key());
    let outcome = run(control, config, &slugs, module, suffix).await;
    cleanup(control, config, &slugs).await;
    outcome
}

async fn run(
    control: &ControlPlane,
    config: &AppConfig,
    slugs: &[String; 2],
    module: Option<&'static str>,
    suffix: &str,
) -> Result<IsolationReport, AppError> {
    let mut sides = Vec::new();
    for (index, slug) in slugs.iter().enumerate() {
        let host = format!("{slug}.{HOST_DOMAIN}");
        let email = format!("admin-{slug}@{HOST_DOMAIN}");
        let password = Uuid::now_v7().simple().to_string();
        tracing::info!(tenant = slug, "creating a throwaway tenant");
        let tenant = create_tenant(
            control,
            config,
            &OperatorActor::command_line(),
            TenantRegistration {
                slug: slug.clone(),
                name: format!("Isolation check {}", if index == 0 { "A" } else { "B" }),
                hosts: vec![host.clone()],
            },
            InstallCommand {
                organization_name: format!("Isolation check {slug}"),
                timezone: "UTC".to_owned(),
                currency: "USD".to_owned(),
                locale: "en".to_owned(),
                admin_name: "Isolation check".to_owned(),
                admin_email: email.clone(),
                admin_password: Secret::new(password.clone()),
                public_base_url: Some(format!("https://{host}")),
                business: None,
                ip: None,
                modules: if index == 0 {
                    module.map(str::to_owned).into_iter().collect()
                } else {
                    Vec::new()
                },
                module_settings: Default::default(),
            },
        )
        .await?;
        sides.push(Side {
            tenant,
            host,
            email,
            password,
        });
    }
    let directory = TenantDirectory::load(control.pool().clone()).await?;
    let runtimes = TenantRuntimes::new(control.clone(), config.clone());
    let context = Context {
        control: control.clone(),
        router: crate::tenancy::router(control, config, directory.clone(), runtimes.clone()),
        directory,
        runtimes: runtimes.clone(),
        module,
        marker: format!("Isolation marker {suffix}"),
    };
    let report = checks(&context, &sides[0], &sides[1]).await;
    runtimes.clear();
    report
}

async fn cleanup(control: &ControlPlane, config: &AppConfig, slugs: &[String; 2]) {
    let actor = OperatorActor::command_line();
    for slug in slugs {
        let Ok(Some(_)) = control.find_tenant(slug).await else {
            continue;
        };
        tracing::info!(tenant = slug, "deleting the throwaway tenant");
        let result = match begin_deletion(control, &actor, slug, None).await {
            Ok(tenant) => delete_tenant(control, config, &actor, &tenant, None)
                .await
                .map(|_| ()),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            tracing::error!(
                tenant = slug,
                "could not delete the throwaway tenant: {error}"
            );
        }
    }
}

async fn runtime(context: &Context, side: &Side) -> Result<Arc<TenantRuntime>, AppError> {
    context.runtimes.get(&Arc::new(side.tenant.clone())).await
}

async fn checks(context: &Context, a: &Side, b: &Side) -> Result<IsolationReport, AppError> {
    let mut report = IsolationReport::default();
    let runtime_a = runtime(context, a).await?;
    let runtime_b = runtime(context, b).await?;

    report.record_result(
        "database per tenant",
        database_check(&runtime_a, &runtime_b, a, b).await,
    );
    report.record_result(
        "fail closed outside a tenant",
        outside_scope_check(&runtime_a).await,
    );
    report.record_result(
        "tenant-local state",
        local_state_check(&runtime_a, &runtime_b).await,
    );
    report.record_result(
        "enabled modules",
        modules_check(context, &runtime_a, &runtime_b).await,
    );
    report.record_result("stored files", files_check(&runtime_a, &runtime_b).await);
    report.record_result("unknown hosts", unknown_host_check(context).await);
    let sessions = sessions_check(context, a, b).await;
    let cookies = sessions.as_ref().ok().map(|(_, cookies)| cookies.clone());
    report.record_result("sessions", sessions.map(|(detail, _)| detail));
    match cookies {
        Some((cookie_a, cookie_b)) => {
            let records = records_check(context, a, b, &cookie_a, &cookie_b).await;
            let team_id = records.as_ref().ok().map(|(_, id)| id.clone());
            report.record_result("records and lists", records.map(|(detail, _)| detail));
            match team_id {
                Some(team_id) => report.record_result(
                    "events",
                    events_check(&runtime_a, &runtime_b, &team_id).await,
                ),
                None => report.record("events", false, "skipped: the team was not created"),
            }
            report.record_result(
                "suspended tenants",
                suspension_check(context, b, &cookie_b).await,
            );
        }
        None => {
            for name in ["records and lists", "events", "suspended tenants"] {
                report.record(name, false, "skipped: signing in failed");
            }
        }
    }
    Ok(report)
}

async fn scoped_database(runtime: &Arc<TenantRuntime>) -> Result<String, String> {
    scope(runtime.clone(), async {
        let pool = crate::state::runtime_pool().map_err(|error| error.to_string())?;
        sqlx::query_scalar::<_, Option<String>>("SELECT DATABASE()")
            .fetch_one(&pool)
            .await
            .map_err(|error| error.to_string())
            .map(Option::unwrap_or_default)
    })
    .await
}

async fn database_check(
    runtime_a: &Arc<TenantRuntime>,
    runtime_b: &Arc<TenantRuntime>,
    a: &Side,
    b: &Side,
) -> Result<String, String> {
    let database_a = scoped_database(runtime_a).await?;
    let database_b = scoped_database(runtime_b).await?;
    if database_a == a.tenant.database_name && database_b == b.tenant.database_name {
        Ok(format!("{database_a} and {database_b}"))
    } else {
        Err(format!(
            "tenant A reached {database_a:?} and tenant B reached {database_b:?}"
        ))
    }
}

async fn outside_scope_check(runtime_a: &Arc<TenantRuntime>) -> Result<String, String> {
    if crate::state::runtime_state().is_ok() {
        return Err("runtime_state() answered outside a tenant scope".to_owned());
    }
    if PROBE.get().is_some() {
        return Err("tenant-local state answered outside a tenant scope".to_owned());
    }
    let storage = runtime_a
        .state()
        .ok_or("the tenant state is not ready")?
        .storage
        .clone();
    if storage
        .put("tenancy-check/outside", Bytes::from_static(b"outside"))
        .await
        .is_ok()
    {
        return Err("file storage accepted a key outside a tenant scope".to_owned());
    }
    Ok("runtime state, tenant-local state and file storage refuse".to_owned())
}

async fn local_state_check(
    runtime_a: &Arc<TenantRuntime>,
    runtime_b: &Arc<TenantRuntime>,
) -> Result<String, String> {
    scope(runtime_a.clone(), async {
        if let Some(probe) = PROBE.get() {
            probe.store(7, Ordering::SeqCst);
        }
    })
    .await;
    let seen_by_b = scope(runtime_b.clone(), async {
        PROBE.get().map(|probe| probe.load(Ordering::SeqCst))
    })
    .await;
    match seen_by_b {
        Some(0) => Ok("a value written by A is not visible to B".to_owned()),
        Some(value) => Err(format!("B saw {value} written by A")),
        None => Err("tenant-local state is unavailable inside a scope".to_owned()),
    }
}

async fn modules_check(
    context: &Context,
    runtime_a: &Arc<TenantRuntime>,
    runtime_b: &Arc<TenantRuntime>,
) -> Result<String, String> {
    let Some(module) = context.module else {
        return Ok("skipped: this build has no optional module without dependencies".to_owned());
    };
    let app = crate::modules::app();
    let in_a = scope(runtime_a.clone(), async { app.is_enabled(module) }).await;
    let in_b = scope(runtime_b.clone(), async { app.is_enabled(module) }).await;
    if in_a && !in_b {
        Ok(format!("`{module}` is enabled in A only"))
    } else {
        Err(format!("`{module}` enabled in A: {in_a}, in B: {in_b}"))
    }
}

async fn read_file(runtime: &Arc<TenantRuntime>, key: &str) -> Result<Option<Vec<u8>>, String> {
    let storage = runtime
        .state()
        .ok_or("the tenant state is not ready")?
        .storage
        .clone();
    let key = key.to_owned();
    scope(runtime.clone(), async move {
        match storage.get_stream(&key).await {
            Ok(mut stream) => {
                let mut content = Vec::new();
                while let Some(chunk) = stream.next().await {
                    content.extend_from_slice(&chunk.map_err(|error| error.to_string())?);
                }
                Ok(Some(content))
            }
            Err(AppError::NotFound(_)) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    })
    .await
}

async fn write_file(runtime: &Arc<TenantRuntime>, content: &'static [u8]) -> Result<(), String> {
    let storage = runtime
        .state()
        .ok_or("the tenant state is not ready")?
        .storage
        .clone();
    scope(runtime.clone(), async move {
        storage
            .put(MARKER_KEY, Bytes::from_static(content))
            .await
            .map_err(|error| error.to_string())
    })
    .await
}

async fn files_check(
    runtime_a: &Arc<TenantRuntime>,
    runtime_b: &Arc<TenantRuntime>,
) -> Result<String, String> {
    write_file(runtime_a, b"tenant a").await?;
    if let Some(content) = read_file(runtime_b, MARKER_KEY).await? {
        return Err(format!(
            "B read {} bytes stored by A under the same key",
            content.len()
        ));
    }
    write_file(runtime_b, b"tenant b").await?;
    match read_file(runtime_a, MARKER_KEY).await? {
        Some(content) if content == b"tenant a" => {
            Ok("the same key holds a different file per tenant".to_owned())
        }
        Some(_) => Err("A read the file stored by B under the same key".to_owned()),
        None => Err("A lost its own file".to_owned()),
    }
}

struct Reply {
    status: StatusCode,
    cookie: Option<String>,
    body: Value,
}

async fn send(
    context: &Context,
    host: &str,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Result<Reply, String> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, host)
        .header("x-requested-with", "aphel")
        .header(header::ACCEPT, "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let request_body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let mut request = builder
        .body(request_body)
        .map_err(|error| error.to_string())?;
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let response = match context.router.clone().oneshot(request).await {
        Ok(response) => response,
        Err(infallible) => match infallible {},
    };
    let status = response.status();
    let cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("aphel_session="))
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    let bytes = axum::body::to_bytes(response.into_body(), BODY_LIMIT)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Reply {
        status,
        cookie,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    })
}

async fn sign_in(context: &Context, side: &Side) -> Result<String, String> {
    let reply = send(
        context,
        &side.host,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({ "email": side.email, "password": side.password })),
    )
    .await?;
    reply.cookie.ok_or_else(|| {
        format!(
            "signing in to {} answered {} without a session",
            side.tenant.slug, reply.status
        )
    })
}

async fn sessions_check(
    context: &Context,
    a: &Side,
    b: &Side,
) -> Result<(String, (String, String)), String> {
    let cookie_a = sign_in(context, a).await?;
    let cookie_b = sign_in(context, b).await?;
    let own = send(
        context,
        &a.host,
        Method::GET,
        "/api/v1/auth/me",
        Some(&cookie_a),
        None,
    )
    .await?;
    if own.status != StatusCode::OK {
        return Err(format!("A's session on A answered {}", own.status));
    }
    let foreign = send(
        context,
        &b.host,
        Method::GET,
        "/api/v1/auth/me",
        Some(&cookie_a),
        None,
    )
    .await?;
    if foreign.status != StatusCode::UNAUTHORIZED {
        return Err(format!("A's session on B answered {}", foreign.status));
    }
    Ok((
        "a session of A is refused on B".to_owned(),
        (cookie_a, cookie_b),
    ))
}

fn mentions(value: &Value, marker: &str) -> bool {
    value.to_string().contains(marker)
}

async fn records_check(
    context: &Context,
    a: &Side,
    b: &Side,
    cookie_a: &str,
    cookie_b: &str,
) -> Result<(String, String), String> {
    let created = send(
        context,
        &a.host,
        Method::POST,
        "/api/v1/teams",
        Some(cookie_a),
        Some(json!({ "name": context.marker })),
    )
    .await?;
    if !created.status.is_success() {
        return Err(format!("creating a team on A answered {}", created.status));
    }
    let team_id = created
        .body
        .pointer("/data/id")
        .or_else(|| created.body.pointer("/id"))
        .and_then(Value::as_str)
        .ok_or("creating a team on A answered without an id")?
        .to_owned();
    let listed_a = send(
        context,
        &a.host,
        Method::GET,
        "/api/v1/teams",
        Some(cookie_a),
        None,
    )
    .await?;
    if !mentions(&listed_a.body, &context.marker) {
        return Err("A does not list its own team".to_owned());
    }
    let listed_b = send(
        context,
        &b.host,
        Method::GET,
        "/api/v1/teams",
        Some(cookie_b),
        None,
    )
    .await?;
    if !listed_b.status.is_success() {
        return Err(format!("listing teams on B answered {}", listed_b.status));
    }
    if mentions(&listed_b.body, &context.marker) {
        return Err("B lists the team created on A".to_owned());
    }
    Ok((
        "a team created on A is listed on A only".to_owned(),
        team_id,
    ))
}

async fn team_events(runtime: &Arc<TenantRuntime>, team_id: &str) -> Result<i64, String> {
    let team_id = team_id.to_owned();
    scope(runtime.clone(), async move {
        let pool = crate::state::runtime_pool().map_err(|error| error.to_string())?;
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM domain_events WHERE record_id = ?")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .map_err(|error| error.to_string())
    })
    .await
}

async fn events_check(
    runtime_a: &Arc<TenantRuntime>,
    runtime_b: &Arc<TenantRuntime>,
    team_id: &str,
) -> Result<String, String> {
    let in_a = team_events(runtime_a, team_id).await?;
    let in_b = team_events(runtime_b, team_id).await?;
    if in_a == 0 {
        return Err("A holds no event about the team it created".to_owned());
    }
    if in_b > 0 {
        return Err(format!("B holds {in_b} events about A's team"));
    }
    Ok(format!(
        "A holds {in_a} events about its team, B holds none"
    ))
}

async fn unknown_host_check(context: &Context) -> Result<String, String> {
    let host = format!("missing-{}.{HOST_DOMAIN}", Uuid::now_v7().simple());
    let reply = send(context, &host, Method::GET, "/api/v1/auth/me", None, None).await?;
    if reply.status == StatusCode::NOT_FOUND {
        Ok("an unknown host answers 404".to_owned())
    } else {
        Err(format!("an unknown host answered {}", reply.status))
    }
}

async fn suspension_check(context: &Context, b: &Side, cookie_b: &str) -> Result<String, String> {
    let actor = OperatorActor::command_line();
    suspend_tenant(&context.control, &actor, &b.tenant.slug)
        .await
        .map_err(|error| error.to_string())?;
    let refreshed = context.directory.refresh().await;
    context.runtimes.retain(&context.directory);
    let reply = match refreshed {
        Ok(()) => {
            send(
                context,
                &b.host,
                Method::GET,
                "/api/v1/auth/me",
                Some(cookie_b),
                None,
            )
            .await
        }
        Err(error) => Err(error.to_string()),
    };
    resume_tenant(&context.control, &actor, &b.tenant.slug)
        .await
        .map_err(|error| error.to_string())?;
    let reply = reply?;
    if reply.status == StatusCode::SERVICE_UNAVAILABLE {
        Ok("a suspended tenant answers 503 even with a valid session".to_owned())
    } else {
        Err(format!("a suspended tenant answered {}", reply.status))
    }
}
