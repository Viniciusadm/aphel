import { CheckSquare } from 'lucide-react'
import i18n from '@/i18n'
import { registerRecordResource, type RecordItem } from '@/sdk'
import { ActivityListItem } from './components/ActivityListItem'
import { groupByDue } from './lib/due'
import { ACTIVITY_RECORD_TYPE, ACTIVITY_TYPE_RECORD_TYPE, TASKS_PATH } from '@/sdk'

export { ACTIVITY_RECORD_TYPE, ACTIVITY_TYPE_RECORD_TYPE, TASKS_PATH }

export const activityResource = registerRecordResource({
  icon: CheckSquare,
  recordType: ACTIVITY_RECORD_TYPE,
  createPermission: 'tasks.create',
  updatePermission: 'tasks.update',
  basePath: '/v1/tasks',
  titleField: 'title',
  detailPath: (id) => `${TASKS_PATH}/${encodeURIComponent(id)}`,
  listItem: ActivityListItem,
  listGroups: (records, { timezone }) => {
    const items = records as (RecordItem & { due_at?: string | null; status?: string })[]
    const closed = (status: string) => items.filter((item) => item.status === status)
    return [
      ...groupByDue(
        items.filter((item) => item.status !== 'done' && item.status !== 'canceled'),
        timezone,
      ).map((group) => ({ key: group.state, title: i18n.t(`mine.groups.${group.state}`, { ns: 'tasks' }), items: group.items })),
      { key: 'done', title: i18n.t('mine.groups.done', { ns: 'tasks' }), items: closed('done') },
      { key: 'canceled', title: i18n.t('mine.groups.canceled', { ns: 'tasks' }), items: closed('canceled') },
    ].filter((group) => group.items.length > 0)
  },
  defaultLayout: 'list',
})

export const activityTypeResource = registerRecordResource({
  recordType: ACTIVITY_TYPE_RECORD_TYPE,
  basePath: '/v1/activity-types',
  titleField: 'name',
})

export const TASK_DETAIL_FIELDS = [
  'title',
  'activity_type_id',
  'status',
  'priority',
  'due_at',
  'owner_id',
  'duration_minutes',
  'description',
  'completed_at',
  'created_by',
  'created_at',
]
