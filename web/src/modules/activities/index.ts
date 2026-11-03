import { CalendarCheck, CalendarDays, CheckSquare, Inbox, MapPin } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { calendarRoutes } from './calendar/routes'
import { CreateTaskDialog } from './create-action'
import { inboxRoutes } from './inbox/routes'
import { ActivityQuickForm } from './mobile/ActivityQuickForm'
import { CheckInQuickForm } from './mobile/CheckInQuickForm'
import { withActivitiesTab } from './record-tabs'
import { ACTIVITY_TYPE_REFERENCE_OPTIONS } from './reference-options'
import { ACTIVITY_RECORD_TYPE } from './resource'
import { activitiesRoutes, activityTypesSettingsRoute } from './routes'
import { ACTIVITY_TIMELINE_RENDERERS } from './timeline/renderers'

const ACTIVITIES_READ = ['activities.read']

const activitiesModule = defineModule({
  key: 'activities',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([...activitiesRoutes, ...calendarRoutes, ...inboxRoutes])
    app.nav({ to: '/tasks', label: 'nav.tasks', icon: CheckSquare, permission: ACTIVITIES_READ, order: 80 })
    app.nav({ to: '/inbox', label: 'nav.inbox', icon: Inbox, permission: ACTIVITIES_READ, order: 90 })
    app.nav({ to: '/calendar', label: 'nav.calendar', icon: CalendarDays, permission: ACTIVITIES_READ, order: 110 })
    app.createAction({
      id: 'task',
      label: 'createAction.label',
      linkedLabel: 'createAction.linked',
      icon: CheckSquare,
      permission: 'activities.create',
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
      permission: 'activities.create',
      order: 10,
      render: CheckInQuickForm,
    })
    app.quickAdd({
      key: 'activity',
      label: 'quickAdd.options.activity',
      title: 'quickAdd.titles.activity',
      icon: CalendarCheck,
      permission: 'activities.create',
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
