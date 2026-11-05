import { CONTACTS_PATH, CONTACT_RECORD_TYPE } from './resource'
import { PeopleDashboardEventLabel } from './dashboard/EventLabel'
import { createElement } from 'react'
import { PeopleSummary } from './dashboard/Summary'
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
    app.routes(contactsRoutes)
    app.nav({ section: 'records', shortcut: 'c', to: CONTACTS_PATH, label: 'nav.people', icon: Users, permission: ['people.read'], order: 20 })
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
