import { useMemo } from 'react'
import type { RecordListExtension, RecordListExtensionProps, RecordListExtensionResult } from '@/kernel/modules/define'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { FieldDefinition } from '@/features/records/types'
import { isTaggable, useRecordTagsLookup, useTags, type Tag } from './api'
import { TagChipList } from './TagChip'

export const TAGS_FIELD_KEY = 'tags'
const TAGS_MODULE_KEY = 'tags'
const TAG_OPERATORS: FieldDefinition['operators'] = ['in', 'not_in', 'is_empty', 'is_not_empty']

function tagsField(recordType: string, tags: Tag[], label: string): FieldDefinition {
  return {
    id: `virtual-${TAGS_FIELD_KEY}`,
    key: TAGS_FIELD_KEY,
    label,
    record_type: recordType,
    field_type: 'multi_select',
    config: {},
    is_active: true,
    is_filterable: true,
    is_required: false,
    is_searchable: false,
    is_sortable: false,
    is_system: true,
    is_unique: false,
    is_personal: false,
    is_sensitive: false,
    access: 'editable',
    operators: TAG_OPERATORS,
    position: Number.MAX_SAFE_INTEGER,
    options: tags.map((tag, position) => ({
      id: tag.id,
      value: tag.id,
      label: tag.name,
      color: tag.color,
      is_active: true,
      position,
    })),
  }
}

function useTagsListExtension({ recordType, records, enabled }: RecordListExtensionProps): RecordListExtensionResult {
  const t = useModuleTranslation(TAGS_MODULE_KEY)
  const taggable = enabled && isTaggable(recordType)
  const tags = useTags(recordType, { enabled: taggable })
  const lookup = useRecordTagsLookup(
    recordType,
    taggable ? records.map((record) => record.id) : [],
    { enabled: taggable },
  )
  const label = t('list.field')
  const filterFields = useMemo(
    () => (taggable && tags.data && tags.data.length > 0 ? [tagsField(recordType, tags.data, label)] : undefined),
    [taggable, tags.data, recordType, label],
  )
  const assigned = lookup.data
  const titleAddon = useMemo(() => {
    if (!taggable || !assigned) return undefined
    return (record: { id: string }) => <TagChipList tags={assigned.get(record.id) ?? []} />
  }, [taggable, assigned])
  return useMemo(() => ({ filterFields, titleAddon }), [filterFields, titleAddon])
}

export const TAGS_LIST_EXTENSION: RecordListExtension = {
  key: 'tags',
  recordTypes: isTaggable,
  use: useTagsListExtension,
}
