import { RequirePermission } from '@/sdk'
import { OutcomesPage } from './OutcomesPage'

export function CallOutcomesSettingsPage() {
  return (
    <RequirePermission permission="settings.manage">
      <OutcomesPage />
    </RequirePermission>
  )
}
