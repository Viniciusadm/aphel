import { BellRing } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { FollowRecordButton } from './FollowRecordButton'
import { followingRoutes } from './routes'

const followersModule = defineModule({
  key: 'followers',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(followingRoutes)
    app.nav({ section: 'work', to: '/following', label: 'followers:nav.following', icon: BellRing, order: 100 })
    app.slot('record.detail.actions', {
      key: 'follow',
      order: 10,
      render: ({ recordType, record }) => <FollowRecordButton recordType={recordType} recordId={record.id} />,
    })
  },
})

export default followersModule
