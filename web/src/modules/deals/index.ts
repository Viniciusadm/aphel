import { DealsDashboardEventLabel } from './dashboard/EventLabel'
import { DealPendingSections, type DealPendingData } from './dashboard/PendingSections'
import { SalesIndicatorsWidget } from './dashboard/SalesIndicatorsWidget'
import { DealsByStageWidget } from './dashboard/DealsByStageWidget'
import type { SalesIndicatorsData, DealsByStageData } from './dashboard/types'
import { createElement } from 'react'
import { DealsSummary } from './dashboard/Summary'
import { DealReportLibraryCards } from './reports/LibraryCards'
import { dealReportRoutes } from './reports/routes'
import { LineItemAmountLock } from './LineItemAmountLock'
import { RottingDealBadge } from './RottingDealBadge'
import { Briefcase, Users, UsersRound } from 'lucide-react'
import { SETTINGS_MANAGE, rottingOf } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { CreateDealDialog } from './create-action'
import { DealTeamRecordTab, RelatedDealsTab } from './record-tabs'
import { DEAL_RECORD_TYPE, DEALS_PATH } from './resource'
import { dealsRoutes } from './routes'
import { dealMemberRolesSettingsRoute } from './team/routes'
import { DEAL_TEAM_TIMELINE_RENDERERS } from './team/timeline'
import './slots'

const dealsModule = defineModule({
  key: 'deals',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: {
    notifications: {
      'pt-BR': {
        kinds: {
          deal_team_member_added: {
            label: 'Entrada na equipe de oportunidade',
            description: 'Quando alguém adiciona você à equipe de uma oportunidade.',
          },
        },
      },
      en: {
        kinds: {
          deal_team_member_added: {
            label: 'Added to a deal team',
            description: 'When someone adds you to the team of a deal.',
          },
        },
      },
    },
  },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.filter('dashboard.eventLabel', (label, { event }) => event.event_type.startsWith('deal.') ? createElement(DealsDashboardEventLabel, { event }) : label)
    app.slot('dashboard.source', { key: 'deals.summary', render: ({ collection, source, data }) => collection === 'summary' && source === 'deals.summary' ? createElement(DealsSummary, { data: data as Record<string, unknown> }) : null })
    app.slot('dashboard.source', { key: 'deals.stale', render: ({ collection, source, data }) => collection === 'pending' && source === 'deals.stale' ? createElement(DealPendingSections, { data: data as DealPendingData }) : null })
    app.widget('sales_indicators', { render: (data) => createElement(SalesIndicatorsWidget, { data: data as SalesIndicatorsData }) })
    app.widget('deals_by_stage', { render: (data) => createElement(DealsByStageWidget, { data: data as DealsByStageData }), isEmpty: (data) => { const stages = data as DealsByStageData; return stages.pipeline !== null && stages.stages.every((stage) => stage.count === 0) } })
    app.slot('deals.detail.fieldLocks', { key: 'line-item-amount', render: LineItemAmountLock })
    app.slot('deals.badges', { key: 'rotting', render: RottingDealBadge })
    app.filter('deals.tone', (tone, { deal }) => (rottingOf(deal).isRotting ? 'warning' : tone))
    app.routes(dealReportRoutes)
    app.slot('reports.library', { key: 'sales', permission: 'deals.read', render: DealReportLibraryCards })
    app.routes(dealsRoutes)
    app.nav({ section: 'records', to: DEALS_PATH, label: 'nav.deals', icon: Briefcase, permission: ['deals.read'], order: 50 })
    app.createAction({
      id: 'deal',
      label: 'createAction.label',
      icon: Briefcase,
      permission: 'deals.create',
      recordType: DEAL_RECORD_TYPE,
      order: 30,
      render: CreateDealDialog,
    })
    app.settings('organization', {
      group: 'sales',
      path: 'deal-team-roles',
      label: 'settings:tabs.dealTeamRoles',
      description: 'settings:items.dealTeamRoles.description',
      icon: UsersRound,
      permission: SETTINGS_MANAGE,
      order: 110,
      routes: [dealMemberRolesSettingsRoute],
    })
    app.recordSection(['people.person', 'organizations.organization'], {
      key: 'deals',
      group: 'relationships',
      label: 'related.tab',
      icon: Briefcase,
      order: 5,
      render: RelatedDealsTab,
    })
    app.recordSection(DEAL_RECORD_TYPE, {
      key: 'team',
      label: 'team.tab',
      icon: Users,
      order: 30,
      render: DealTeamRecordTab,
    })
    for (const [eventType, renderer] of Object.entries(DEAL_TEAM_TIMELINE_RENDERERS))
      app.eventRenderer(eventType, renderer)
  },
})

export default dealsModule
