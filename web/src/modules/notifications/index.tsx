import { Bell } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { NotificationSettingsPage, NotificationBell } from './ui'
import { PushLifecycle } from './PushLifecycle'
const module = defineModule({ key: 'notifications', register(app) { app.settings('account', {
      group: 'profile',
      path: 'notifications',
      label: 'settings:tabs.notifications',
      description: 'settings:items.notifications.description',
      icon: Bell,
      lazy: async () => ({ Component: NotificationSettingsPage }),
    }); app.slot('app.topbar', { render: NotificationBell }); app.slot('app.shell', { render: PushLifecycle }) } })

export default module
