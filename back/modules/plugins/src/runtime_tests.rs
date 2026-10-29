use serde_json::{Value, json};
use sqlx::{MySqlPool, Row, types::Json};
use uuid::Uuid;

use crate::{effects, events, registry, runtime, schedule};
use aphel_kernel::application::shared::{Actor, DomainEvent, EventConsumer, JobContext};
use aphel_kernel::domain::{
    UserId,
    access::{AccessScope, PermissionSet, RoleKey},
};
use aphel_kernel::modules::{AppBuilder, install_app};
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::{AppState, install_runtime_state};

fn job(kind: &str) -> JobContext {
    JobContext {
        job_id: Uuid::now_v7(),
        queue: "default".to_owned(),
        kind: kind.to_owned(),
        attempt: 1,
        max_attempts: 5,
    }
}

async fn drain(pool: &MySqlPool) {
    for _ in 0..100 {
        let rows = sqlx::query("SELECT id, kind, payload FROM jobs WHERE finished_at IS NULL AND kind IN ('plugin.effect', 'plugin.committed') ORDER BY created_at, id")
            .fetch_all(pool).await.unwrap();
        if rows.is_empty() {
            return;
        }
        for row in rows {
            let id: String = row.try_get("id").unwrap();
            let kind: String = row.try_get("kind").unwrap();
            let Json(payload): Json<Value> = row.try_get("payload").unwrap();
            let handler = if kind == effects::RUN_JOB {
                effects::Run::shared(pool.clone())
            } else {
                crate::committed::Run::shared(pool.clone())
            };
            handler.handle(&job(&kind), payload).await.unwrap();
            sqlx::query(
                "UPDATE jobs SET finished_at = UTC_TIMESTAMP(6), unique_key = NULL WHERE id = ?",
            )
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        }
    }
    panic!("plugin effects did not converge");
}

async fn settings(pool: &MySqlPool, value: Value) {
    sqlx::query("UPDATE plugins_installed SET settings = ? WHERE plugin_key = 'contract_rust'")
        .bind(Json(value))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE plugins_state SET version = version + 1 WHERE id = 1")
        .execute(pool)
        .await
        .unwrap();
    registry::invalidate().await;
}

