import { Mail, MailCheck } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { CUSTOM_RECORD_TYPE_PREFIX } from '@/features/custom-records/resources'
import { CORE_EMAIL_RECORD_TYPES } from './lib/record-types'
import { EMAILS_RECORD_TAB, withEmailsTabPlacement } from './record-tab'
import { emailSettingsRoute, mailboxesSettingsRoute } from './routes'
import { EMAIL_TIMELINE_RENDERERS } from './timeline/renderers'

export { placeTabAfter } from './record-tab'
export { EmailsTab } from './record-tab'
export type { EmailsTabProps } from './record/EmailsTab'
export {
  EMAILS_READ_PERMISSION,
  EMAILS_SEND_PERMISSION,
  EMAILS_TAB_KEY,
  EMAIL_EVENT_TYPES,
  MAILBOXES_PATH,
} from './constants'
export { emailApi, emailKeys } from './api'
export type * from './types'

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
