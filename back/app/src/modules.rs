use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{App, AppBuilder, Module};

pub const REQUIRED: &[&str] = &[
    "people",
    "organizations",
    "deals",
    "notes",
    "tags",
    "integrations",
    "plugins",
    "exports",
    "org_export",
];

pub fn compiled() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(aphel_kernel::KernelModule),
        #[cfg(feature = "plugins")]
        Box::new(module_objects::ObjectsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_notifications::NotificationsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_config_packages::ConfigPackagesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_search::SearchModule),
        #[cfg(feature = "plugins")]
        Box::new(module_saved_views::SavedViewsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_record_layouts::RecordLayoutsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_formulas::FormulasModule),
        #[cfg(feature = "plugins")]
        Box::new(module_validation_rules::ValidationRulesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_marketplace::MarketplaceModule),
        #[cfg(feature = "plugins")]
        Box::new(module_people::PeopleModule),
        #[cfg(feature = "plugins")]
        Box::new(module_organizations::OrganizationsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_leads::LeadsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_scoring::ScoringModule),
        #[cfg(feature = "plugins")]
        Box::new(module_money::MoneyModule),
        #[cfg(feature = "plugins")]
        Box::new(module_workflow::WorkflowModule),
        #[cfg(feature = "plugins")]
        Box::new(module_deals::DealsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_line_items::LineItemsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_deals_revenue::DealsRevenueModule),
        #[cfg(feature = "plugins")]
        Box::new(module_renewals::RenewalsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_goals::GoalsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_forecast::ForecastModule),
        #[cfg(feature = "plugins")]
        Box::new(module_tasks::TasksModule),
        #[cfg(feature = "plugins")]
        Box::new(module_notes::NotesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_reminders::RemindersModule),
        #[cfg(feature = "plugins")]
        Box::new(module_timeline::TimelineModule),
        #[cfg(feature = "plugins")]
        Box::new(module_favorites::FavoritesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_themes::ThemesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_followers::FollowersModule),
        #[cfg(feature = "plugins")]
        Box::new(module_dashboard::DashboardModule),
        #[cfg(feature = "plugins")]
        Box::new(module_reports::ReportsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_onboarding::OnboardingModule),
        #[cfg(feature = "plugins")]
        Box::new(module_demo::DemoModule),
        #[cfg(feature = "plugins")]
        Box::new(module_assignment::AssignmentModule),
        #[cfg(feature = "plugins")]
        Box::new(module_tags::TagsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_segments::SegmentsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_calls::CallsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_email::EmailModule),
        #[cfg(feature = "plugins")]
        Box::new(module_sequences::SequencesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_scheduling::SchedulingModule),
        #[cfg(feature = "plugins")]
        Box::new(module_approvals::ApprovalsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_automations::AutomationsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_webhooks::WebhooksModule),
        #[cfg(feature = "plugins")]
        Box::new(module_integrations::IntegrationsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_plugins::PluginsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_imports::ImportsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_exports::ExportsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_bulk_actions::BulkActionsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_duplicates::DuplicatesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_trash::TrashModule),
        #[cfg(feature = "plugins")]
        Box::new(module_privacy::PrivacyModule),
        #[cfg(feature = "plugins")]
        Box::new(module_org_export::OrgExportModule),
        #[cfg(feature = "plugins")]
        Box::new(module_migrations_assistant::MigrationsAssistantModule),
        #[cfg(feature = "plugins")]
        Box::new(module_products::ProductsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_price_lists::PriceListsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_quotes::QuotesModule),
        #[cfg(feature = "plugins")]
        Box::new(module_proposals::ProposalsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_insights::InsightsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_lead_forms::LeadFormsModule),
        #[cfg(feature = "plugins")]
        Box::new(module_ai::AiModule),
        #[cfg(feature = "plugins")]
        Box::new(module_oidc::OidcModule),
        #[cfg(feature = "plugins")]
        Box::new(module_google::GoogleModule),
        #[cfg(feature = "plugins")]
        Box::new(module_microsoft::MicrosoftModule),
        #[cfg(feature = "plugins")]
        Box::new(module_esign::EsignModule),
        #[cfg(feature = "plugins")]
        Box::new(module_enrichment::EnrichmentModule),
        #[cfg(feature = "plugins")]
        Box::new(module_messaging::MessagingModule),
        #[cfg(feature = "plugins")]
        Box::new(module_calendar::CalendarModule),
        #[cfg(feature = "plugins")]
        Box::new(module_documents::DocumentsModule),
    ]
}

pub fn build() -> Result<App, AppError> {
    let builder = AppBuilder::new();
    #[cfg(feature = "plugins")]
    let builder = {
        let mut builder = builder;
        crate::blueprints::register(&mut builder);
        builder
    };
    builder
        .modules(compiled())
        .require(REQUIRED.iter().copied())
        .build()
}
