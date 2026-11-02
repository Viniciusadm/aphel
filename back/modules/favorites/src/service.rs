use std::collections::{BTreeSet, HashMap};

use uuid::Uuid;

use crate::domain::{
    CreateFavoriteCommand, FAVORITE_ALREADY_EXISTS, FAVORITE_LIMIT_REACHED, FAVORITE_NOT_FOUND,
    FAVORITE_TARGET_NOT_FOUND, Favorite, FavoriteEntry, FavoriteKind, MAX_FAVORITES,
    ReadableRecordType,
};
use crate::store::FavoriteStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::files::UNSUPPORTED_RECORD_TYPE;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::{AppError, FieldViolation};

fn parse_id(value: Option<&str>, field: &str) -> Result<Uuid, AppError> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    let Some(value) = value else {
        return Err(AppError::invalid_fields(
            "invalid favorite",
            vec![FieldViolation::new(field, "is required")],
        ));
    };
    Uuid::parse_str(value).map_err(|_| {
        AppError::invalid_fields(
            "invalid favorite",
            vec![FieldViolation::new(field, "must be a valid id")],
        )
    })
}

fn target_not_found() -> AppError {
    AppError::not_found_code(
        FAVORITE_TARGET_NOT_FOUND,
        "the record or view does not exist or is not visible",
    )
}

fn readable(ctx: &Ctx<'_>, record_type: &RecordType) -> Option<ReadableRecordType> {
    let definition = ctx.registry().get(record_type)?;
    let visibility = ctx.scope.filter_for(&definition.read_permission)?;
    Some(ReadableRecordType {
        definition: definition.clone(),
        title_column: definition.title_column()?.to_owned(),
        visibility,
    })
}

fn readable_view(ctx: &Ctx<'_>, record_type: &RecordType) -> bool {
    ctx.registry()
        .get(record_type)
        .is_some_and(|definition| ctx.scope.filter_for(&definition.read_permission).is_some())
}

async fn resolve(ctx: &Ctx<'_>, favorites: Vec<Favorite>) -> Result<Vec<FavoriteEntry>, AppError> {
    let store = FavoriteStore::of(ctx);
    let mut record_ids: HashMap<RecordType, Vec<Uuid>> = HashMap::new();
    let mut view_ids = Vec::new();
    for favorite in &favorites {
        match favorite.kind {
            FavoriteKind::Record => record_ids
                .entry(favorite.record_type.clone())
                .or_default()
                .push(favorite.target_id),
            FavoriteKind::View => view_ids.push(favorite.target_id),
        }
    }
    let mut record_titles: HashMap<(RecordType, Uuid), String> = HashMap::new();
    for (record_type, ids) in record_ids {
        let Some(readable) = readable(ctx, &record_type) else {
            continue;
        };
        let titles = store.record_titles(ctx.scope, &readable, &ids).await?;
        for (id, title) in titles {
            record_titles.insert((record_type.clone(), id), title);
        }
    }
    let views = if view_ids.is_empty() {
        HashMap::new()
    } else {
        store.visible_views(ctx.scope, &view_ids).await?
    };
    Ok(favorites
        .into_iter()
        .map(|favorite| {
            let title = match favorite.kind {
                FavoriteKind::Record => {
                    record_titles.remove(&(favorite.record_type.clone(), favorite.target_id))
                }
                FavoriteKind::View => views
                    .get(&favorite.target_id)
                    .filter(|view| readable_view(ctx, &view.record_type))
                    .map(|view| view.name.clone()),
            };
            FavoriteEntry {
                available: title.is_some(),
                title,
                favorite,
            }
        })
        .collect())
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<FavoriteEntry>, AppError> {
    let favorites = FavoriteStore::of(ctx).list(ctx.scope).await?;
    resolve(ctx, favorites).await
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateFavoriteCommand,
) -> Result<FavoriteEntry, AppError> {
    let store = FavoriteStore::of(ctx);
    let kind = command.favorite_type;
    let (record_type, target_id, title) = match kind {
        FavoriteKind::Record => record_target(ctx, &command).await?,
        FavoriteKind::View => view_target(ctx, &command).await?,
    };
    if store
        .find_target(ctx.scope, kind, target_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict_code(
            FAVORITE_ALREADY_EXISTS,
            "this item is already a favorite",
        ));
    }
    let existing = store.list(ctx.scope).await?;
    if existing.len() >= MAX_FAVORITES {
        return Err(AppError::conflict_code(
            FAVORITE_LIMIT_REACHED,
            format!("at most {MAX_FAVORITES} favorites are allowed"),
        ));
    }
    let position = existing
        .iter()
        .map(|favorite| favorite.position)
        .max()
        .map_or(0, |position| position + 1);
    let favorite = Favorite {
        id: Uuid::now_v7(),
        kind,
        record_type,
        target_id,
        position,
        created_at: ctx.now(),
    };
    let mut tx = ctx.begin().await?;
    store.insert(&mut tx, ctx.scope, &favorite).await?;
    tx.commit().await?;
    Ok(FavoriteEntry {
        favorite,
        title: Some(title),
        available: true,
    })
}

