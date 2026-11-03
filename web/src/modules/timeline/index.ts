import { defineModule } from '@/kernel/modules/define'
import { TIMELINE_MODULE_KEY } from './api'
import { TIMELINE_RENDERERS } from './renderers'
import { recordTimelineSource } from './source'

export { TIMELINE_MODULE_KEY, timelineApi, type TimelineTarget } from './api'
export { TimelineLine, TimelineRecordLink } from './contents'
export { timelineRenderer } from './renderers'
export { recordTimelineSource, timelineKindOptions } from './source'
export { asObject, asText, eventType, nameOf, payloadOf, type TimelinePayload } from './text'

const timelineModule = defineModule({
  key: TIMELINE_MODULE_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.timelineSource(recordTimelineSource)
    for (const [kind, renderer] of Object.entries(TIMELINE_RENDERERS)) app.timelineRenderer(kind, renderer)
  },
})

export default timelineModule
