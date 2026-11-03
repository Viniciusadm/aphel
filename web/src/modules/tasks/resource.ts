import { CheckSquare } from 'lucide-react'
import { registerRecordResource } from '@/features/records/resources'

export const ACTIVITY_RECORD_TYPE = 'tasks.task'
export const ACTIVITY_TYPE_RECORD_TYPE = 'tasks.type'
export const TASKS_PATH = '/tasks'

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
