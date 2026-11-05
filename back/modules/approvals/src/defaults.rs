use aphel_kernel::errors::AppError;
use sqlx::MySqlConnection;
use uuid::Uuid;
pub(crate) async fn seed_templates(connection: &mut MySqlConnection) -> Result<(), AppError> {
    let templates = aphel_sdk::platform::modules::app()
        .contributions::<aphel_sdk::approvals::ApprovalPolicyTemplate>();
    let mut keys = std::collections::BTreeSet::new();
    for template in &templates {
        if !keys.insert(template.key.clone()) {
            return Err(AppError::Configuration(format!(
                "duplicate default approval policy {}",
                template.key
            )));
        }
    }
    for template in templates {
        if template.steps.is_empty() || template.steps.len() > crate::domain::MAX_POLICY_STEPS {
            return Err(AppError::Configuration(
                "invalid default approval policy steps".into(),
            ));
        }
        sqlx::query("INSERT IGNORE INTO approval_policies (id,`key`,name,description,record_type,moment,target_id,condition_kind,condition_filter,condition_formula,locked_fields,due_hours,is_active,position) VALUES (?,?,?,?,?,?,NULL,'filter',?,NULL,?,NULL,TRUE,0)")
            .bind(Uuid::now_v7().to_string()).bind(&template.key).bind(&template.name).bind(&template.description).bind(&template.record_type).bind(template.moment.as_str()).bind(sqlx::types::Json(&template.filter)).bind(sqlx::types::Json(&template.locked_fields)).execute(&mut *connection).await?;
        let id: String = sqlx::query_scalar("SELECT id FROM approval_policies WHERE `key` = ?")
            .bind(&template.key)
            .fetch_one(&mut *connection)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM approval_policy_steps WHERE policy_id = ?")
                .bind(&id)
                .fetch_one(&mut *connection)
                .await?;
        if count > 0 {
            continue;
        }
        for (position, step) in template.steps.iter().enumerate() {
            if crate::domain::ApproverKind::parse(&step.approver_kind).is_none()
                || crate::domain::ApprovalRule::parse(&step.rule).is_none()
            {
                return Err(AppError::Configuration(
                    "invalid default approval step".into(),
                ));
            }
            sqlx::query("INSERT INTO approval_policy_steps (id,policy_id,position,name,approver_kind,approver_user_id,approver_role_id,rule) VALUES (?,?,?,?,?,?,?,?)")
                .bind(Uuid::now_v7().to_string()).bind(&id).bind(i32::try_from(position).expect("bounded policy steps")).bind(&step.name).bind(&step.approver_kind).bind(step.approver_user_id.map(|id|id.to_string())).bind(step.approver_role_id.map(|id|id.to_string())).bind(&step.rule).execute(&mut *connection).await?;
        }
    }
    Ok(())
}
