import { Sparkles } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { AI_KEY, AI_RECORD_TYPES, USE_PERMISSION } from './api'
import { ComposerTool } from './ComposerTool'
import { QueryTool } from './QueryTool'
import { AiSettingsPanel } from './SettingsPanel'

const RecordAiTab = lazyComponent(async () => (await import('./RecordAiTab')).RecordAiTab)

import { AiCard } from './IntegrationCard'

const aiModule = defineModule({
  key: AI_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  settingsPanel: AiSettingsPanel,
  register(app) {
    app.keybinding({ id: 'ai.compose_run', label: 'keybindings.composeRun', scope: 'editor', keys: ['mod+enter'] })
    app.slot('integrations.cards', { key: 'assistant', order: 200, permission: [USE_PERMISSION], render: AiCard })
    app.settings('integrations', {
      group: 'connections',
      path: 'ai',
      label: 'nav',
      description: 'page.menuDescription',
      icon: Sparkles,
      permission: USE_PERMISSION,
      lazy: async () => ({ Component: (await import('./AiPage')).AiPage }),
    })
    app.recordSection(AI_RECORD_TYPES, {
      key: `${AI_KEY}.assistant`,
      group: 'insights',
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
