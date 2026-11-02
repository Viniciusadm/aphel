use crate::application::fields::EntityDefinition;
use crate::application::query::FilterOperator;
use crate::domain::fields::{FieldDefinition, FieldType};

const RELATIVE_DATES: [FilterOperator; 16] = [
    FilterOperator::Today,
    FilterOperator::Yesterday,
    FilterOperator::Tomorrow,
    FilterOperator::ThisWeek,
    FilterOperator::LastWeek,
    FilterOperator::NextWeek,
    FilterOperator::ThisMonth,
    FilterOperator::LastMonth,
    FilterOperator::NextMonth,
    FilterOperator::ThisYear,
    FilterOperator::WithinLastDays,
    FilterOperator::WithinNextDays,
    FilterOperator::OlderThanDays,
    FilterOperator::Anniversary,
    FilterOperator::IsEmpty,
    FilterOperator::IsNotEmpty,
];

pub fn operators_for(field_type: FieldType) -> Vec<FilterOperator> {
    if matches!(field_type, FieldType::Custom { .. })
        && let Some(definition) = crate::application::fields::field_types::find(field_type.as_str())
    {
        return definition.operators.clone();
    }
    use FilterOperator::*;
    match field_type.base_type() {
        FieldType::Text | FieldType::Url | FieldType::Email | FieldType::Phone => vec![
            Eq,
            Neq,
            Contains,
            NotContains,
            StartsWith,
            In,
            NotIn,
            IsEmpty,
            IsNotEmpty,
        ],
        FieldType::LongText => vec![Contains, NotContains, IsEmpty, IsNotEmpty],
        FieldType::Number | FieldType::Currency => vec![
            Eq, Neq, Gt, Gte, Lt, Lte, Between, In, NotIn, IsEmpty, IsNotEmpty,
        ],
        FieldType::Date | FieldType::DateTime => [Eq, Neq, Gt, Gte, Lt, Lte, Between]
            .into_iter()
            .chain(RELATIVE_DATES)
            .collect(),
        FieldType::Select | FieldType::User | FieldType::Record => {
            vec![Eq, Neq, In, NotIn, IsEmpty, IsNotEmpty]
        }
        FieldType::MultiSelect => vec![In, NotIn, IsEmpty, IsNotEmpty],
        FieldType::Checkbox => vec![Eq, Neq],
        FieldType::Custom { .. } => unreachable!("native field representation"),
    }
}

pub fn supports(field_type: FieldType, operator: FilterOperator) -> bool {
    operators_for(field_type).contains(&operator)
}

pub fn hierarchical_target(definition: &FieldDefinition) -> Option<EntityDefinition> {
    if definition.field_type.base_type() != FieldType::Record {
        return None;
    }
    let target = definition.config.target_record_type.as_deref()?;
    let app = crate::modules::try_app()?;
    app.record_types()
        .require(target)
        .ok()
        .filter(|target| target.available_hierarchy().is_some())
        .cloned()
}

pub fn operators_for_definition(definition: &FieldDefinition) -> Vec<FilterOperator> {
    let mut operators = operators_for(definition.field_type);
    if hierarchical_target(definition).is_some() {
        operators.push(FilterOperator::InGroup);
    }
    operators
}

pub fn supports_definition(definition: &FieldDefinition, operator: FilterOperator) -> bool {
    operators_for_definition(definition).contains(&operator)
}
