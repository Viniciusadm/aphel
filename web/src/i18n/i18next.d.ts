import 'i18next'
import type aiNamespace from '../modules/ai/i18n/pt-BR.json'
import type apiTokensNamespace from './pt-BR/api-tokens.json'
import type approvalsNamespace from '../modules/approvals/i18n/pt-BR.json'
import type assignmentNamespace from '../modules/assignment/i18n/pt-BR.json'
import type auditNamespace from './pt-BR/audit.json'
import type authNamespace from './pt-BR/auth.json'
import type automationsNamespace from '../modules/automations/i18n/pt-BR.json'
import type bulkActionsNamespace from '../modules/bulk_actions/i18n/pt-BR.json'
import type calendarNamespace from '../modules/calendar/i18n/pt-BR.json'
import type callsNamespace from '../modules/calls/i18n/pt-BR.json'
import type catalogNamespace from './pt-BR/catalog.json'
import type commandPaletteNamespace from './pt-BR/command-palette.json'
import type commonNamespace from './pt-BR/common.json'
import type configPackageVersioningNamespace from './pt-BR/config-package-versioning.json'
import type configPackagesNamespace from './pt-BR/config-packages.json'
import type currenciesNamespace from '../modules/currencies/i18n/pt-BR.json'
import type dashboardNamespace from '../modules/dashboard/i18n/pt-BR.json'
import type dealItemsNamespace from '../modules/deal_items/i18n/pt-BR.json'
import type dealsNamespace from '../modules/deals/i18n/pt-BR.json'
import type demoNamespace from './pt-BR/demo.json'
import type duplicatesNamespace from '../modules/duplicates/i18n/pt-BR.json'
import type emailNamespace from '../modules/email/i18n/pt-BR.json'
import type enrichmentNamespace from '../modules/enrichment/i18n/pt-BR.json'
import type esignNamespace from '../modules/esign/i18n/pt-BR.json'
import type exportsNamespace from '../modules/exports/i18n/pt-BR.json'
import type favoritesNamespace from '../modules/favorites/i18n/pt-BR.json'
import type fieldPermissionsNamespace from './pt-BR/field-permissions.json'
import type fieldsNamespace from './pt-BR/fields.json'
import type filesNamespace from './pt-BR/files.json'
import type followersNamespace from '../modules/followers/i18n/pt-BR.json'
import type forecastNamespace from '../modules/forecast/i18n/pt-BR.json'
import type formulasNamespace from './pt-BR/formulas.json'
import type goalsNamespace from '../modules/goals/i18n/pt-BR.json'
import type googleNamespace from '../modules/google/i18n/pt-BR.json'
import type importsNamespace from '../modules/imports/i18n/pt-BR.json'
import type insightsNamespace from '../modules/insights/i18n/pt-BR.json'
import type integrationCardNamespace from './pt-BR/integration-card.json'
import type integrationsNamespace from '../modules/integrations/i18n/pt-BR.json'
import type layoutNamespace from './pt-BR/layout.json'
import type layoutsNamespace from './pt-BR/layouts.json'
import type leadFormsNamespace from '../modules/lead_forms/i18n/pt-BR.json'
import type leadProNamespace from '../modules/lead_pro/i18n/pt-BR.json'
import type leadsNamespace from '../modules/leads/i18n/pt-BR.json'
import type messagingNamespace from '../modules/messaging/i18n/pt-BR.json'
import type microsoftNamespace from '../modules/microsoft/i18n/pt-BR.json'
import type migrationsAssistantNamespace from '../modules/migrations_assistant/i18n/pt-BR.json'
import type mobileNamespace from './pt-BR/mobile.json'
import type modulesNamespace from './pt-BR/modules.json'
import type navigationSettingsNamespace from './pt-BR/navigation-settings.json'
import type notesNamespace from '../modules/notes/i18n/pt-BR.json'
import type notificationsNamespace from './pt-BR/notifications.json'
import type objectsNamespace from './pt-BR/objects.json'
import type offlineNamespace from './pt-BR/offline.json'
import type oidcNamespace from '../modules/oidc/i18n/pt-BR.json'
import type onboardingNamespace from '../modules/onboarding/i18n/pt-BR.json'
import type orgExportNamespace from '../modules/org_export/i18n/pt-BR.json'
import type organizationSettingsNamespace from './pt-BR/organization-settings.json'
import type organizationsNamespace from '../modules/organizations/i18n/pt-BR.json'
import type peopleNamespace from '../modules/people/i18n/pt-BR.json'
import type priceListsNamespace from '../modules/price_lists/i18n/pt-BR.json'
import type privacyNamespace from '../modules/privacy/i18n/pt-BR.json'
import type productsNamespace from '../modules/products/i18n/pt-BR.json'
import type proposalsNamespace from '../modules/proposals/i18n/pt-BR.json'
import type pwaNamespace from './pt-BR/pwa.json'
import type quotesNamespace from '../modules/quotes/i18n/pt-BR.json'
import type recordsNamespace from './pt-BR/records.json'
import type recordsCalendarNamespace from './pt-BR/records-calendar.json'
import type recordsDetailNamespace from './pt-BR/records-detail.json'
import type recordsFormNamespace from './pt-BR/records-form.json'
import type recordsTableNamespace from './pt-BR/records-table.json'
import type renewalsNamespace from '../modules/renewals/i18n/pt-BR.json'
import type reportsNamespace from '../modules/reports/i18n/pt-BR.json'
import type schedulingNamespace from '../modules/scheduling/i18n/pt-BR.json'
import type scoringNamespace from '../modules/scoring/i18n/pt-BR.json'
import type searchNamespace from './pt-BR/search.json'
import type securityNamespace from './pt-BR/security.json'
import type segmentsNamespace from '../modules/segments/i18n/pt-BR.json'
import type sequencesNamespace from '../modules/sequences/i18n/pt-BR.json'
import type settingsNamespace from './pt-BR/settings.json'
import type setupNamespace from './pt-BR/setup.json'
import type sharingNamespace from './pt-BR/sharing.json'
import type systemNamespace from './pt-BR/system.json'
import type tagsNamespace from '../modules/tags/i18n/pt-BR.json'
import type tasksNamespace from '../modules/tasks/i18n/pt-BR.json'
import type themesNamespace from '../modules/themes/i18n/pt-BR.json'
import type timelineNamespace from '../modules/timeline/i18n/pt-BR.json'
import type trashNamespace from '../modules/trash/i18n/pt-BR.json'
import type twoFactorNamespace from './pt-BR/two-factor.json'
import type validationMessagesNamespace from './pt-BR/validation-messages.json'
import type validationRulesNamespace from './pt-BR/validation-rules.json'
import type webhooksNamespace from '../modules/webhooks/i18n/pt-BR.json'
import type workflowNamespace from '../modules/workflow/i18n/pt-BR.json'

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'common'
    resources: {
      ai: typeof aiNamespace
      'api-tokens': typeof apiTokensNamespace
      approvals: typeof approvalsNamespace
      assignment: typeof assignmentNamespace
      audit: typeof auditNamespace
      auth: typeof authNamespace
      automations: typeof automationsNamespace
      bulk_actions: typeof bulkActionsNamespace
      calendar: typeof calendarNamespace
      calls: typeof callsNamespace
      catalog: typeof catalogNamespace
      'command-palette': typeof commandPaletteNamespace
      common: typeof commonNamespace
      'config-package-versioning': typeof configPackageVersioningNamespace
      'config-packages': typeof configPackagesNamespace
      currencies: typeof currenciesNamespace
      dashboard: typeof dashboardNamespace
      deal_items: typeof dealItemsNamespace
      deals: typeof dealsNamespace
      demo: typeof demoNamespace
      duplicates: typeof duplicatesNamespace
      email: typeof emailNamespace
      enrichment: typeof enrichmentNamespace
      esign: typeof esignNamespace
      exports: typeof exportsNamespace
      favorites: typeof favoritesNamespace
      'field-permissions': typeof fieldPermissionsNamespace
      fields: typeof fieldsNamespace
      files: typeof filesNamespace
      followers: typeof followersNamespace
      forecast: typeof forecastNamespace
      formulas: typeof formulasNamespace
      goals: typeof goalsNamespace
      google: typeof googleNamespace
      imports: typeof importsNamespace
      insights: typeof insightsNamespace
      'integration-card': typeof integrationCardNamespace
      integrations: typeof integrationsNamespace
      layout: typeof layoutNamespace
      layouts: typeof layoutsNamespace
      lead_forms: typeof leadFormsNamespace
      lead_pro: typeof leadProNamespace
      leads: typeof leadsNamespace
      messaging: typeof messagingNamespace
      microsoft: typeof microsoftNamespace
      migrations_assistant: typeof migrationsAssistantNamespace
      mobile: typeof mobileNamespace
      modules: typeof modulesNamespace
      'navigation-settings': typeof navigationSettingsNamespace
      notes: typeof notesNamespace
      notifications: typeof notificationsNamespace
      objects: typeof objectsNamespace
      offline: typeof offlineNamespace
      oidc: typeof oidcNamespace
      onboarding: typeof onboardingNamespace
      org_export: typeof orgExportNamespace
      'organization-settings': typeof organizationSettingsNamespace
      organizations: typeof organizationsNamespace
      people: typeof peopleNamespace
      price_lists: typeof priceListsNamespace
      privacy: typeof privacyNamespace
      products: typeof productsNamespace
      proposals: typeof proposalsNamespace
      pwa: typeof pwaNamespace
      quotes: typeof quotesNamespace
      records: typeof recordsNamespace
      'records-calendar': typeof recordsCalendarNamespace
      'records-detail': typeof recordsDetailNamespace
      'records-form': typeof recordsFormNamespace
      'records-table': typeof recordsTableNamespace
      renewals: typeof renewalsNamespace
      reports: typeof reportsNamespace
      scheduling: typeof schedulingNamespace
      scoring: typeof scoringNamespace
      search: typeof searchNamespace
      security: typeof securityNamespace
      segments: typeof segmentsNamespace
      sequences: typeof sequencesNamespace
      settings: typeof settingsNamespace
      setup: typeof setupNamespace
      sharing: typeof sharingNamespace
      system: typeof systemNamespace
      tags: typeof tagsNamespace
      tasks: typeof tasksNamespace
      themes: typeof themesNamespace
      timeline: typeof timelineNamespace
      trash: typeof trashNamespace
      'two-factor': typeof twoFactorNamespace
      'validation-messages': typeof validationMessagesNamespace
      'validation-rules': typeof validationRulesNamespace
      webhooks: typeof webhooksNamespace
      workflow: typeof workflowNamespace
    }
  }
}
