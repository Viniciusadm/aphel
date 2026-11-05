import { ListChecks } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { SETTINGS_MANAGE } from '@/sdk'
import { validationRulesSettingsRoute } from './ui/routes'
const module = defineModule({ key: 'validation_rules', register(app) { app.settings('organization', {
      group: 'records',
      path: 'validation-rules',
      label: 'settings:tabs.validationRules',
      description: 'settings:items.validationRules.description',
      icon: ListChecks,
      permission: SETTINGS_MANAGE,
      routes: [validationRulesSettingsRoute],
    });  } })

export default module
