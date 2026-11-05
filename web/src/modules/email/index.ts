import { Mail, MailCheck } from 'lucide-react'
import { CUSTOM_RECORD_TYPE_PREFIX, SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import {CORE_EMAIL_RECORD_TYPES} from '@/sdk'
import {EMAILS_RECORD_TAB, withEmailsTabPlacement} from '@/sdk'
import { emailSettingsRoute, mailboxesSettingsRoute } from './routes'
import { EMAIL_TIMELINE_RENDERERS } from './timeline/renderers'


const emailModule = defineModule({
  key: 'email',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('account', {
      group: 'communication',
      path: 'mailboxes',
      label: 'settings:tabs.mailboxes',
      description: 'settings:items.mailboxes.description',
      icon: Mail,
      routes: [mailboxesSettingsRoute],
    })
    app.settings('organization', {
      group: 'communication',
      path: 'email',
      label: 'settings:tabs.email',
      description: 'settings:items.email.description',
      icon: MailCheck,
      permission: SETTINGS_MANAGE,
      routes: [emailSettingsRoute],
    })
    app.recordSection([...CORE_EMAIL_RECORD_TYPES, `${CUSTOM_RECORD_TYPE_PREFIX}*`], EMAILS_RECORD_TAB)
    app.filter('records.detailTabs', withEmailsTabPlacement)
    for (const [eventType, renderer] of Object.entries(EMAIL_TIMELINE_RENDERERS)) app.eventRenderer(eventType, renderer)
  },
})

export default emailModule
