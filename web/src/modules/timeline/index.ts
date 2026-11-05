import { defineModule } from '@/kernel/modules/define'
import { TIMELINE_MODULE_KEY, TIMELINE_RENDERERS, recordTimelineSource } from '@/sdk'

const timelineModule = defineModule({
  key: TIMELINE_MODULE_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.timelineSource(recordTimelineSource)
    for (const [kind, renderer] of Object.entries(TIMELINE_RENDERERS)) app.timelineRenderer(kind, renderer)
  },
})

export default timelineModule
