import { Plug } from 'lucide-react'
import { SYSTEM_MANAGE } from '@/kernel/permissions'
import { defineModule } from '@/kernel/modules/define'

export default defineModule({
  key: 'wasm_runtime',
  register(app) {
    app.settings('integrations', {
      group: 'connections',
      path: 'wasm-runtime',
      label: 'settings:tabs.plugins',
      description: 'settings:items.plugins.description',
      icon: Plug,
      permission: SYSTEM_MANAGE,
      lazy: async () => ({ Component: (await import('@/features/settings/modules/PluginsPanel')).PluginsPanel }),
    })
  },
})
