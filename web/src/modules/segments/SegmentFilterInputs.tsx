import { useTranslation } from 'react-i18next'
import { type FilterExtensionSummaryProps, type FilterExtensionValueProps, NativeSelect } from '@/sdk'
import {useSegmentOptions} from '@/sdk'

export function SegmentValueInput({ recordType, value, onChange, label }: FilterExtensionValueProps) {
  const { t } = useTranslation('segments')
  const segments = useSegmentOptions(recordType)
  const selected = typeof value === 'string' ? value : ''
  const options = segments.data ?? []
  return (
    <NativeSelect
      aria-label={label}
      className="w-full sm:w-56"
      value={selected}
      onChange={(event) => onChange(event.target.value || null)}
    >
      <option value="">{segments.isPending ? t('filter.loading') : t('filter.choose')}</option>
      {options.map((segment) => (
        <option key={segment.id} value={segment.id}>
          {segment.name}
        </option>
      ))}
      {selected && !segments.isPending && !options.some((segment) => segment.id === selected) && (
        <option value={selected}>{t('filter.unknown')}</option>
      )}
    </NativeSelect>
  )
}

export function SegmentSummary({ recordType, condition, label }: FilterExtensionSummaryProps) {
  const { t } = useTranslation('segments')
  const segments = useSegmentOptions(recordType)
  const name = segments.data?.find((segment) => segment.id === condition.value)?.name
  return <>{t('filter.summary', { field: label, name: name ?? t('filter.unknown') })}</>
}
