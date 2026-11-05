import { CheckSquare } from 'lucide-react'
import { registerRecordResource } from '@/sdk'
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
