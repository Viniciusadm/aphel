import { NativeSelect } from '@/components/ui/native-select'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { PageLoader } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { useOrganizationUsers } from '@/features/records/api'
import { useAvailability } from '../api'
import { AvailabilityForm } from './AvailabilityForm'
import { Field } from '@/components/ui/field'

const SELF = 'self'

export const SCHEDULING_PERMISSION = 'scheduling.manage'

function UserPicker({ value, onChange }: { value: string | null; onChange: (userId: string | null) => void }) {
  const { t } = useTranslation('scheduling')
  const { me } = useSession()
  const users = useOrganizationUsers()
  const others = (users.data ?? []).filter((user) => user.id !== me.user.id)
  if (others.length === 0) return null
  return (
    <Field label={t('availability.user')} className="max-w-xs">
      {(fieldControl) => <NativeSelect value={value ?? SELF} onValueChange={(next) => onChange(next === SELF ? null : next)} id={fieldControl.id}>
          <option value={SELF}>{t('availability.me')}</option>
          {others.map((user) => (
            <option key={user.id} value={user.id}>
              {user.display_name}
            </option>
          ))}
        </NativeSelect>}
    </Field>
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
