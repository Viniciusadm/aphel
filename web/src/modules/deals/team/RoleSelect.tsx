import { NativeSelect } from '@/components/ui/native-select'
import { useTranslation } from 'react-i18next'
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
    <NativeSelect value={value ?? NO_ROLE}
      disabled={disabled}
      onValueChange={(next) => onChange(next === NO_ROLE ? null : next)} id={id} aria-label={props['aria-label']} className={className}>
        <option value={NO_ROLE}>{t('noRole')}</option>
        {options.map((option) => (
          <option key={option.id} value={option.id}>
            {option.name}
          </option>
        ))}
      </NativeSelect>
  )
}
