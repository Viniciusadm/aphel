use crate::application::directory::{DirectoryQuery, DirectoryUser, UserDirectory};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::errors::AppError;

const MAX_SEARCH_LENGTH: usize = 100;

pub struct DirectoryUseCases<D> {
    directory: D,
}

impl<D> DirectoryUseCases<D>
where
    D: UserDirectory,
{
    pub fn new(directory: D) -> Self {
        Self { directory }
    }

    pub async fn list_users(
        &self,
        search: Option<String>,
        window: ListWindow,
    ) -> Result<Listing<DirectoryUser>, AppError> {
        let search = search
            .map(|search| search.trim().to_owned())
            .filter(|search| !search.is_empty());
        if search
            .as_ref()
            .is_some_and(|search| search.chars().count() > MAX_SEARCH_LENGTH)
        {
            return Err(AppError::Validation(format!(
                "search must have at most {MAX_SEARCH_LENGTH} characters"
            )));
        }
        self.directory
            .list_active(&DirectoryQuery { search }, window)
            .await
    }
}
