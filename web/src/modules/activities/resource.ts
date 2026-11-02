import { registerRecordResource } from '@/features/records/resources'

export const ACTIVITY_RECORD_TYPE = 'activity'
export const ACTIVITY_TYPE_RECORD_TYPE = 'activity_type'
export const TASKS_PATH = '/tasks'

export const activityResource = registerRecordResource({
  recordType: ACTIVITY_RECORD_TYPE,
  createPermission: 'activities.create',
  updatePermission: 'activities.update',
  basePath: '/v1/activities',
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