async fn record_target(
    ctx: &Ctx<'_>,
    command: &CreateFavoriteCommand,
) -> Result<(RecordType, Uuid, String), AppError> {
    let raw_type = command
        .record_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::invalid_fields(
                "invalid favorite",
                vec![FieldViolation::new("record_type", "is required")],
            )
        })?;
    let unsupported = || {
        AppError::validation_code(
            UNSUPPORTED_RECORD_TYPE,
            format!("records of type {raw_type} cannot be favorites"),
        )
    };
    let record_type = RecordType::parse(raw_type).map_err(|_| unsupported())?;
    let record_id = parse_id(command.record_id.as_deref(), "record_id")?;
    let definition = ctx
        .registry()
        .get(&record_type)
        .filter(|definition| definition.title_column().is_some())
        .ok_or_else(unsupported)?;
    ctx.scope.record_filter(&definition.read_permission)?;
    let readable = readable(ctx, &record_type).ok_or_else(unsupported)?;
    let title = FavoriteStore::of(ctx)
        .record_titles(ctx.scope, &readable, &[record_id])
        .await?
        .remove(&record_id)
        .ok_or_else(target_not_found)?;
    Ok((record_type, record_id, title))
}

async fn view_target(
    ctx: &Ctx<'_>,
    command: &CreateFavoriteCommand,
) -> Result<(RecordType, Uuid, String), AppError> {
    let view_id = parse_id(command.view_id.as_deref(), "view_id")?;
    let view = FavoriteStore::of(ctx)
        .visible_views(ctx.scope, &[view_id])
        .await?
        .remove(&view_id)
        .ok_or_else(target_not_found)?;
    let definition = ctx
        .registry()
        .get(&view.record_type)
        .ok_or_else(target_not_found)?;
    ctx.scope.record_filter(&definition.read_permission)?;
    Ok((view.record_type, view_id, view.name))
}

pub(crate) async fn delete(ctx: &Ctx<'_>, favorite_id: &str) -> Result<(), AppError> {
    let not_found = || AppError::not_found_code(FAVORITE_NOT_FOUND, "favorite not found");
    let id = Uuid::parse_str(favorite_id.trim()).map_err(|_| not_found())?;
    let mut tx = ctx.begin().await?;
    if !FavoriteStore::of(ctx)
        .delete(&mut tx, ctx.scope, id)
        .await?
    {
        return Err(not_found());
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn reorder(ctx: &Ctx<'_>, ids: &[String]) -> Result<Vec<FavoriteEntry>, AppError> {
    let store = FavoriteStore::of(ctx);
    let invalid = || {
        AppError::invalid_fields(
            "invalid order",
            vec![FieldViolation::new(
                "ids",
                "must list every favorite of the user exactly once",
            )],
        )
    };
    let ordered = ids
        .iter()
        .map(|id| Uuid::parse_str(id.trim()).map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    let existing = store.list(ctx.scope).await?;
    let requested: BTreeSet<Uuid> = ordered.iter().copied().collect();
    let known: BTreeSet<Uuid> = existing.iter().map(|favorite| favorite.id).collect();
    if requested.len() != ordered.len() || requested != known {
        return Err(invalid());
    }
    let current: Vec<Uuid> = existing.iter().map(|favorite| favorite.id).collect();
    if current != ordered {
        let mut tx = ctx.begin().await?;
        store.reorder(&mut tx, ctx.scope, &ordered).await?;
        tx.commit().await?;
    }
    list(ctx).await
}
