import { LayoutTemplate } from 'lucide-react'
import type { ComponentType } from 'react'
import type { RouteObject } from 'react-router'
import { defineModule } from '@/kernel/modules/define'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { Navigate } from 'react-router'
import { DEFAULT_RECORD_TYPE } from '@/features/settings/fields/lib/field-types'
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
