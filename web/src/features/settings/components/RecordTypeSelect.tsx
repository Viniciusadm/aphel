import { NativeSelect } from '@/components/ui/native-select'
import { useNavigate } from 'react-router'
import { useRecordTypeOptions } from '../fields/lib/record-types'
import { useCustomObjectsAdmin } from '@/modules/objects/ui/api'

interface RecordTypeSelectProps {
  value: string
  label: string
  to: (recordType: string) => string
  extraTypes?: { value: string; label: string }[]
}

export function RecordTypeSelect({ value, label, to, extraTypes = [] }: RecordTypeSelectProps) {
  const navigate = useNavigate()
  const objects = useCustomObjectsAdmin()

  const core = useRecordTypeOptions()
  const custom = (objects.data ?? [])
    .filter((object) => (object.is_active && !object.is_internal) || object.record_type === value)
    .map((object) => ({ value: object.record_type, label: object.label_plural }))
  const known = [...core, ...extraTypes, ...custom].filter(
    (option, index, all) => all.findIndex((other) => other.value === option.value) === index,
  )
  const options = known.some((option) => option.value === value) ? known : [...known, { value, label: value }]

  return (
    <NativeSelect value={value} onValueChange={(next) => navigate(to(next))} aria-label={label} className="w-full sm:w-48">
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </NativeSelect>
  )
}
