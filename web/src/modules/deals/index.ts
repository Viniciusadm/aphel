import { Briefcase, Users, UsersRound } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
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
    app.routes(dealsRoutes)
    app.nav({ to: DEALS_PATH, label: 'nav.deals', icon: Briefcase, permission: ['deals.read'], order: 50 })
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
