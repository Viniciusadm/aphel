import { useTranslation } from 'react-i18next'
import { Skeleton } from '@/components/ui/skeleton'
import { useTeams } from '@/features/settings/api'
import { CheckboxList } from './CheckboxList'

export interface TeamsFieldProps {
  selected: string[]
  disabled?: boolean
  onChange: (selected: string[]) => void
}

export function TeamsField({ selected, disabled, onChange }: TeamsFieldProps) {
  const { t } = useTranslation('pipelines')
  const teams = useTeams()

  if (teams.isPending) return <Skeleton className="h-16 w-full" />
  if (teams.isError) return <p className="text-sm text-destructive">{t('teams.loadError')}</p>

  return (
    <CheckboxList
      legend={t('teams.legend')}
      description={t('teams.description')}
      items={teams.data.map((team) => ({ value: team.id, label: team.name }))}
      selected={selected}
      emptyText={t('teams.empty')}
      disabled={disabled}
      onChange={onChange}
    />
  )
}
