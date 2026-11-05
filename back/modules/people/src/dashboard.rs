use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use aphel_kernel::errors::AppError;
use aphel_sdk::dashboard::{DashboardPorts, DashboardSource, WidgetContext, WidgetPorts};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use async_trait::async_trait;
use serde_json::{Map, Value, json};
pub(crate) struct PeopleSummarySource;
#[async_trait]
impl DashboardSource for PeopleSummarySource {
    fn key(&self) -> &str {
        "people.summary"
    }
    fn collection(&self) -> &str {
        "summary"
    }
    fn available(&self, scope: &AccessScope) -> bool {
        scope.filter_for(&Permission::CONTACTS_READ).is_some()
    }
    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        _settings: &Value,
    ) -> Result<Value, AppError> {
        let mut totals = Map::new();
        let previous = Map::new();
        if let Some(planner) = context.planner(&RecordType::CONTACT).await? {
            let plan = planner.plan(Vec::new(), Vec::new(), 1)?;
            totals.insert(
                "contacts_total".into(),
                json!(
                    context
                        .ports
                        .dashboard()
                        .count(context.scope, &plan)
                        .await?
                ),
            );
        }

        totals.insert(
            "previous".into(),
            if context.compare_to_previous {
                Value::Object(previous)
            } else {
                Value::Null
            },
        );
        Ok(Value::Object(totals))
    }
}
