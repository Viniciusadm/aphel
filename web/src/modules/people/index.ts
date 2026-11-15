import { CONTACTS_PATH, CONTACT_RECORD_TYPE } from './resource'
import { PeopleDashboardEventLabel } from './dashboard/EventLabel'
import { createElement } from 'react'
import { PeopleSummary } from './dashboard/Summary'
import { BirthdaysWidget } from './dashboard/BirthdaysWidget'
import type { BirthdaysData } from './dashboard/birthdays-types'
import { UserPlus, Users } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ContactQuickForm } from './ContactQuickForm'
import { CreateContactDialog } from './create-action'
import { contactsRoutes } from './routes'

const contactsModule = defineModule({
  key: 'people',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.filter('dashboard.eventLabel', (label, { event }) => event.event_type.startsWith('contact.') ? createElement(PeopleDashboardEventLabel, { event }) : label)
    app.slot('dashboard.source', { key: 'people.summary', render: ({ collection, source, data }) => collection === 'summary' && source === 'people.summary' ? createElement(PeopleSummary, { data: data as Record<string, unknown> }) : null })
    app.widget('birthdays', {
      render: (data) => createElement(BirthdaysWidget, { data: data as BirthdaysData }),
      isEmpty: (data) => {
        const value = data as BirthdaysData
        return value.status === 'ok' && value.total === 0
      },
      settings: [{ kind: 'integer', key: 'limit', label: 'people:widgetSettings.birthdaysLimit', min: 5, max: 50 }],
    })
    app.routes(contactsRoutes)
    app.nav({ section: 'records', to: CONTACTS_PATH, label: 'nav.people', icon: Users, permission: ['people.read'], order: 20 })
    app.keybinding({ id: 'people.go', label: 'nav.people', to: CONTACTS_PATH, keys: ['g c'], permission: ['people.read'] })
    app.createAction({
      id: 'contact',
      label: 'createAction.label',
      icon: UserPlus,
      permission: 'people.create',
      recordType: CONTACT_RECORD_TYPE,
      order: 10,
      render: CreateContactDialog,
    })
    app.quickAdd({
      key: 'contact',
      label: 'quickAdd.options.contact',
      title: 'quickAdd.titles.contact',
      icon: UserPlus,
      permission: 'people.create',
      order: 50,
      linksRecord: false,
      render: ContactQuickForm,
    })
  },
})

export default contactsModule
