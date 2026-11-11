#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockError {
    pub code: String,
    pub field: String,
    pub message: String,
}

impl BlockError {
    pub fn new(code: &str, field: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            field: field.to_owned(),
            message: message.into(),
        }
    }

    pub fn at(mut self, field: &str) -> Self {
        self.field = field.to_owned();
        self
    }
}

#[cfg(feature = "plugin")]
impl From<BlockError> for aphel_plugin_sdk::PluginError {
    fn from(error: BlockError) -> Self {
        aphel_plugin_sdk::PluginError::field(error.code, error.field, error.message)
    }
}
