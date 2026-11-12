use std::io::Cursor;

use aphel_contract_site::{IMAGE_WIDTHS, SiteExposure};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::RecordCtx;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::files::{ListAttachmentsQuery, StoredFile};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::domain::access::AccessScope;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::runtime::SiteRuntime;

pub(crate) const FILES_SEGMENT: &str = "files";
const ORIGINAL_VARIANT: &str = "original";
const MAX_IMAGES: u64 = 100;
const MAX_SOURCE_BYTES: usize = 20 * 1024 * 1024;
const MAX_SOURCE_PIXELS: u32 = 8_000;
const JPEG_QUALITY: u8 = 82;
const IMAGE_CACHE_CONTROL: &str = "public, max-age=86400";
const PUBLIC_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp", "image/gif"];

fn site_scope() -> AccessScope {
    AccessScope::system().acting_as("site", "public")
}

fn image_url(record_type: &str, record_id: &str, file_id: &Uuid) -> String {
    format!("/_aphel/site/{FILES_SEGMENT}/{record_type}/{record_id}/{file_id}/{ORIGINAL_VARIANT}")
}

async fn public_images(
    state: &AppState,
    record_type: &str,
    record_id: &str,
) -> Result<Vec<StoredFile>, AppError> {
    let scope = site_scope();
    let listing = Ctx::from_state(state, &scope)
        .await?
        .files()
        .await?
        .list_attachments(
            &scope,
            ListAttachmentsQuery {
                record_type: record_type.to_owned(),
                record_id: record_id.to_owned(),
            },
            ListWindow {
                offset: 0,
                limit: MAX_IMAGES,
            },
        )
        .await?;
    Ok(listing
        .items
        .into_iter()
        .map(|attachment| attachment.file)
        .filter(|file| PUBLIC_IMAGE_TYPES.contains(&file.mime_type.as_str()))
        .collect())
}

