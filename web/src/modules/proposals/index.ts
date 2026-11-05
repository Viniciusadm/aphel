import { FileText } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import {GENERATE_PERMISSION, MANAGE_PERMISSION, PROPOSALS_KEY, PROPOSALS_PATH} from '@/sdk'

const DealProposalsTab = lazyComponent(async () => (await import('./DealProposalsTab')).DealProposalsTab)

const proposalsModule = defineModule({
  key: PROPOSALS_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([
      {
        path: 'proposals',
        children: [
          { index: true, lazy: async () => ({ Component: (await import('./ProposalsPage')).ProposalsPage }) },
          { path: 'new', lazy: async () => ({ Component: (await import('./TemplateEditorPage')).TemplateEditorPage }) },
          {
            path: ':templateId',
            lazy: async () => ({ Component: (await import('./TemplateEditorPage')).TemplateEditorPage }),
          },
        ],
      },
    ])
    app.nav({ to: PROPOSALS_PATH, label: 'nav', icon: FileText, permission: [MANAGE_PERMISSION] })
    app.recordSection('deal', {
      key: `${PROPOSALS_KEY}.documents`,
      label: 'tab',
      icon: FileText,
      permission: [GENERATE_PERMISSION],
      render: DealProposalsTab,
    })
  },
})

export default proposalsModule
