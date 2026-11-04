import { useId, useMemo, type ComponentProps } from 'react'
import { Input } from '@/components/ui/input'
import { availableTimeZones } from './format'

export type TimeZoneInputProps = Omit<ComponentProps<typeof Input>, 'list' | 'type'>

export function TimeZoneInput(props: TimeZoneInputProps) {
  const listId = useId()
  const zones = useMemo(() => availableTimeZones(), [])
  return (
    <>
      <Input
        {...props}
        list={zones.length > 0 ? listId : undefined}
        autoComplete="off"
        spellCheck={false}
      />
      {zones.length > 0 && (
        <datalist id={listId}>
          {zones.map((zone) => (
            <option key={zone} value={zone} />
          ))}
        </datalist>
      )}
    </>
  )
}
