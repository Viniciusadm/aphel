use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    CurrencyContext, EnabledCurrency, ExchangeRate, ExchangeRateFilter, RateSource,
    RatesProviderSettings,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper;

const RATE_COLUMNS: &str = "id, currency, effective_date, rate, source, provider, created_by, \
     created_at, updated_at";

fn rate_from_row(row: &MySqlRow) -> Result<ExchangeRate, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let source: String = row.try_get("source")?;
    let created_by: Option<String> = row.try_get("created_by")?;
    Ok(ExchangeRate {
        id: uuid_column(row, "id")?,
        currency: row.try_get("currency")?,
        effective_date: row.try_get("effective_date")?,
        rate: row.try_get("rate")?,
        source: RateSource::parse(&source)
            .ok_or_else(|| mapper::invalid_column("exchange_rates.source", &source))?,
        provider: row.try_get("provider")?,
        created_by: created_by
            .map(|value| UserId::new(value).map_err(AppError::from))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub struct CurrencyStore {
    pool: MySqlPool,
}

impl CurrencyStore {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn begin(&self) -> Result<MySqlTx, AppError> {
        aphel_kernel::database::receipt::begin(&self.pool).await
    }

    pub async fn find_context(&self) -> Result<Option<CurrencyContext>, AppError> {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT currency, timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(base, timezone)| CurrencyContext { base, timezone }))
    }

    pub(crate) async fn context(&self) -> Result<CurrencyContext, AppError> {
        self.find_context()
            .await?
            .ok_or_else(|| AppError::NotFound("organization was not found".to_owned()))
    }

    pub(crate) async fn enabled(&self) -> Result<Vec<EnabledCurrency>, AppError> {
        let rows: Vec<(String, NaiveDateTime, Option<String>)> = sqlx::query_as(
            "SELECT currency, enabled_at, enabled_by FROM money_currencies \
             ORDER BY currency",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|(code, enabled_at, enabled_by)| {
                Ok(EnabledCurrency {
                    code,
                    enabled_at: mapper::utc(enabled_at),
                    enabled_by: enabled_by
                        .map(|value| UserId::new(value).map_err(AppError::from))
                        .transpose()?,
                })
            })
            .collect()
    }

    pub(crate) async fn enabled_codes(&self) -> Result<Vec<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT c.currency FROM money_currencies c \
             JOIN organization_settings o ON o.id = 1 \
             WHERE c.currency <> o.currency ORDER BY c.currency",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn is_enabled(&self, currency: &str) -> Result<bool, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM money_currencies WHERE currency = ?) \
             OR EXISTS(SELECT 1 FROM organization_settings WHERE id = 1 AND currency = ?)",
        )
        .bind(currency)
        .bind(currency)
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn enable(
        &self,
        tx: &mut MySqlTx,
        currency: &str,
        by: Option<&UserId>,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT IGNORE INTO money_currencies (currency, enabled_by, enabled_at) \
             VALUES (?, ?, ?)",
        )
        .bind(currency)
        .bind(by.map(UserId::as_str))
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn disable(&self, tx: &mut MySqlTx, currency: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM money_currencies WHERE currency = ?")
            .bind(currency)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn rates(
        &self,
        filter: &ExchangeRateFilter,
        window: ListWindow,
    ) -> Result<Listing<ExchangeRate>, AppError> {
        let push_conditions = |builder: &mut QueryBuilder<sqlx::MySql>| {
            if let Some(currency) = &filter.currency {
                builder
                    .push(" WHERE currency = ")
                    .push_bind(currency.clone());
            }
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM money_exchange_rates");
        push_conditions(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut select =
            QueryBuilder::new(format!("SELECT {RATE_COLUMNS} FROM money_exchange_rates"));
        push_conditions(&mut select);
        select
            .push(" ORDER BY effective_date DESC, currency, id LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = select.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(rate_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn find_rate(&self, id: Uuid) -> Result<Option<ExchangeRate>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RATE_COLUMNS} FROM money_exchange_rates WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(rate_from_row)
        .transpose()
    }

    pub async fn rate_on(
        &self,
        currency: &str,
        date: NaiveDate,
    ) -> Result<Option<ExchangeRate>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RATE_COLUMNS} FROM money_exchange_rates \
             WHERE currency = ? AND effective_date <= ? \
             ORDER BY effective_date DESC LIMIT 1"
        )))
        .bind(currency)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(rate_from_row)
        .transpose()
    }

    pub(crate) async fn rate_dated(
        &self,
        currency: &str,
        date: NaiveDate,
    ) -> Result<Option<ExchangeRate>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RATE_COLUMNS} FROM money_exchange_rates \
             WHERE currency = ? AND effective_date = ?"
        )))
        .bind(currency)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(rate_from_row)
        .transpose()
    }

    pub(crate) async fn insert_rate(
        &self,
        tx: &mut MySqlTx,
        rate: &ExchangeRate,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO money_exchange_rates (id, currency, effective_date, rate, source, \
             provider, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rate.id.to_string())
        .bind(&rate.currency)
        .bind(rate.effective_date)
        .bind(rate.rate)
        .bind(rate.source.as_str())
        .bind(rate.provider.as_deref())
        .bind(rate.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(rate.created_at))
        .bind(mapper::naive(rate.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_rate(
        &self,
        tx: &mut MySqlTx,
        rate: &ExchangeRate,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE money_exchange_rates SET effective_date = ?, rate = ?, source = ?, provider = ?, \
             updated_at = ? WHERE id = ?",
        )
        .bind(rate.effective_date)
        .bind(rate.rate)
        .bind(rate.source.as_str())
        .bind(rate.provider.as_deref())
        .bind(mapper::naive(rate.updated_at))
        .bind(rate.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_rate(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM money_exchange_rates WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn provider_settings(&self) -> Result<RatesProviderSettings, AppError> {
        let row: Option<(String, Option<String>, Option<NaiveDateTime>)> = sqlx::query_as(
            "SELECT rates_provider, rates_provider_url, last_synced_at FROM money_settings \
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row
            .map(|(provider, url, last_synced_at)| RatesProviderSettings {
                provider,
                url,
                last_synced_at: last_synced_at.map(mapper::utc),
            })
            .unwrap_or_default())
    }

    pub(crate) async fn save_provider_settings(
        &self,
        tx: &mut MySqlTx,
        settings: &RatesProviderSettings,
        by: Option<&UserId>,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO money_settings (id, rates_provider, rates_provider_url, updated_by, \
             updated_at) VALUES (1, ?, ?, ?, ?) AS incoming ON DUPLICATE KEY UPDATE \
             rates_provider = incoming.rates_provider, \
             rates_provider_url = incoming.rates_provider_url, \
             updated_by = incoming.updated_by, updated_at = incoming.updated_at",
        )
        .bind(&settings.provider)
        .bind(settings.url.as_deref())
        .bind(by.map(UserId::as_str))
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_synced(&self, at: DateTime<Utc>) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO money_settings (id, last_synced_at) VALUES (1, ?) AS incoming \
             ON DUPLICATE KEY UPDATE last_synced_at = incoming.last_synced_at",
        )
        .bind(mapper::naive(at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn lock_rates_base(
        &self,
        tx: &mut MySqlTx,
    ) -> Result<Option<String>, AppError> {
        sqlx::query("INSERT IGNORE INTO money_settings (id) VALUES (1)")
            .execute(&mut **tx)
            .await?;
        Ok(
            sqlx::query_scalar("SELECT rates_base FROM money_settings WHERE id = 1 FOR UPDATE")
                .fetch_one(&mut **tx)
                .await?,
        )
    }

    pub(crate) async fn set_rates_base(
        &self,
        tx: &mut MySqlTx,
        base: &str,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE money_settings SET rates_base = ? WHERE id = 1")
            .bind(base)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn base_in(&self, tx: &mut MySqlTx) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&mut **tx)
                .await?,
        )
    }

    pub(crate) async fn all_rates_in(
        &self,
        tx: &mut MySqlTx,
    ) -> Result<Vec<ExchangeRate>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RATE_COLUMNS} FROM money_exchange_rates ORDER BY currency, effective_date FOR UPDATE"
        )))
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(rate_from_row)
        .collect()
    }

    pub(crate) async fn rate_dated_in(
        &self,
        tx: &mut MySqlTx,
        currency: &str,
        date: NaiveDate,
    ) -> Result<Option<ExchangeRate>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RATE_COLUMNS} FROM money_exchange_rates \
             WHERE currency = ? AND effective_date = ? FOR UPDATE"
        )))
        .bind(currency)
        .bind(date)
        .fetch_optional(&mut **tx)
        .await?
        .as_ref()
        .map(rate_from_row)
        .transpose()
    }
}
