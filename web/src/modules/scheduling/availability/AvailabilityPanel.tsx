import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { PageLoader } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useOrganizationUsers } from '@/features/records/api'
import { useAvailability } from '../api'
import { AvailabilityForm } from './AvailabilityForm'

const SELF = 'self'

export const SCHEDULING_PERMISSION = 'scheduling.manage'

function UserPicker({ value, onChange }: { value: string | null; onChange: (userId: string | null) => void }) {
  const { t } = useTranslation('scheduling')
  const { me } = useSession()
  const users = useOrganizationUsers()
  const others = (users.data ?? []).filter((user) => user.id !== me.user.id)
  if (others.length === 0) return null
  return (
    <div className="grid max-w-xs gap-2">
      <Label htmlFor="availability-user">{t('availability.user')}</Label>
      <Select value={value ?? SELF} onValueChange={(next) => onChange(next === SELF ? null : next)}>
        <SelectTrigger id="availability-user">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={SELF}>{t('availability.me')}</SelectItem>
          {others.map((user) => (
            <SelectItem key={user.id} value={user.id}>
              {user.display_name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

export function AvailabilityPanel() {
  const { t } = useTranslation('scheduling')
  const { permissionScope } = useSession()
  const canManageOthers = (permissionScope(SCHEDULING_PERMISSION) ?? 'own') !== 'own'
  const [userId, setUserId] = useState<string | null>(null)
  const availability = useAvailability(userId)

  return (
    <div className="grid gap-4">
      {canManageOthers && <UserPicker value={userId} onChange={setUserId} />}
      {availability.isPending ? (
        <PageLoader />
      ) : availability.isError ? (
        <ErrorState
          description={
            isApiError(availability.error) && availability.error.status === 403
              ? t('availability.forbidden')
              : undefined
          }
          onRetry={() => void availability.refetch()}
        />
      ) : (
        <AvailabilityForm
          key={`${userId ?? SELF}:${availability.data.updated_at ?? 'new'}`}
          userId={userId}
          availability={availability.data}
        />
      )}
    </div>
  )
}