pub(crate) async fn attach_images(
    state: &AppState,
    exposure: &SiteExposure,
    record: &mut Value,
) -> Result<(), AppError> {
    if !exposure.images {
        return Ok(());
    }
    let Some(record_id) = record.get("id").and_then(Value::as_str).map(str::to_owned) else {
        return Ok(());
    };
    let images: Vec<Value> = public_images(state, &exposure.entity, &record_id)
        .await?
        .iter()
        .map(|file| {
            json!({
                "id": file.id,
                "name": file.original_name,
                "type": file.mime_type,
                "url": image_url(&exposure.entity, &record_id, &file.id),
            })
        })
        .collect();
    let registry = state.registry().await?;
    let file_fields: Vec<String> = registry
        .require(&exposure.entity)
        .map(|entity| {
            exposure
                .fields
                .iter()
                .filter(|key| {
                    entity
                        .system_field_by_key(key)
                        .is_some_and(|field| field.field_type.is_file())
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(fields) = record.as_object_mut() {
        for key in file_fields {
            let picture = fields
                .get(&key)
                .and_then(Value::as_str)
                .and_then(|id| images.iter().find(|image| image["id"].as_str() == Some(id)))
                .cloned()
                .unwrap_or(Value::Null);
            fields.insert(format!("{key}_image"), picture);
        }
        fields.insert("images".to_owned(), Value::Array(images));
    }
    Ok(())
}

fn condition_holds(record: &Value, field: &str, expected: &Value) -> bool {
    let actual = record.get(field).or_else(|| {
        record
            .get("custom_fields")
            .and_then(|custom| custom.get(field))
    });
    match (actual, expected) {
        (Some(actual), expected) if actual == expected => true,
        (Some(Value::String(actual)), expected) => {
            expected
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| expected.to_string())
                == *actual
        }
        (Some(actual), Value::String(expected)) => {
            serde_json::from_str::<Value>(expected).is_ok_and(|parsed| parsed == *actual)
        }
        _ => false,
    }
}

async fn published(
    state: &AppState,
    exposure: &SiteExposure,
    record_id: Uuid,
) -> Result<bool, AppError> {
    let scope = site_scope();
    let Some(record) = state
        .registry()
        .await?
        .handler(&exposure.entity)?
        .load(&RecordCtx::new(&scope, state), record_id)
        .await?
    else {
        return Ok(false);
    };
    let archived = record
        .get("archived_at")
        .is_some_and(|value| !value.is_null());
    Ok(!archived
        && exposure
            .conditions
            .iter()
            .all(|(field, expected)| condition_holds(&record, field, expected)))
}

enum Variant {
    Original,
    Width(u32),
}

fn variant_of(value: &str) -> Option<Variant> {
    if value == ORIGINAL_VARIANT {
        return Some(Variant::Original);
    }
    value
        .strip_prefix('w')
        .and_then(|width| width.parse::<u32>().ok())
        .filter(|width| IMAGE_WIDTHS.contains(width))
        .map(Variant::Width)
}

fn not_found() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

pub(crate) async fn serve(
    state: &AppState,
    runtime: &SiteRuntime,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response, AppError> {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let [FILES_SEGMENT, record_type, record_id, file_id, variant] = parts.as_slice() else {
        return Ok(not_found());
    };
    let (Ok(record_uuid), Ok(file_uuid), Some(variant)) = (
        Uuid::parse_str(record_id),
        Uuid::parse_str(file_id),
        variant_of(variant),
    ) else {
        return Ok(not_found());
    };
    let Some(exposure) = runtime.image_exposure(record_type) else {
        return Ok(not_found());
    };
    let private = match published(state, exposure, record_uuid).await? {
        true => false,
        false
            if crate::preview::active(state, headers)
                .await
                .is_some_and(|preview| preview.shows_record(record_type, record_id)) =>
        {
            true
        }
        false => return Ok(not_found()),
    };
    let response =
        file_response(state, record_type, record_id, file_uuid, variant, headers).await?;
    Ok(match private {
        true => without_caching(response),
        false => response,
    })
}

fn without_caching(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}

async fn file_response(
    state: &AppState,
    record_type: &str,
    record_id: &str,
    file_uuid: Uuid,
    variant: Variant,
    headers: &HeaderMap,
) -> Result<Response, AppError> {
    let Some(file) = public_images(state, record_type, record_id)
        .await?
        .into_iter()
        .find(|file| file.id == file_uuid)
    else {
        return Ok(not_found());
    };
    let variant_key = match variant {
        Variant::Original => ORIGINAL_VARIANT.to_owned(),
        Variant::Width(width) => format!("w{width}"),
    };
    let etag = format!(
        "\"{}-{variant_key}\"",
        &file.checksum[..file.checksum.len().min(24)]
    );
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|candidate| candidate.trim() == etag))
    {
        return Ok(with_headers(
            StatusCode::NOT_MODIFIED.into_response(),
            None,
            &etag,
        ));
    }
    match variant {
        Variant::Original => {
            let scope = site_scope();
            let download = Ctx::from_state(state, &scope)
                .await?
                .files()
                .await?
                .download(&scope, &file.id.to_string())
                .await?;
            Ok(with_headers(
                (StatusCode::OK, Body::from_stream(download.content)).into_response(),
                Some(&file.mime_type),
                &etag,
            ))
        }
        Variant::Width(width) => {
            let (content_type, bytes) = resized(state, &file, width).await?;
            Ok(with_headers(
                (StatusCode::OK, bytes).into_response(),
                Some(content_type),
                &etag,
            ))
        }
    }
}

fn with_headers(mut response: Response, content_type: Option<&str>, etag: &str) -> Response {
    let headers = response.headers_mut();
    if let Some(value) = content_type.and_then(|value| HeaderValue::from_str(value).ok()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(header::ETAG, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(IMAGE_CACHE_CONTROL),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn variant_storage_key(file: &StoredFile, width: u32) -> String {
    format!("site/variants/{}/w{width}", file.id)
}

fn output_type(source: &str) -> (&'static str, image::ImageFormat) {
    match source {
        "image/jpeg" => ("image/jpeg", image::ImageFormat::Jpeg),
        "image/webp" => ("image/webp", image::ImageFormat::WebP),
        _ => ("image/png", image::ImageFormat::Png),
    }
}

pub(crate) async fn collect(
    mut stream: aphel_sdk::platform::application::shared::ByteStream,
) -> Result<Vec<u8>, AppError> {
    let mut buffer = Vec::new();
    while let Some(chunk) = stream.next().await {
        buffer.extend_from_slice(&chunk?);
        if buffer.len() > MAX_SOURCE_BYTES {
            return Err(AppError::validation_code(
                "site_image_too_large",
                "the image is too large to resize",
            ));
        }
    }
    Ok(buffer)
}

async fn resized(
    state: &AppState,
    file: &StoredFile,
    width: u32,
) -> Result<(&'static str, Bytes), AppError> {
    let (content_type, format) = output_type(&file.mime_type);
    let key = variant_storage_key(file, width);
    if let Ok(stream) = state.storage.get_stream(&key).await
        && let Ok(bytes) = collect(stream).await
        && !bytes.is_empty()
    {
        return Ok((content_type, Bytes::from(bytes)));
    }
    let source = collect(state.storage.get_stream(&file.storage_key).await?).await?;
    let encoded = tokio::task::spawn_blocking(move || resize(&source, width, format))
        .await
        .map_err(|error| AppError::Internal(format!("image resize task failed: {error}")))??;
    let bytes = Bytes::from(encoded);
    state.storage.put(&key, bytes.clone()).await?;
    Ok((content_type, bytes))
}

fn resize(source: &[u8], width: u32, format: image::ImageFormat) -> Result<Vec<u8>, AppError> {
    let invalid = |error: image::ImageError| {
        AppError::validation_code(
            "site_image_invalid",
            format!("the image cannot be read: {error}"),
        )
    };
    let mut reader = image::ImageReader::new(Cursor::new(source))
        .with_guessed_format()
        .map_err(|error| AppError::Internal(format!("could not read the image: {error}")))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_PIXELS);
    limits.max_image_height = Some(MAX_SOURCE_PIXELS);
    reader.limits(limits);
    let decoded = reader.decode().map_err(invalid)?;
    let scaled = if decoded.width() > width {
        decoded.resize(width, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        decoded
    };
    let mut output = Cursor::new(Vec::new());
    match format {
        image::ImageFormat::Jpeg => {
            let encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, JPEG_QUALITY);
            scaled
                .to_rgb8()
                .write_with_encoder(encoder)
                .map_err(invalid)?;
        }
        other => scaled.write_to(&mut output, other).map_err(invalid)?,
    }
    Ok(output.into_inner())
}
