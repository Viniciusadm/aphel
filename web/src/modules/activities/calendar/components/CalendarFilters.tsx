import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { useActivityTypes } from '@/modules/activities/api'
import { useOrganizationUsers } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useViewTeams } from '@/features/records/views/viewsApi'

export interface CalendarFiltersProps {
  ownerId: string
  teamId: string
  typeId: string
  onChange: (patch: { ownerId?: string; teamId?: string; typeId?: string }) => void
}

export function CalendarFilters({ ownerId, teamId, typeId, onChange }: CalendarFiltersProps) {
  const { t } = useTranslation('activities', { keyPrefix: 'calendar' })
  const { me } = useSession()
  const users = useOrganizationUsers()
  const teams = useViewTeams(true)
  const types = useActivityTypes()
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      <NativeSelect
        aria-label={t('filters.owner')}
        className="w-full sm:w-48"
        value={ownerId}
        onChange={(event) => onChange({ ownerId: event.target.value })}
      >
        <option value="">{t('filters.allOwners')}</option>
        {(users.data ?? []).map((user) => (
          <option key={user.id} value={user.id}>
            {user.id === me.user.id ? t('filters.me', { name: user.display_name }) : user.display_name}
          </option>
        ))}
      </NativeSelect>
      <NativeSelect
        aria-label={t('filters.team')}
        className="w-full sm:w-44"
        value={teamId}
        onChange={(event) => onChange({ teamId: event.target.value })}
      >
        <option value="">{t('filters.allTeams')}</option>
        {(teams.data ?? []).map((team) => (
          <option key={team.id} value={team.id}>
            {team.name}
          </option>
        ))}
      </NativeSelect>
      <NativeSelect
        aria-label={t('filters.type')}
        className="w-full sm:w-44"
        value={typeId}
        onChange={(event) => onChange({ typeId: event.target.value })}
      >
        <option value="">{t('filters.allTypes')}</option>
        {(types.data ?? []).map((type) => (
          <option key={type.id} value={type.id}>
            {type.name}
          </option>
        ))}
      </NativeSelect>
    </div>
  )
}
