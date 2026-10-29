use aphel_kernel::database::receipt::{self, MutationReceipt};
use aphel_kernel::errors::AppError;
use sqlx::MySqlPool;
use uuid::Uuid;

#[tokio::test]
async fn mutation_receipts_survive_retries_without_duplicate_writes() {
    let url = std::env::var("APHEL_TEST_DATABASE_URL")
        .expect("APHEL_TEST_DATABASE_URL must point to an isolated test database");
    let pool = MySqlPool::connect(&url).await.unwrap();
    let database: String = sqlx::query_scalar("SELECT DATABASE()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(database.ends_with("_test"));
    sqlx::query("CREATE TABLE IF NOT EXISTS mutation_receipts (id CHAR(36) PRIMARY KEY, committed_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6))").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS receipt_test_records (id CHAR(36) PRIMARY KEY, value INT NOT NULL)").execute(&pool).await.unwrap();
    let id = Uuid::now_v7();
    let rolled_back: Result<(), AppError> = MutationReceipt::scope(id, async {
        let mut tx = receipt::begin(&pool).await?;
        sqlx::query("INSERT INTO receipt_test_records VALUES (?, 1)")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        tx.rollback().await?;
        Ok(())
    })
    .await;
    rolled_back.unwrap();
    assert!(!receipt::applied(&pool, id).await.unwrap());
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let execute = |pool: MySqlPool| async move {
        MutationReceipt::scope(id, async {
            let mut tx = receipt::begin(&pool).await?;
            sqlx::query("INSERT INTO receipt_test_records VALUES (?, 1)")
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            Ok::<_, AppError>(())
        })
        .await
    };
    let (first, second) = tokio::join!(execute(first_pool), execute(second_pool));
    assert_ne!(first.is_ok(), second.is_ok());
    assert!(receipt::applied(&pool, id).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM receipt_test_records WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(execute(pool.clone()).await.is_err());
    let rejected: Result<(), AppError> = MutationReceipt::scope(Uuid::now_v7(), async {
        let tx = receipt::begin(&pool).await?;
        assert!(receipt::begin(&pool).await.is_err());
        tx.rollback().await?;
        Ok(())
    })
    .await;
    rejected.unwrap();
    let restarted_id = Uuid::now_v7();
    let restarted: Result<(), AppError> = MutationReceipt::scope(restarted_id, async {
        let tx = receipt::begin(&pool).await?;
        tx.rollback().await?;
        let mut tx = receipt::begin(&pool).await?;
        sqlx::query("INSERT INTO receipt_test_records VALUES (?, 2)")
            .bind(restarted_id.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        assert!(receipt::begin(&pool).await.is_err());
        Ok(())
    })
    .await;
    restarted.unwrap();
    assert!(receipt::applied(&pool, restarted_id).await.unwrap());
}
