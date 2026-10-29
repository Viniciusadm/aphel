export {
  customRecordsApi,
  customRecordsKeys,
  requiredRelations,
  useCustomObjectLabel,
  useCustomObjects,
  useObjectSchema,
  useRecordRelations,
  useRelationships,
  type ObjectSchema,
  type RelationInfo,
  type Relationship,
  type RelationshipCardinality,
  type RelationshipOnDelete,
} from './api'
export { CustomObjectsNav, type CustomObjectsNavProps } from './CustomObjectsNav'
export { notifyObjectsError, objectsErrorMessage } from './errors'
export {
  DEFAULT_OBJECT_COLOR,
  DEFAULT_OBJECT_ICON,
  OBJECT_COLORS,
  OBJECT_ICON_NAMES,
  objectColor,
  objectIcon,
} from './icons'
export { ObjectIcon, type ObjectIconProps } from './ObjectIcon'
export { registerCustomRecordContributions } from './register'
export { useRelationTabs } from './relation-tabs'
export { RelationPanel, RelationsTab, type RelationPanelProps, type RelationsTabProps } from './RelationsTab'
export {
  CUSTOM_RECORD_TYPE_PREFIX,
  customObjectKey,
  customObjectPath,
  customRecordPath,
  customRecordType,
  isCustomRecordType,
  registerCustomObjectResources,
  type CustomObject,
} from './resources'
export { customRecordsRoutes } from './routes'
