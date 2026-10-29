use aphel_kernel::application::fields::{RecordTypeDefinition, RecordTypeRegistry, SystemField};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::{access::Permission, fields::FieldType};
use aphel_kernel::records::snapshot;
use serde_json::json;
use sqlx::MySqlPool;
use uuid::Uuid;

#[tokio::test]
async fn snapshots_read_uncommitted_native_and_custom_values_without_changing_persisted_reads() {
    let url =
        std::env::var("APHEL_TEST_DATABASE_URL").expect("an isolated test database is required");
    let root = MySqlPool::connect(&url).await.unwrap();
    let database: String = sqlx::query_scalar("SELECT DATABASE()")
        .fetch_one(&root)
        .await
        .unwrap();
    assert!(database.ends_with("_test"));
    let name = format!("hook_snapshots_{}_test", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    )))
    .execute(&root)
    .await
    .unwrap();
    let options: sqlx::mysql::MySqlConnectOptions = url.parse().unwrap();
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect_with(options.database(&name).collation("utf8mb4_unicode_ci"))
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0400_create_field_definitions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0401_create_field_options.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0402_create_custom_field_values.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0403_create_custom_field_multi_values.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/1301_add_field_privacy_flags.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE sample_records (id CHAR(36) PRIMARY KEY, name VARCHAR(100), score INT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = Uuid::now_v7();
    let field = Uuid::now_v7();
    sqlx::query("INSERT INTO sample_records VALUES (?, 'before', 1)")
        .bind(id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO field_definitions (id, record_type, `key`, label, field_type) VALUES (?, 'contact', 'extra', 'Extra', 'text')").bind(field.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO custom_field_values (record_type, record_id, field_id, value_text) VALUES ('contact', ?, ?, 'before')").bind(id.to_string()).bind(field.to_string()).execute(&pool).await.unwrap();
    let definition = RecordTypeDefinition::new(
        RecordType::CONTACT,
        "sample_records",
        Permission::CONTACTS_READ,
    )
    .system_field(SystemField::new("name", "Name", FieldType::Text, "name").unwrap())
    .system_field(SystemField::new("score", "Score", FieldType::Number, "score").unwrap());
    let registry = RecordTypeRegistry::empty().register(definition).unwrap();
    let record = RecordRef::new(RecordType::CONTACT, id);
    let mut tx = MySqlTx::from(pool.begin().await.unwrap());
    let before = snapshot::capture(&mut tx, &pool, &registry, &record)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before["values"]["name"], "before");
    assert_eq!(before["custom_fields"]["extra"], "before");
    sqlx::query("UPDATE sample_records SET name = 'proposed', score = 2 WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE custom_field_values SET value_text = 'proposed' WHERE record_id = ?")
        .bind(id.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let proposed = snapshot::capture(&mut tx, &pool, &registry, &record)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(proposed["values"]["name"], "proposed");
    assert_eq!(proposed["values"]["score"], 2);
    assert_eq!(proposed["custom_fields"]["extra"], "proposed");
    let persisted: String = sqlx::query_scalar("SELECT name FROM sample_records WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(persisted, "before");
    let overlay = snapshot::proposed(
        Some(&before),
        json!({"name": "filtered"}).as_object().unwrap(),
        json!({}).as_object().unwrap(),
    );
    assert_eq!(overlay["values"]["name"], "filtered");
    assert_eq!(overlay["values"]["score"], 1);
    assert_eq!(overlay["custom_fields"]["extra"], "before");
    tx.rollback().await.unwrap();
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE `{name}`")))
        .execute(&root)
        .await
        .unwrap();
}
