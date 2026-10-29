import { useTranslation } from 'react-i18next'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { DealMemberRole, DealMemberRoleSummary } from './types'

const NO_ROLE = '__none'

export interface RoleSelectProps {
  id?: string
  value: string | null
  roles: DealMemberRole[]
  current?: DealMemberRoleSummary | null
  disabled?: boolean
  onChange: (roleId: string | null) => void
  'aria-label'?: string
  className?: string
}

export function RoleSelect({ id, value, roles, current, disabled, onChange, className, ...props }: RoleSelectProps) {
  const { t } = useTranslation('deals', { keyPrefix: 'team' })
  const options = roles.map((role) => ({ id: role.id, name: role.name }))
  if (current && !options.some((option) => option.id === current.id)) {
    options.push({ id: current.id, name: current.name })
  }
  return (
    <Select
      value={value ?? NO_ROLE}
      disabled={disabled}
      onValueChange={(next) => onChange(next === NO_ROLE ? null : next)}
    >
      <SelectTrigger id={id} aria-label={props['aria-label']} className={className}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={NO_ROLE}>{t('noRole')}</SelectItem>
        {options.map((option) => (
          <SelectItem key={option.id} value={option.id}>
            {option.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
