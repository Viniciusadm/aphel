import { CatalogSourcePanel } from './CatalogSourcePanel'
import { Store } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'

export default defineModule({
  key: 'marketplace',
  register(app) {
    app.slot('modules.settings', { render: CatalogSourcePanel })
    app.settings('data', { group: 'transfer', path: 'catalog', label: 'settings:tabs.catalog', description: 'settings:items.catalog.description', icon: Store, permission: 'settings.manage', lazy: async () => ({ Component: (await import('./CatalogPage')).CatalogPage }) })
  },
})
