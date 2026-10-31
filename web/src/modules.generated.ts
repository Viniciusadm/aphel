import type { WebModule } from '@/kernel/modules/define'
import kernelModule from '@modules/kernel'
import contactsModule from '@modules/contacts'
import companiesModule from '@modules/companies'
import leadsModule from '@modules/leads'
import scoringModule from '@modules/scoring'
import currenciesModule from '@modules/currencies'
import pipelinesModule from '@modules/pipelines'
import dealsModule from '@modules/deals'
import dealItemsModule from '@modules/deal_items'
import renewalsModule from '@modules/renewals'
import goalsModule from '@modules/goals'
import forecastModule from '@modules/forecast'
import activitiesModule from '@modules/activities'
import notesModule from '@modules/notes'
import timelineModule from '@modules/timeline'
import favoritesModule from '@modules/favorites'
import themesModule from '@modules/themes'
import followersModule from '@modules/followers'
import dashboardModule from '@modules/dashboard'
import reportsModule from '@modules/reports'
import onboardingModule from '@modules/onboarding'
import leadDistributionModule from '@modules/lead_distribution'
import tagsModule from '@modules/tags'
import segmentsModule from '@modules/segments'
import callsModule from '@modules/calls'
import emailModule from '@modules/email'
import sequencesModule from '@modules/sequences'
import schedulingModule from '@modules/scheduling'
import approvalsModule from '@modules/approvals'
import automationsModule from '@modules/automations'
import webhooksModule from '@modules/webhooks'
import integrationsModule from '@modules/integrations'
import importsModule from '@modules/imports'
import exportsModule from '@modules/exports'
import bulkActionsModule from '@modules/bulk_actions'
import duplicatesModule from '@modules/duplicates'
import trashModule from '@modules/trash'
import privacyModule from '@modules/privacy'
import orgExportModule from '@modules/org_export'
import migrationsAssistantModule from '@modules/migrations_assistant'
import productsModule from '@modules/products'
import priceListsModule from '@modules/price_lists'
import quotesModule from '@modules/quotes'
import proposalsModule from '@modules/proposals'
import insightsModule from '@modules/insights'
import leadFormsModule from '@modules/lead_forms'
import aiModule from '@modules/ai'
import oidcModule from '@modules/oidc'
import googleModule from '@modules/google'
import microsoftModule from '@modules/microsoft'
import esignModule from '@modules/esign'
import enrichmentModule from '@modules/enrichment'
import messagingModule from '@modules/messaging'

export const MODULES: WebModule[] = [
  kernelModule,
  contactsModule,
  companiesModule,
  leadsModule,
  scoringModule,
  currenciesModule,
  pipelinesModule,
  dealsModule,
  dealItemsModule,
  renewalsModule,
  goalsModule,
  forecastModule,
  activitiesModule,
  notesModule,
  timelineModule,
  favoritesModule,
  themesModule,
  followersModule,
  dashboardModule,
  reportsModule,
  onboardingModule,
  leadDistributionModule,
  tagsModule,
  segmentsModule,
  callsModule,
  emailModule,
  sequencesModule,
  schedulingModule,
  approvalsModule,
  automationsModule,
  webhooksModule,
  integrationsModule,
  importsModule,
  exportsModule,
  bulkActionsModule,
  duplicatesModule,
  trashModule,
  privacyModule,
  orgExportModule,
  migrationsAssistantModule,
  productsModule,
  priceListsModule,
  quotesModule,
  proposalsModule,
  insightsModule,
  leadFormsModule,
  aiModule,
  oidcModule,
  googleModule,
  microsoftModule,
  esignModule,
  enrichmentModule,
  messagingModule,
]
