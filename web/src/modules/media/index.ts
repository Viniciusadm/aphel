import { Globe, Images } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { MEDIA_READ } from '@/sdk/media/api'

const mediaModule = defineModule({
  key: 'media',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([{ path: 'site/media', lazy: async () => ({ Component: (await import('./MediaLibraryPage')).MediaLibraryPage }) }])
    app.nav({
      to: '/site/media',
      label: 'nav.media',
      icon: Images,
      permission: MEDIA_READ,
      section: 'operations',
      group: { key: 'site', label: 'nav.site', icon: Globe },
      order: 40.5,
    })
  },
})

export default mediaModule
