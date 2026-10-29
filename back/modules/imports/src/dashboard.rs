use sqlx::{MySql, QueryBuilder};

use aphel_kernel::infrastructure::repositories::mysql_dashboard::RecentEventExclusion;

use crate::writer::IMPORT_EVENT_ORIGIN;

pub(crate) struct ImportedEventsExclusion;

impl RecentEventExclusion for ImportedEventsExclusion {
    fn exclude(&self, builder: &mut QueryBuilder<MySql>) {
        builder
            .push(" AND (e.origin_type IS NULL OR e.origin_type <> ")
            .push_bind(IMPORT_EVENT_ORIGIN)
            .push(")");
    }
}
