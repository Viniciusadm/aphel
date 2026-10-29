import { defineModule } from '@/kernel/modules/define'
import { FavoriteRecordButton, FavoriteViewButton } from './FavoriteButton'
import { FavoriteCommands } from './FavoriteCommands'
import { SidebarFavorites } from './SidebarFavorites'

const favoritesModule = defineModule({
  key: 'favorites',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.slot('app.sidebar', {
      key: 'favorites',
      render: ({ collapsed, onNavigate }) => <SidebarFavorites collapsed={collapsed} onNavigate={onNavigate} />,
    })
    app.slot('app.shell', { key: 'favorites-commands', render: FavoriteCommands })
    app.slot('record.detail.title', {
      key: 'favorite',
      render: ({ recordType, record }) => (
        <FavoriteRecordButton recordType={recordType} recordId={record.id} className="-my-1" />
      ),
    })
    app.slot('record.list.viewActions', {
      key: 'favorite',
      render: ({ recordType, viewId }) => <FavoriteViewButton recordType={recordType} viewId={viewId} />,
    })
  },
})

export default favoritesModule
