import { useNavigate } from 'react-router'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { RECORD_TYPES, type KnownRecordType } from '../fields/lib/field-types'
import { useCustomObjectsAdmin } from '../objects/api'

interface RecordTypeSelectProps {
  value: string
  label: string
  coreLabel: (recordType: KnownRecordType) => string
  to: (recordType: string) => string
  extraTypes?: { value: string; label: string }[]
}

export function RecordTypeSelect({ value, label, coreLabel, to, extraTypes = [] }: RecordTypeSelectProps) {
  const navigate = useNavigate()
  const objects = useCustomObjectsAdmin()

  const core = RECORD_TYPES.map((recordType) => ({ value: recordType as string, label: coreLabel(recordType) }))
  const custom = (objects.data ?? [])
    .filter((object) => object.is_active || object.record_type === value)
    .map((object) => ({ value: object.record_type, label: object.label_plural }))
  const known = [...core, ...extraTypes, ...custom]
  const options = known.some((option) => option.value === value) ? known : [...known, { value, label: value }]

  return (
    <Select value={value} onValueChange={(next) => navigate(to(next))}>
      <SelectTrigger aria-label={label} className="w-full sm:w-48">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
