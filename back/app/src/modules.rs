use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{App, AppBuilder, Module};

pub const REQUIRED: &[&str] = &[
    "contacts",
    "companies",
    "currencies",
    "pipelines",
    "deals",
    "activities",
    "notes",
    "timeline",
    "tags",
    "integrations",
    "plugins",
    "exports",
    "org_export",
];

pub fn compiled() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(aphel_kernel::KernelModule),
        Box::new(module_contacts::ContactsModule),
        Box::new(module_companies::CompaniesModule),
        Box::new(module_leads::LeadsModule),
        Box::new(module_scoring::ScoringModule),
        Box::new(module_currencies::CurrenciesModule),
        Box::new(module_pipelines::PipelinesModule),
        Box::new(module_deals::DealsModule),
        Box::new(module_deal_items::DealItemsModule),
        Box::new(module_renewals::RenewalsModule),
        Box::new(module_goals::GoalsModule),
        Box::new(module_forecast::ForecastModule),
        Box::new(module_activities::ActivitiesModule),
        Box::new(module_notes::NotesModule),
        Box::new(module_reminders::RemindersModule),
        Box::new(module_timeline::TimelineModule),
        Box::new(module_favorites::FavoritesModule),
        Box::new(module_followers::FollowersModule),
        Box::new(module_dashboard::DashboardModule),
        Box::new(module_reports::ReportsModule),
        Box::new(module_onboarding::OnboardingModule),
        Box::new(module_demo::DemoModule),
        Box::new(module_lead_distribution::LeadDistributionModule),
        Box::new(module_tags::TagsModule),
        Box::new(module_segments::SegmentsModule),
        Box::new(module_calls::CallsModule),
        Box::new(module_email::EmailModule),
        Box::new(module_sequences::SequencesModule),
        Box::new(module_scheduling::SchedulingModule),
        Box::new(module_approvals::ApprovalsModule),
        Box::new(module_automations::AutomationsModule),
        Box::new(module_webhooks::WebhooksModule),
        Box::new(module_integrations::IntegrationsModule),
        Box::new(module_plugins::PluginsModule),
        Box::new(module_imports::ImportsModule),
        Box::new(module_exports::ExportsModule),
        Box::new(module_bulk_actions::BulkActionsModule),
        Box::new(module_duplicates::DuplicatesModule),
        Box::new(module_trash::TrashModule),
        Box::new(module_privacy::PrivacyModule),
        Box::new(module_org_export::OrgExportModule),
        Box::new(module_migrations_assistant::MigrationsAssistantModule),
        Box::new(module_products::ProductsModule),
        Box::new(module_price_lists::PriceListsModule),
        Box::new(module_quotes::QuotesModule),
        Box::new(module_proposals::ProposalsModule),
        Box::new(module_insights::InsightsModule),
        Box::new(module_lead_forms::LeadFormsModule),
        Box::new(module_ai::AiModule),
        Box::new(module_oidc::OidcModule),
        Box::new(module_google::GoogleModule),
        Box::new(module_microsoft::MicrosoftModule),
        Box::new(module_esign::EsignModule),
        Box::new(module_enrichment::EnrichmentModule),
        Box::new(module_messaging::MessagingModule),
    ]
}

pub fn build() -> Result<App, AppError> {
    AppBuilder::new()
        .modules(compiled())
        .require(REQUIRED.iter().copied())
        .build()
}
