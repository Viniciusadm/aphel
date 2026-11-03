import { UserPlus, Users } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ContactQuickForm } from './ContactQuickForm'
import { CreateContactDialog } from './create-action'
import { CONTACTS_PATH, CONTACT_RECORD_TYPE } from './resource'
import { contactsRoutes } from './routes'

const contactsModule = defineModule({
  key: 'people',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(contactsRoutes)
    app.nav({ to: CONTACTS_PATH, label: 'nav.people', icon: Users, permission: ['people.read'], order: 20 })
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
