import { LayoutTemplate } from 'lucide-react'
import type { ComponentType } from 'react'
import type { RouteObject } from 'react-router'
import { defineModule } from '@/kernel/modules/define'
import { DEFAULT_RECORD_TYPE, SETTINGS_MANAGE } from '@/sdk'
import { Navigate } from 'react-router'
function page<M, K extends keyof M>(load: () => Promise<M>, name: K): RouteObject['lazy'] { return async () => ({ Component: (await load())[name] as ComponentType }) }
const module = defineModule({ key: 'record_layouts', register(app) { app.settings('organization', {
      group: 'records',
      path: 'layouts',
      label: 'settings:tabs.layouts',
      description: 'settings:items.layouts.description',
      icon: LayoutTemplate,
      permission: SETTINGS_MANAGE,
      routes: [
        {
          path: 'layouts',
          element: <Navigate to={DEFAULT_RECORD_TYPE} replace />,
        },
        {
          path: 'layouts/:recordType',
          lazy: page(() => import('@/modules/record_layouts/ui/LayoutsPage'), 'LayoutsPage'),
        },
      ],
    });  } })

export default module
