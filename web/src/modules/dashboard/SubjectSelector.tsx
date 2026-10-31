import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { useDashboardSubjects } from './api'
import type { DashboardSubject } from './types'
import { NativeSelect } from '@/components/ui/native-select'

const ALL = 'all'

function subjectValue(subject: DashboardSubject | null | undefined): string {
  return subject ? `${subject.type}:${subject.id}` : ALL
}

function parseSubject(value: string): DashboardSubject | null {
  const [type, ...rest] = value.split(':')
  const id = rest.join(':')
  if ((type === 'user' || type === 'team') && id) return { type, id }
  return null
}

export interface SubjectSelectorProps {
  value: DashboardSubject | null | undefined
  onChange: (subject: DashboardSubject | null) => void
}

export function SubjectSelector({ value, onChange }: SubjectSelectorProps) {
  const { t } = useTranslation('dashboard')
  const id = useId()
  const subjects = useDashboardSubjects()
  const data = subjects.data
  const selected = subjectValue(value)
  const known =
    selected === ALL ||
    Boolean(
      data &&
        (data.users.some((user) => `user:${user.id}` === selected) ||
          data.teams.some((team) => `team:${team.id}` === selected)),
    )

  return (
    <div className="grid w-full min-w-0 gap-2.5 sm:w-auto">
      <Label htmlFor={`${id}-subject`}>{t('subject.label')}</Label>
      <NativeSelect
        id={`${id}-subject`}
        value={known ? selected : ALL}
        disabled={subjects.isPending}
        onChange={(event) => onChange(parseSubject(event.target.value))}
      >
        <option value={ALL}>{t(`subject.all.${data?.scope ?? 'own'}`)}</option>
        {data && data.teams.length > 0 && (
          <optgroup label={t('subject.teams')}>
            {data.teams.map((team) => (
              <option key={team.id} value={`team:${team.id}`}>
                {team.name}
              </option>
            ))}
          </optgroup>
        )}
        {data && data.users.length > 0 && (
          <optgroup label={t('subject.users')}>
            {data.users.map((user) => (
              <option key={user.id} value={`user:${user.id}`}>
                {user.display_name || t('subject.unnamed')}
              </option>
            ))}
          </optgroup>
        )}
      </NativeSelect>
    </div>
  )
}

export interface CompareToggleProps {
  checked: boolean
  onChange: (checked: boolean) => void
}

export function CompareToggle({ checked, onChange }: CompareToggleProps) {
  const { t } = useTranslation('dashboard')
  const id = useId()
  return (
    <div className="flex h-9 items-center gap-2">
      <Checkbox id={id} checked={checked} onCheckedChange={(state) => onChange(state === true)} />
      <Label htmlFor={id} className="text-sm font-normal">
        {t('compare.label')}
      </Label>
    </div>
  )
}
