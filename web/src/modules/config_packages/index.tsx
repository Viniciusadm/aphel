import { PackageOpen } from 'lucide-react'
import type { ComponentType } from 'react'
import type { RouteObject } from 'react-router'
import { defineModule } from '@/kernel/modules/define'
import { SETTINGS_MANAGE } from '@/sdk'
function page<M, K extends keyof M>(load: () => Promise<M>, name: K): RouteObject['lazy'] { return async () => ({ Component: (await load())[name] as ComponentType }) }
const module = defineModule({ key: 'config_packages', register(app) { app.settings('data', {
      group: 'transfer',
      path: 'config-packages',
      label: 'settings:tabs.configPackages',
      description: 'settings:items.configPackages.description',
      icon: PackageOpen,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/modules/config_packages/ui/ConfigPackagesPage'), 'ConfigPackagesPage'),
    });  } })

export default module
