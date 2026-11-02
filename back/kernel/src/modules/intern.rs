use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub fn intern(value: &str) -> &'static str {
    static VALUES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut values = VALUES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = values.get(value) {
        return existing;
    }
    let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
    values.insert(leaked);
    leaked
}
