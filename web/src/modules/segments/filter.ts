import { type FilterFieldExtension, pseudoFilterField } from '@/sdk'
import i18n from '@/i18n'
import { isSegmentRecordType } from './api'
import { SegmentSummary, SegmentValueInput } from './SegmentFilterInputs'

export const SEGMENT_FILTER_FIELD = 'segment'

export function segmentFilterExtensions(
  extensions: FilterFieldExtension[],
  { recordType }: { recordType: string },
): FilterFieldExtension[] {
  if (!isSegmentRecordType(recordType)) return extensions
  return [
    ...extensions,
    {
      field: pseudoFilterField({
        recordType,
        key: SEGMENT_FILTER_FIELD,
        label: i18n.t('filter.field', { ns: 'segments' }),
        fieldType: 'select',
        operators: ['in_saved_filter'],
      }),
      operands: { in_saved_filter: 'single' },
      ValueInput: SegmentValueInput,
      Summary: SegmentSummary,
    },
  ]
}
