import { defineModule } from '@/kernel/modules/define'
import { DemoBanner } from './DemoBanner'

const demoModule = defineModule({
  key: 'demo',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.slot('app.banner', { key: 'demo', render: DemoBanner })
  },
})

export default demoModule
