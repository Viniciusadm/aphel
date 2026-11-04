use serde_json::Value;
#[derive(Debug, Clone)]
pub struct FragmentCollection {
    pub path: Option<&'static str>,
    pub kind: &'static str,
    pub key_fields: &'static [&'static str],
}
impl FragmentCollection {
    pub fn key(&self, entry: &Value) -> Option<String> {
        self.key_fields
            .iter()
            .map(|field| entry.get(*field)?.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .map(|keys| keys.join("."))
    }
}
#[derive(Debug, Clone)]
pub struct SectionFragmentLayout {
    pub section: &'static str,
    pub collections: Vec<FragmentCollection>,
}
#[derive(Debug, Clone)]
pub struct NestedFragmentLayout {
    pub section: &'static str,
    pub parent_kind: &'static str,
    pub child_kind: &'static str,
    pub children_field: &'static str,
}
