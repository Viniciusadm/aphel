#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldType {
    Text,
    LongText,
    Number,
    Currency,
    Date,
    DateTime,
    Select,
    MultiSelect,
    Checkbox,
    Url,
    Email,
    Phone,
    User,
    Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueColumn {
    Text,
    LongText,
    Number,
    Date,
    DateTime,
    Bool,
    Option,
    MultiOption,
}

impl ValueColumn {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::Text => "value_text",
            Self::LongText => "value_long_text",
            Self::Number => "value_number",
            Self::Date => "value_date",
            Self::DateTime => "value_datetime",
            Self::Bool => "value_bool",
            Self::Option | Self::MultiOption => "value_option_id",
        }
    }
}

impl FieldType {
    pub const ALL: [Self; 14] = [
        Self::Text,
        Self::LongText,
        Self::Number,
        Self::Currency,
        Self::Date,
        Self::DateTime,
        Self::Select,
        Self::MultiSelect,
        Self::Checkbox,
        Self::Url,
        Self::Email,
        Self::Phone,
        Self::User,
        Self::Record,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::LongText => "long_text",
            Self::Number => "number",
            Self::Currency => "currency",
            Self::Date => "date",
            Self::DateTime => "datetime",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Checkbox => "checkbox",
            Self::Url => "url",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::User => "user",
            Self::Record => "record",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|field_type| field_type.as_str() == value)
    }

    pub fn value_column(&self) -> ValueColumn {
        match self {
            Self::Text | Self::Url | Self::Email | Self::Phone | Self::User | Self::Record => {
                ValueColumn::Text
            }
            Self::LongText => ValueColumn::LongText,
            Self::Number | Self::Currency => ValueColumn::Number,
            Self::Date => ValueColumn::Date,
            Self::DateTime => ValueColumn::DateTime,
            Self::Checkbox => ValueColumn::Bool,
            Self::Select => ValueColumn::Option,
            Self::MultiSelect => ValueColumn::MultiOption,
        }
    }

    pub fn has_options(&self) -> bool {
        matches!(self, Self::Select | Self::MultiSelect)
    }

    pub fn is_textual(&self) -> bool {
        matches!(
            self,
            Self::Text | Self::LongText | Self::Url | Self::Email | Self::Phone
        )
    }

    pub fn supports_unique(&self) -> bool {
        matches!(
            self,
            Self::Text | Self::Number | Self::Url | Self::Email | Self::Phone
        )
    }

    pub fn is_sortable(&self) -> bool {
        !matches!(
            self,
            Self::MultiSelect | Self::LongText | Self::User | Self::Record
        )
    }

    pub fn can_change_to(&self, target: Self) -> bool {
        if *self == target {
            return true;
        }
        let group = |field_type: Self| match field_type {
            Self::Text | Self::Url | Self::Email | Self::Phone => Some(0),
            Self::Number | Self::Currency => Some(1),
            _ => None,
        };
        matches!((group(*self), group(target)), (Some(from), Some(to)) if from == to)
    }
}
