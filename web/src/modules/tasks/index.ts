import { TasksDashboardEventLabel } from './dashboard/EventLabel'
import { TaskPendingSections } from './dashboard/PendingSections'
import type { TaskPendingData } from './dashboard/types'
import { createElement } from 'react'
import { TasksSummary } from './dashboard/Summary'
import { CalendarCreateDialog } from "./CalendarCreateDialog"
import { CalendarCheck, CheckSquare, Inbox, MapPin } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { CreateTaskDialog } from './create-action'
import { inboxRoutes } from './inbox/routes'
import { ActivityQuickForm } from './mobile/ActivityQuickForm'
import { CheckInQuickForm } from './mobile/CheckInQuickForm'
import { withActivitiesTab } from './record-tabs'
import { ACTIVITY_TYPE_REFERENCE_OPTIONS } from './reference-options'
import { ACTIVITY_RECORD_TYPE } from './resource'
import { activitiesRoutes, activityTypesSettingsRoute } from './routes'
import { ACTIVITY_TIMELINE_RENDERERS } from './timeline/renderers'

const TASKS_READ = ['tasks.read']

const activitiesModule = defineModule({
  key: 'tasks',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.filter('dashboard.eventLabel', (label, { event }) => event.event_type.startsWith('activity.') ? createElement(TasksDashboardEventLabel, { event }) : label)
    app.slot('dashboard.source', { key: 'tasks.summary', render: ({ collection, source, data }) => collection === 'summary' && source === 'tasks.summary' ? createElement(TasksSummary, { data: data as Record<string, unknown> }) : null })
    app.slot('dashboard.source', { key: 'tasks.pending', render: ({ collection, source, data }) => collection === 'pending' && source === 'tasks.pending' ? createElement(TaskPendingSections, { data: data as TaskPendingData }) : null })
    app.slot('calendar.create', { key: 'tasks', permission: 'tasks.create', render: CalendarCreateDialog })
    app.routes([...activitiesRoutes, ...inboxRoutes])
    app.nav({ to: '/tasks', label: 'nav.tasks', icon: CheckSquare, permission: TASKS_READ, order: 80 })
    app.nav({ to: '/inbox', label: 'nav.inbox', icon: Inbox, permission: TASKS_READ, order: 90 })
    app.createAction({
      id: 'task',
      label: 'createAction.label',
      linkedLabel: 'createAction.linked',
      icon: CheckSquare,
      permission: 'tasks.create',
      recordType: ACTIVITY_RECORD_TYPE,
      targetRecordTypes: ['people.person', 'organizations.organization', 'deals.deal'],
      order: 40,
      render: CreateTaskDialog,
    })
    app.quickAdd({
      key: 'checkin',
      label: 'quickAdd.options.checkin',
      title: 'quickAdd.titles.checkin',
      recordShortcut: 'quickAdd.checkIn.button',
      icon: MapPin,
      permission: 'tasks.create',
      order: 10,
      render: CheckInQuickForm,
    })
    app.quickAdd({
      key: 'activity',
      label: 'quickAdd.options.activity',
      title: 'quickAdd.titles.activity',
      icon: CalendarCheck,
      permission: 'tasks.create',
      order: 20,
      render: ActivityQuickForm,
    })
    app.settings('organization', {
      group: 'records',
      path: 'activity-types',
      label: 'settings:tabs.activityTypes',
      description: 'settings:items.activityTypes.description',
      icon: CalendarCheck,
      permission: SETTINGS_MANAGE,
      routes: [activityTypesSettingsRoute],
    })
    app.filter('records.detailTabs', withActivitiesTab)
    app.referenceOptions('activity_type', ACTIVITY_TYPE_REFERENCE_OPTIONS)
    for (const [kind, renderer] of Object.entries(ACTIVITY_TIMELINE_RENDERERS)) app.timelineRenderer(kind, renderer)
  },
})

export default activitiesModule
