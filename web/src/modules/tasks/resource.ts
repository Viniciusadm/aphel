import { CheckSquare } from 'lucide-react'
import i18n from '@/i18n'
import { registerRecordResource, type RecordItem } from '@/sdk'
import { ActivityEditDialog } from './components/ActivityEditDialog'
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
  editDialog: ActivityEditDialog,
  listSorts: () => [
    { key: 'due', label: i18n.t('sort.due', { ns: 'tasks' }), sort: [{ field: 'due_at', direction: 'asc' }] },
    {
      key: 'priority',
      label: i18n.t('sort.priority', { ns: 'tasks' }),
      sort: [
        { field: 'priority', direction: 'desc' },
        { field: 'due_at', direction: 'asc' },
      ],
    },
  ],
  listGroups: (records, { timezone, sort }) => {
    if (sort[0] && sort[0].field !== 'due_at') return [{ key: 'all', title: '', items: records }]
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
  calendarLink: { module: 'calendar', to: '/calendar' },
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
