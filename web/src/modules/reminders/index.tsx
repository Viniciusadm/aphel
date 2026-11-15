import { AlarmClock } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { getRecordResource, recordTitle } from '@/sdk'
import { RemindRecordButton } from './RemindRecordButton'
import { reminderRoutes } from './routes'

const remindersModule = defineModule({
  key: 'reminders',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(reminderRoutes)
    app.nav({ section: 'work', to: '/reminders', label: 'reminders:nav.reminders', icon: AlarmClock, order: 110 })
    app.slot('record.detail.actions', {
      key: 'remind',
      order: 20,
      render: ({ recordType, record }) => {
        const resource = getRecordResource(recordType)
        return <RemindRecordButton recordType={recordType} recordId={record.id} title={resource ? recordTitle(resource, record) : null} />
      },
    })
  },
})

export default remindersModule
