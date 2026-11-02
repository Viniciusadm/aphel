use async_trait::async_trait;
use serde_json::{Value, json};

use crate::registry;
use crate::runtime::on_data_subject;
use aphel_kernel::application::data_subject::{
    DataSubjectOutcome, DataSubjectRequest, DataSubjectSource,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::runtime_pool;

pub const PRIVACY_REQUEST_TYPE: &str = "privacy_request";

pub struct PluginDataSubjects;

fn input(request: &DataSubjectRequest<'_>) -> Value {
    let records: Vec<Value> = request
        .records
        .iter()
        .map(|record| {
            json!({
                "record_type": record.record_type.as_str(),
                "id": record.record_id.to_string(),
            })
        })
        .collect();
    json!({
        "request_id": request.id.to_string(),
        "action": request.action.as_str(),
        "records": records,
    })
}

async fn subject_plugins() -> Result<Vec<std::sync::Arc<registry::LoadedPlugin>>, AppError> {
    let snapshot = registry::current(&runtime_pool()?).await?;
    Ok(snapshot
        .plugins
        .iter()
        .filter(|plugin| plugin.manifest.privacy.subject_data)
        .cloned()
        .collect())
}

#[async_trait]
impl DataSubjectSource for PluginDataSubjects {
    async fn run(&self, request: &DataSubjectRequest<'_>) -> Vec<DataSubjectOutcome> {
        let plugins = match subject_plugins().await {
            Ok(plugins) => plugins,
            Err(error) => {
                return vec![DataSubjectOutcome {
                    source: "plugins".to_owned(),
                    result: Err(error),
                }];
            }
        };
        let input = input(request);
        let mut outcomes = Vec::with_capacity(plugins.len());
        for plugin in plugins {
            outcomes.push(DataSubjectOutcome {
                source: plugin.key.clone(),
                result: on_data_subject(&plugin, request, &input).await,
            });
        }
        outcomes
    }
}
