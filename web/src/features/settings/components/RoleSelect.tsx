import { NativeSelect } from '@/components/ui/native-select'
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
    <NativeSelect value={value} onValueChange={onChange} disabled={disabled} id={id} aria-invalid={invalid || undefined} aria-describedby={describedBy}><option value="" disabled>{placeholder}</option>
        {roles.map((role) => (
          <option key={role.id} value={role.id}>
            {role.name}
          </option>
        ))}
      </NativeSelect>
  )
}
