import { Navigate, type RouteObject } from 'react-router'
import { DEFAULT_RECORD_TYPE } from '@/sdk'
export const VALIDATION_RULES_PATH = '/settings/validation-rules'

export const validationRulesSettingsRoute: RouteObject = {
  path: 'validation-rules',
  children: [
    { index: true, element: <Navigate to={DEFAULT_RECORD_TYPE} replace /> },
    {
      path: ':recordType',
      lazy: async () => ({ Component: (await import('@/modules/validation_rules/ui/ValidationRulesPage')).ValidationRulesPage }),
    },
  ],
}
