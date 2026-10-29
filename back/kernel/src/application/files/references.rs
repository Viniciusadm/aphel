use std::sync::Arc;

use crate::application::fields::is_sql_identifier;
use crate::modules::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileReferenceColumn {
    pub table: &'static str,
    pub column: &'static str,
}

impl FileReferenceColumn {
    pub const fn new(table: &'static str, column: &'static str) -> Self {
        Self { table, column }
    }
}

pub trait FileReferenceSource: Send + Sync {
    fn file_columns(&self) -> Vec<FileReferenceColumn>;
}

pub const PLATFORM_FILE_COLUMNS: [FileReferenceColumn; 4] = [
    FileReferenceColumn::new("attachments", "file_id"),
    FileReferenceColumn::new("organization_settings", "logo_file_id"),
    FileReferenceColumn::new("users", "avatar_file_id"),
    FileReferenceColumn::new("installed_modules", "data_export_file_id"),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformFileReferences;

impl FileReferenceSource for PlatformFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        PLATFORM_FILE_COLUMNS.to_vec()
    }
}

pub fn declared_file_columns(app: &App) -> Vec<FileReferenceColumn> {
    let mut columns: Vec<FileReferenceColumn> = Vec::new();
    let sources = std::iter::once(Arc::new(PlatformFileReferences) as Arc<dyn FileReferenceSource>)
        .chain(
            app.all_contributions::<dyn FileReferenceSource>()
                .into_iter()
                .map(|(_, source)| source),
        );
    for source in sources {
        for column in source.file_columns() {
            if is_sql_identifier(column.table)
                && is_sql_identifier(column.column)
                && !columns.contains(&column)
            {
                columns.push(column);
            }
        }
    }
    columns
}

pub fn unreferenced_file_condition(columns: &[FileReferenceColumn], file_alias: &str) -> String {
    if columns.is_empty() {
        return "TRUE".to_owned();
    }
    columns
        .iter()
        .map(|reference| {
            format!(
                "NOT EXISTS (SELECT 1 FROM `{table}` rf WHERE rf.`{column}` = {file_alias}.id)",
                table = reference.table,
                column = reference.column,
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}
