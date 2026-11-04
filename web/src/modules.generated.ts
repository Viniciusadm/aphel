import type { WebModule } from '@/kernel/modules/define'
import kernelModule from '@modules/kernel'
import objectsModule from '@modules/objects'
import notificationsModule from '@modules/notifications'
import configPackagesModule from '@modules/config_packages'
import searchModule from '@modules/search'
import savedViewsModule from '@modules/saved_views'
import recordLayoutsModule from '@modules/record_layouts'
import formulasModule from '@modules/formulas'
import validationRulesModule from '@modules/validation_rules'
import marketplaceModule from '@modules/marketplace'
import peopleModule from '@modules/people'
import organizationsModule from '@modules/organizations'
import leadsModule from '@modules/leads'
import scoringModule from '@modules/scoring'
import moneyModule from '@modules/money'
import workflowModule from '@modules/workflow'
import dealsModule from '@modules/deals'
import lineItemsModule from '@modules/line_items'
import dealsRevenueModule from '@modules/deals_revenue'
import renewalsModule from '@modules/renewals'
import goalsModule from '@modules/goals'
import forecastModule from '@modules/forecast'
import tasksModule from '@modules/tasks'
import notesModule from '@modules/notes'
import timelineModule from '@modules/timeline'
import favoritesModule from '@modules/favorites'
import themesModule from '@modules/themes'
import followersModule from '@modules/followers'
import dashboardModule from '@modules/dashboard'
import reportsModule from '@modules/reports'
import onboardingModule from '@modules/onboarding'
import assignmentModule from '@modules/assignment'
import taggingModule from '@modules/tagging'
import segmentsModule from '@modules/segments'
import telephonyModule from '@modules/telephony'
import emailModule from '@modules/email'
import sequencesModule from '@modules/sequences'
import bookingModule from '@modules/booking'
import approvalsModule from '@modules/approvals'
import automationsModule from '@modules/automations'
import webhooksModule from '@modules/webhooks'
import integrationsModule from '@modules/integrations'
import wasmRuntimeModule from '@modules/wasm_runtime'
import importsModule from '@modules/imports'
import exportsModule from '@modules/exports'
import bulkActionsModule from '@modules/bulk_actions'
import duplicatesModule from '@modules/duplicates'
import trashModule from '@modules/trash'
import privacyModule from '@modules/privacy'
import orgExportModule from '@modules/org_export'
import migrationsAssistantModule from '@modules/migrations_assistant'
import catalogItemsModule from '@modules/catalog_items'
import pricingModule from '@modules/pricing'
import quotesModule from '@modules/quotes'
import proposalsModule from '@modules/proposals'
import insightsModule from '@modules/insights'
import formsModule from '@modules/forms'
import aiModule from '@modules/ai'
import ssoModule from '@modules/sso'
import googleModule from '@modules/google'
import microsoftModule from '@modules/microsoft'
import esignModule from '@modules/esign'
import enrichmentModule from '@modules/enrichment'
import messagingModule from '@modules/messaging'
import calendarModule from '@modules/calendar'

export const MODULES: WebModule[] = [
  kernelModule,
  objectsModule,
  notificationsModule,
  configPackagesModule,
  searchModule,
  savedViewsModule,
  recordLayoutsModule,
  formulasModule,
  validationRulesModule,
  marketplaceModule,
  peopleModule,
  organizationsModule,
  leadsModule,
  scoringModule,
  moneyModule,
  workflowModule,
  dealsModule,
  lineItemsModule,
  dealsRevenueModule,
  renewalsModule,
  goalsModule,
  forecastModule,
  tasksModule,
  notesModule,
  timelineModule,
  favoritesModule,
  themesModule,
  followersModule,
  dashboardModule,
  reportsModule,
  onboardingModule,
  assignmentModule,
  taggingModule,
  segmentsModule,
  telephonyModule,
  emailModule,
  sequencesModule,
  bookingModule,
  approvalsModule,
  automationsModule,
  webhooksModule,
  integrationsModule,
  wasmRuntimeModule,
  importsModule,
  exportsModule,
  bulkActionsModule,
  duplicatesModule,
  trashModule,
  privacyModule,
  orgExportModule,
  migrationsAssistantModule,
  catalogItemsModule,
  pricingModule,
  quotesModule,
  proposalsModule,
  insightsModule,
  formsModule,
  aiModule,
  ssoModule,
  googleModule,
  microsoftModule,
  esignModule,
  enrichmentModule,
  messagingModule,
  calendarModule,
]
