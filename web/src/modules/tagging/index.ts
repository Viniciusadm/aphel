import { Tag } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { TAGS_LIST_EXTENSION } from './list-extension'
import { RecordTagsAction } from './RecordTagsAction'
import { TAG_REFERENCE_OPTIONS } from './reference-options'
import { tagsSettingsRoute } from './routes'

export {
  DEFAULT_TAG_COLOR,
  useTaggingEntities,
  useTaggingEntity,
  isTaggable,
  tagUpdatePermission,
  tagsApi,
  tagsKeys,
  useRecordTags,
  useRecordTagsLookup,
  useTags,
  type Tag,
  type TagSummary,
  type TaggableRecordType,
} from './api'
export { RecordTagsControl, type RecordTagsControlProps } from './RecordTagsControl'
export { TagChip, TagChipList, type TagChipListProps, type TagChipProps } from './TagChip'

const tagsModule = defineModule({
  key: 'tagging',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'records',
      path: 'tagging',
      label: 'settings:tabs.tags',
      description: 'settings:items.tags.description',
      icon: Tag,
      permission: SETTINGS_MANAGE,
      routes: [tagsSettingsRoute],
    })
    app.slot('record.detail.actions', { key: 'tagging', render: RecordTagsAction })
    app.recordList(TAGS_LIST_EXTENSION)
    app.referenceOptions('tag', TAG_REFERENCE_OPTIONS)
  },
})

export default tagsModule
