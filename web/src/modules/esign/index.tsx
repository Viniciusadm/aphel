import { FileSignature } from 'lucide-react'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { ESIGN_KEY, SEND_PERMISSION, VIEW_PERMISSION } from './api'
import { EsignEventContent } from './EventContent'
import { EsignSettingsPanel } from './SettingsPanel'

const DealSignaturesTab = lazyComponent(async () => (await import('./DealSignaturesTab')).DealSignaturesTab)

const eventRenderer: TimelineRenderer = {
  icon: FileSignature,
  render: (item) => <EsignEventContent item={item} />,
}

const EVENTS = ['sent', 'signed', 'refused', 'cancelled', 'completed', 'expired', 'signed_file_attached']

const esignModule = defineModule({
  key: ESIGN_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: EsignSettingsPanel,
  register(app) {
    app.recordTab('deal', {
      key: `${ESIGN_KEY}.signatures`,
      label: 'tab',
      icon: FileSignature,
      permission: [VIEW_PERMISSION, SEND_PERMISSION],
      render: DealSignaturesTab,
    })
    for (const event of EVENTS) app.eventRenderer(`esign.${event}`, eventRenderer)
  },
})

export default esignModule
