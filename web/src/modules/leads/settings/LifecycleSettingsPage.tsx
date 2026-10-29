import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { LIFECYCLE_MANAGE_PERMISSION } from '../constants'
import { DisqualificationReasonsSection } from './DisqualificationReasonsSection'
import { TransitionRulesSection } from './TransitionRulesSection'

export function LifecycleSettingsPage() {
  return (
    <RequirePermission permission={LIFECYCLE_MANAGE_PERMISSION}>
      <div className="flex flex-col gap-8">
        <TransitionRulesSection />
        <DisqualificationReasonsSection />
      </div>
    </RequirePermission>
  )
}
