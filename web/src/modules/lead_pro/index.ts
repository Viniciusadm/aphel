import { Flame } from 'lucide-react'
import { NAV_ORDER, defineModule } from '@/kernel/modules/define'
import { LEAD_PRO_KEY, LEAD_PRO_PATH, READ_PERMISSION, TEMPERATURE_FIELD } from './api'
import { TemperatureTab } from './TemperatureTab'
import { TemperatureDisplay, TemperatureInput } from './TemperatureField'
import { TouchButton } from './TouchButton'

const leadProModule = defineModule({
  key: LEAD_PRO_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: {
    notifications: {
      'pt-BR': {
        kinds: {
          lead_pro_hot: {
            label: 'Lead esquentou',
            description: 'Quando um contato seu passa da temperatura de lead quente do Lead Pro.',
          },
        },
      },
      en: {
        kinds: {
          lead_pro_hot: {
            label: 'Lead got hot',
            description: 'When one of your contacts reaches the hot lead temperature of Lead Pro.',
          },
        },
      },
    },
  },
  register(app) {
    app.routes([
      {
        path: 'lead-pro',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./BoardPage')).BoardPage }) }],
      },
    ])
    app.nav({
      to: LEAD_PRO_PATH,
      label: 'nav',
      icon: Flame,
      permission: READ_PERMISSION,
      order: NAV_ORDER.modules,
    })
    app.recordTab('contact', {
      key: `${LEAD_PRO_KEY}.temperature`,
      label: 'tab',
      icon: Flame,
      permission: READ_PERMISSION,
      order: 25,
      render: TemperatureTab,
    })
    app.slot('record.detail.actions', {
      key: `${LEAD_PRO_KEY}.touch`,
      recordTypes: ['contact'],
      permission: READ_PERMISSION,
      render: TouchButton,
    })
    app.fieldType(`number:${TEMPERATURE_FIELD}`, { Input: TemperatureInput, Display: TemperatureDisplay })
    app.command({
      id: `${LEAD_PRO_KEY}.board`,
      label: 'board.title',
      icon: Flame,
      permission: READ_PERMISSION,
      to: LEAD_PRO_PATH,
    })
  },
})

export default leadProModule
