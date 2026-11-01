import { ListOrdered } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { SEQUENCES_MANAGE_PERMISSION, SEQUENCES_PATH, SEQUENCES_READ_PERMISSION } from './constants'
import { SEQUENCES_RECORD_TAB, withSequencesTabPlacement } from './record-tab'
import { SEQUENCE_REFERENCE_OPTIONS } from './reference-options'
import { sequencesRoutes } from './routes'

export { SequencesTab } from './record/SequencesTab'
export type { SequencesTabProps } from './record/SequencesTab'
export { sequencesApi, sequencesKeys, useActiveSequences } from './api'
export {
  SEQUENCES_ENROLL_PERMISSION,
  SEQUENCES_MANAGE_PERMISSION,
  SEQUENCES_PATH,
  SEQUENCES_READ_PERMISSION,
  SEQUENCES_TAB_KEY,
} from './constants'
export type * from './types'

const SEQUENCES_PLACEMENT_PRIORITY = 20

const sequencesModule = defineModule({
  key: 'sequences',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(sequencesRoutes)
    app.nav({
      to: SEQUENCES_PATH,
      label: 'nav.sequences',
      icon: ListOrdered,
      permission: [SEQUENCES_READ_PERMISSION, SEQUENCES_MANAGE_PERMISSION],
      order: 160,
    })
    app.recordSection('contact', SEQUENCES_RECORD_TAB)
    app.filter('records.detailTabs', withSequencesTabPlacement, SEQUENCES_PLACEMENT_PRIORITY)
    app.referenceOptions('sequence', SEQUENCE_REFERENCE_OPTIONS)
  },
})

export default sequencesModule