async fn install(pool: &MySqlPool, key: &str, bytes: Vec<u8>, user: &UserId, hooks: bool) {
    let mut manifest = json!({"key": key, "name": key, "version": "0.1.0", "api": 1,
        "requires": ["contacts.read", "contacts.create", "contacts.update", "contacts.delete"],
        "events": ["contact.created"], "schedules": [{"name": "tick", "cron": "0 * * * * *"}],
        "settings": [{"key": "patch", "label": "Patch", "type": "boolean"}, {"key": "reject", "label": "Reject", "type": "boolean"}, {"key": "fail", "label": "Fail", "type": "boolean"}]});
    if hooks {
        manifest["hooks"] = json!(
            [
                "record.saving",
                "record.validating",
                "record.saved",
                "record.committed"
            ]
            .iter()
            .map(|hook| json!({"hook": hook, "record_types": ["contact"]}))
            .collect::<Vec<_>>()
        );
    }
    let manifest: crate::manifest::PluginManifest = serde_json::from_value(manifest).unwrap();
    sqlx::query("INSERT INTO plugins_installed (plugin_key, name, version, manifest, checksum, installed_by) VALUES (?, ?, '0.1.0', ?, ?, ?)")
        .bind(key).bind(key).bind(Json(manifest)).bind(runtime::backend_hash(&bytes)).bind(user.as_str()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO plugins_files (plugin_key, path, content_type, content) VALUES (?, 'backend.wasm', 'application/wasm', ?)")
        .bind(key).bind(bytes).execute(pool).await.unwrap();
    sqlx::query("UPDATE plugins_state SET version = version + 1 WHERE id = 1")
        .execute(pool)
        .await
        .unwrap();
    registry::invalidate().await;
}

async fn request(
    pool: &MySqlPool,
    key: &str,
    scope: &AccessScope,
    body: Value,
) -> runtime::wit::Response {
    let snapshot = registry::current(pool).await.unwrap();
    let plugin = snapshot.plugin(key).unwrap();
    let request = runtime::wit::Request {
        method: "POST".to_owned(),
        path: "/contract".to_owned(),
        route: "/contract".to_owned(),
        params: vec![],
        query: vec![],
        headers: vec![],
        body_json: body.to_string(),
        actor: runtime::scope_actor(scope),
    };
    runtime::on_route(plugin, runtime::capability_scope(plugin, scope), &request)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated MySQL and compiled APHEL_RUST_PLUGIN_FIXTURE/APHEL_JS_PLUGIN_FIXTURE"]
async fn wasm_plugins_preserve_transactions_retries_snapshots_and_authorship() {
    let url = std::env::var("APHEL_TEST_DATABASE_URL").unwrap();
    let rust = std::fs::read(std::env::var("APHEL_RUST_PLUGIN_FIXTURE").unwrap()).unwrap();
    let js = std::fs::read(std::env::var("APHEL_JS_PLUGIN_FIXTURE").unwrap()).unwrap();
    let root = MySqlPool::connect(&url).await.unwrap();
    let database: String = sqlx::query_scalar("SELECT DATABASE()")
        .fetch_one(&root)
        .await
        .unwrap();
    assert!(database.ends_with("_test"));
    let name = format!("plugin_runtime_{}_test", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    )))
    .execute(&root)
    .await
    .unwrap();
    let options: sqlx::mysql::MySqlConnectOptions = url.parse().unwrap();
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(20)
        .connect_with(options.database(&name).collation("utf8mb4_unicode_ci"))
        .await
        .unwrap();
    let app = install_app(
        AppBuilder::new()
            .module(Box::new(aphel_kernel::KernelModule))
            .module(Box::new(tags::TagsModule))
            .module(Box::new(companies::CompaniesModule))
            .module(Box::new(contacts::ContactsModule))
            .module(Box::new(crate::PluginsModule))
            .module(Box::new(deals::DealsModule))
            .module(Box::new(activities::ActivitiesModule))
            .module(Box::new(currencies::CurrenciesModule))
            .module(Box::new(pipelines::PipelinesModule))
            .require(["plugins", "tags"])
            .build()
            .unwrap(),
    );
    aphel_kernel::database::migrations::MIGRATOR
        .run(&pool)
        .await
        .unwrap();
    aphel_kernel::modules::migrations::run(&pool, app)
        .await
        .unwrap();
    sqlx::query("INSERT INTO organization_settings (id, name) VALUES (1, 'Plugin tests')")
        .execute(&pool)
        .await
        .unwrap();
    let role = Uuid::now_v7();
    let user = UserId::new(Uuid::now_v7().to_string()).unwrap();
    sqlx::query(
        "INSERT INTO roles (id, `key`, name, is_system) VALUES (?, 'admin', 'Admin', TRUE)",
    )
    .bind(role.to_string())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO users (id, email, display_name, role_id) VALUES (?, 'plugins@example.test', 'Plugin tests', ?)")
        .bind(user.as_str()).bind(role.to_string()).execute(&pool).await.unwrap();
    let state = AppState::new(Some(pool.clone())).with_mysql_platform();
    install_runtime_state(state.clone());
    aphel_kernel::infrastructure::fields::sync_system_fields(
        &pool,
        &state.registry().await.unwrap(),
    )
    .await
    .unwrap();
    let scope = AccessScope::new(
        user.clone(),
        RoleKey::new("admin").unwrap(),
        vec![],
        PermissionSet::everything(),
    )
    .with_enabled_modules(app.enabled().snapshot());
    install(&pool, "contract_rust", rust, &user, true).await;
    install(&pool, "contract_js", js, &user, false).await;
    let snapshot = registry::current(&pool).await.unwrap();
    assert_eq!(registry::readiness().state, "ready");
    assert!(snapshot.plugin("contract_rust").unwrap().backend.is_some());
    let handler = state.registry().await.unwrap().handler("contact").unwrap();
    let created = handler
        .create(
            &RecordCtx::new(&scope, &state),
            json!({"first_name": "Original"}),
        )
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap();
    drain(&pool).await;
    let Json(saved): Json<Value> = sqlx::query_scalar("SELECT value FROM plugins_data WHERE plugin_key = 'contract_rust' AND collection = 'hooks' AND item_id = 'record.committed'").fetch_one(&pool).await.unwrap();
    assert!(saved["before"].is_null());
    assert_eq!(saved["persisted"]["values"]["first_name"], "Original");
    settings(&pool, json!({"patch": true})).await;
    handler
        .update(
            &RecordCtx::new(&scope, &state),
            Uuid::parse_str(id).unwrap(),
            json!({"first_name": "Requested"}),
        )
        .await
        .unwrap();
    drain(&pool).await;
    let Json(validated): Json<Value> = sqlx::query_scalar("SELECT value FROM plugins_data WHERE plugin_key = 'contract_rust' AND collection = 'hooks' AND item_id = 'record.validating'").fetch_one(&pool).await.unwrap();
    assert_eq!(validated["before"]["values"]["first_name"], "Original");
    assert_eq!(validated["proposed"]["values"]["first_name"], "Filtered");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugins_effects")
        .fetch_one(&pool)
        .await
        .unwrap();
    settings(&pool, json!({"reject": true})).await;
    assert!(
        handler
            .create(
                &RecordCtx::new(&scope, &state),
                json!({"first_name": "Rejected"})
            )
            .await
            .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugins_effects")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, after);
    settings(&pool, json!({"fail": true})).await;
    let event = DomainEvent::new(
        Actor::User(user.clone()),
        "contact.created",
        chrono::Utc::now(),
    );
    let consumer = events::PluginEventsConsumer::new(pool.clone());
    consumer.consume(&event).await.unwrap();
    consumer.consume(&event).await.unwrap();
    let rows = sqlx::query("SELECT id, plugin_key FROM plugins_executions WHERE source_key = ?")
        .bind(format!("event:{}", event.id))
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let id: String = row.try_get("id").unwrap();
        let key: String = row.try_get("plugin_key").unwrap();
        let result = events::Run::shared(pool.clone())
            .handle(&job(events::RUN_JOB), json!({"id": id}))
            .await;
        assert_eq!(result.is_err(), key == "contract_rust");
    }
    settings(&pool, json!({})).await;
    for row in rows {
        let id: String = row.try_get("id").unwrap();
        events::Run::shared(pool.clone())
            .handle(&job(events::RUN_JOB), json!({"id": id}))
            .await
            .unwrap();
    }
    drain(&pool).await;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM plugins_effects WHERE effect_key = 'event-write'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    for key in ["contract_rust", "contract_js"] {
        let response = request(&pool, key, &scope, json!({"command": {"kind": "create", "record_type": "contact", "values": {"first_name": key}}})).await;
        assert_eq!(response.status, 202);
        let body: Value = serde_json::from_str(&response.body_json).unwrap();
        assert_eq!(body["execution"]["executor"], key);
        assert_eq!(body["execution"]["initiator"]["id"], user.as_str());
        drain(&pool).await;
        effects::Run::shared(pool.clone())
            .handle(&job(effects::RUN_JOB), json!({"id": body["id"]}))
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contacts WHERE first_name = ?")
            .bind(key)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
        let row = sqlx::query("SELECT actor_type, execution FROM audit_logs WHERE action = 'contact.created' AND actor_id = ? ORDER BY occurred_at DESC LIMIT 1").bind(key).fetch_one(&pool).await.unwrap();
        assert_eq!(row.try_get::<String, _>("actor_type").unwrap(), "plugin");
        let Json(execution): Json<Value> = row.try_get("execution").unwrap();
        assert_eq!(execution["initiator"]["id"], user.as_str());
    }
    let payload =
        json!({"plugin": "contract_rust", "name": "tick", "slot": "2026-10-29T12:00:00Z"});
    for _ in 0..2 {
        schedule::Run::shared(pool.clone())
            .handle(&job(schedule::RUN_JOB), payload.clone())
            .await
            .unwrap();
    }
    drain(&pool).await;
    let Json(schedule): Json<Value> = sqlx::query_scalar("SELECT value FROM plugins_data WHERE plugin_key = 'contract_rust' AND collection = 'schedules'").fetch_one(&pool).await.unwrap();
    assert_eq!(schedule["trigger"], "schedule");
    assert!(schedule["initiator"].is_null());
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE `{name}`")))
        .execute(&root)
        .await
        .unwrap();
}
