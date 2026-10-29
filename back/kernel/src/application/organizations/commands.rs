#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOrganizationCommand {
    pub name: String,
    pub primary_color: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub currency: String,
    pub logo_file_id: Option<Option<String>>,
    pub ip: Option<String>,
}
