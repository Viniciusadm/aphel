import { Phone, PhoneCall } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { CallQuickForm } from './CallQuickForm'
import { ActivityCallPanel } from './components/ActivityCallPanel'
import { PhoneLink } from './components/PhoneLink'
import { callsRoutes } from './routes'
import { callAwareActivityRenderer } from './timeline/renderers'

export default defineModule({
  key: 'calls',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(callsRoutes)
    app.nav({ to: '/calls', label: 'nav.calls', icon: Phone, permission: 'activities.read', order: 180 })
    app.fieldType('phone', { Display: PhoneLink })
    app.timelineRenderer('activity', callAwareActivityRenderer)
    app.slot('record.detail.header', { key: 'call-summary', recordTypes: ['activity'], render: ActivityCallPanel })
    app.quickAdd({
      key: 'call',
      label: 'quickAdd.options.call',
      title: 'quickAdd.titles.call',
      icon: PhoneCall,
      permission: 'activities.create',
      order: 40,
      render: CallQuickForm,
    })
  },
})
