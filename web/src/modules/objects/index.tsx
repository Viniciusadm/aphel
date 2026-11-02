import { Boxes } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { objectsSettingsRoutes } from './ui/routes'
const module = defineModule({ key: 'objects', register(app) { app.settings('organization', {
      group: 'records',
      path: 'objects',
      label: 'settings:tabs.objects',
      description: 'settings:items.objects.description',
      icon: Boxes,
      permission: SETTINGS_MANAGE,
      routes: objectsSettingsRoutes,
    });  } })

export default module
