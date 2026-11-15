import { type RouteObject } from 'react-router'
import { DefaultRecordTypeRedirect } from '@/sdk'
export const VALIDATION_RULES_PATH = '/settings/validation-rules'

export const validationRulesSettingsRoute: RouteObject = {
  path: 'validation-rules',
  children: [
    { index: true, element: <DefaultRecordTypeRedirect /> },
    {
      path: ':recordType',
      lazy: async () => ({ Component: (await import('@/modules/validation_rules/ui/ValidationRulesPage')).ValidationRulesPage }),
    },
    {
      path: ':recordType/:ruleId',
      lazy: async () => ({ Component: (await import('@/modules/validation_rules/ui/ValidationRulePage')).ValidationRulePage }),
    },
  ],
}
