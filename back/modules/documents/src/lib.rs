mod pdf;
use aphel_kernel::modules::{AppBuilder, Manifest, Module, builder::APHEL_VERSION};
use aphel_sdk::{
    AppError,
    documents::{Document, DocumentRenderer},
};
pub struct DocumentsModule;
pub const KEY: &str = "documents";
struct PdfRenderer;
impl DocumentRenderer for PdfRenderer {
    fn render(&self, document: &Document) -> Result<Vec<u8>, AppError> {
        pdf::render(document)
    }
}
impl Module for DocumentsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Documents", APHEL_VERSION)
            .description("Shared document rendering, layout and owner-authorized sources")
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_sdk::documents::DESCRIPTOR);
        app.provide::<dyn DocumentRenderer>(std::sync::Arc::new(PdfRenderer));
        Ok(())
    }
}
