import { Sparkles } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { AI_KEY, AI_PATH, AI_RECORD_TYPES, USE_PERMISSION, VIEW_USAGE_PERMISSION } from './api'
import { ComposerTool } from './ComposerTool'
import { QueryTool } from './QueryTool'
import { AiSettingsPanel } from './SettingsPanel'

const RecordAiTab = lazyComponent(async () => (await import('./RecordAiTab')).RecordAiTab)

const aiModule = defineModule({
  key: AI_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: AiSettingsPanel,
  register(app) {
    app.routes([
      {
        path: 'ai',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./AiPage')).AiPage }) }],
      },
    ])
    app.nav({
      to: AI_PATH,
      label: 'nav',
      icon: Sparkles,
      permission: [USE_PERMISSION, VIEW_USAGE_PERMISSION],
    })
    app.recordSection(AI_RECORD_TYPES, {
      key: `${AI_KEY}.assistant`,
      label: 'tab.label',
      icon: Sparkles,
      permission: [USE_PERMISSION],
      render: RecordAiTab,
    })
    app.slot('composer.tools', { key: 'compose', permission: [USE_PERMISSION], render: ComposerTool })
    app.slot('record.list.toolbar', { key: 'query', permission: [USE_PERMISSION], render: QueryTool })
  },
})

export default aiModule
