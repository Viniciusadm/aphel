import { categoryLabel } from '../lib/events'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { NativeSelect, useOrganizationUsers, useTeams } from '@/sdk'
import type { CalendarRecord } from '../api'

export interface CalendarFiltersProps {
  ownerId: string
  teamId: string
  typeId: string
  sourceId: string
  categories: CalendarRecord['category'][]
  onChange: (patch: { ownerId?: string; teamId?: string; typeId?: string; sourceId?: string }) => void
}

export function CalendarFilters({ ownerId, teamId, typeId, sourceId, onChange, categories }: CalendarFiltersProps) {
  const { t } = useTranslation('calendar')
  const { me, hasPermission } = useSession()
  const users = useOrganizationUsers()
  const canManageTeams = hasPermission('settings.manage')
  const teams = useTeams(canManageTeams)
  const teamOptions = canManageTeams ? teams.data ?? [] : me.teams
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
        {(teamOptions ?? []).map((team) => (
          <option key={team.id} value={team.id}>
            {team.name}
          </option>
        ))}
      </NativeSelect>
      <NativeSelect
        aria-label={t('filters.type')}
        className="w-full sm:w-44"
        value={typeId ? JSON.stringify([sourceId, typeId]) : ''}
        onChange={(event) => { const selected = categories.find((category) => JSON.stringify([category.source, category.id]) === event.target.value); onChange({typeId: selected?.id ?? '', sourceId: selected?.source ?? ''}) }}
      >
        <option value="">{t('filters.allTypes')}</option>
        {categories.map((type) => (
          <option key={JSON.stringify([type.source,type.id])} value={JSON.stringify([type.source,type.id])}>
            {categoryLabel(t, type)}
          </option>
        ))}
      </NativeSelect>
    </div>
  )
}
