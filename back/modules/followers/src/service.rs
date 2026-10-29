use std::collections::BTreeMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::KEY;
use crate::domain::{FollowOrigin, FollowedRecord, MAX_FOLLOWED_RECORDS, RecordFollowers};
use crate::store::FollowerStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::notifications::RecordFollowerSource;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::record_sharing::{RecordDirectory, record_not_found};
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::record_sharing::MySqlRecordDirectory;

fn directory(ctx: &Ctx<'_>) -> MySqlRecordDirectory {
    MySqlRecordDirectory::new(ctx.pool().clone())
}

pub(crate) async fn followers(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<RecordFollowers, AppError> {
    let record = readable(ctx, record_type, record_id).await?;
    summary(ctx, record).await
}

pub(crate) async fn follow(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<RecordFollowers, AppError> {
    let record = readable(ctx, record_type, record_id).await?;
    let mut tx = ctx.begin().await?;
    FollowerStore::of(ctx)
        .follow(
            &mut tx,
            &record,
            ctx.scope.user_id(),
            FollowOrigin::Manual,
            ctx.now(),
        )
        .await?;
    tx.commit().await?;
    summary(ctx, record).await
}

pub(crate) async fn unfollow(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<RecordFollowers, AppError> {
    let definition = ctx.registry().require(record_type)?;
    let id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_found())?;
    let record = RecordRef::new(definition.record_type.clone(), id);
    let mut tx = ctx.begin().await?;
    FollowerStore::of(ctx)
        .unfollow(&mut tx, &record, ctx.scope.user_id())
        .await?;
    tx.commit().await?;
    match readable(ctx, record_type, record_id).await {
        Ok(record) => summary(ctx, record).await,
        Err(_) => Ok(RecordFollowers {
            record,
            following: false,
            followers: Vec::new(),
        }),
    }
}

pub(crate) async fn followed(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
    window: ListWindow,
) -> Result<Listing<FollowedRecord>, AppError> {
    let record_type = record_type.map(str::trim).filter(|value| !value.is_empty());
    if let Some(record_type) = record_type {
        ctx.registry().require(record_type)?;
    }
    let follows = FollowerStore::of(ctx)
        .followed(ctx.scope.user_id(), record_type, MAX_FOLLOWED_RECORDS)
        .await?;
    let mut by_type: BTreeMap<RecordType, Vec<Uuid>> = BTreeMap::new();
    for follow in &follows {
        by_type
            .entry(follow.record.record_type.clone())
            .or_default()
            .push(follow.record.record_id);
    }
    let directory = directory(ctx);
    let mut titles = BTreeMap::new();
    for (record_type, ids) in by_type {
        let Some(definition) = ctx.registry().get(&record_type) else {
            continue;
        };
        let Some(visibility) = ctx.scope.filter_for(&definition.read_permission) else {
            continue;
        };
        let visible = directory
            .visible_titles(ctx.scope, definition, &visibility, &ids)
            .await?;
        for (id, title) in visible {
            titles.insert((record_type.clone(), id), title);
        }
    }
    let visible: Vec<FollowedRecord> = follows
        .into_iter()
        .filter_map(|follow| {
            let title = titles
                .get(&(follow.record.record_type.clone(), follow.record.record_id))?
                .clone();
            Some(FollowedRecord { follow, title })
        })
        .collect();
    let total = visible.len() as u64;
    let items = visible
        .into_iter()
        .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
        .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
        .collect();
    Ok(Listing { items, total })
}

async fn summary(ctx: &Ctx<'_>, record: RecordRef) -> Result<RecordFollowers, AppError> {
    let followers = FollowerStore::of(ctx).followers(&record).await?;
    let following = followers
        .iter()
        .any(|follower| &follower.user.id == ctx.scope.user_id());
    Ok(RecordFollowers {
        record,
        following,
        followers,
    })
}

async fn readable(
    ctx: &Ctx<'_>,
    record_type: &str,
    record_id: &str,
) -> Result<RecordRef, AppError> {
    let definition = ctx.registry().require(record_type)?;
    let visibility = ctx.scope.record_filter(&definition.read_permission)?;
    let id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_found())?;
    directory(ctx)
        .header(ctx.scope, definition, &visibility, id)
        .await?
        .ok_or_else(record_not_found)?;
    Ok(RecordRef::new(definition.record_type.clone(), id))
}

pub(crate) struct FollowerSource;

#[async_trait]
impl RecordFollowerSource for FollowerSource {
    async fn followers(
        &self,
        pool: &sqlx::MySqlPool,
        record: &RecordRef,
    ) -> Result<Vec<UserId>, AppError> {
        if !aphel_kernel::modules::app().is_enabled(KEY) {
            return Ok(Vec::new());
        }
        FollowerStore::new(pool.clone()).follower_ids(record).await
    }
}
