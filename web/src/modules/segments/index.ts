import { Layers } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { segmentFilterExtensions } from './filter'
import { SEGMENT_REFERENCE_OPTIONS } from './reference-options'
import { segmentsRoutes } from './routes'

export { SegmentDialog, type SegmentDialogProps } from './SegmentDialog'

const segmentsModule = defineModule({
  key: 'segments',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(segmentsRoutes)
    app.filter('records.filterFields', segmentFilterExtensions)
    app.referenceOptions('segment', SEGMENT_REFERENCE_OPTIONS)
    app.nav({
      to: '/segments',
      label: 'nav.segments',
      icon: Layers,
      permission: ['people.read', 'organizations.read', 'deals.read'],
      order: 120,
    })
  },
})

export default segmentsModule
