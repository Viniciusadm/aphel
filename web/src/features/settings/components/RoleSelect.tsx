import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { Role } from '../api'

interface RoleSelectProps {
  id: string
  roles: Role[]
  value: string
  placeholder: string
  invalid?: boolean
  describedBy?: string
  disabled?: boolean
  onChange: (value: string) => void
}

export function RoleSelect({ id, roles, value, placeholder, invalid, describedBy, disabled, onChange }: RoleSelectProps) {
  return (
    <Select value={value} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger id={id} aria-invalid={invalid || undefined} aria-describedby={describedBy}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {roles.map((role) => (
          <SelectItem key={role.id} value={role.id}>
            {role.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
