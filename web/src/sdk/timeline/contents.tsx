import { useMemo } from 'react'
import { Link } from 'react-router'
import { GenericTimelineContent, getRecordResource, type TimelineItem, type TimelineRecordReference, useFieldSchema } from '../records'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { TIMELINE_MODULE_KEY } from './api'
import { asObject, asText, eventType, nameOf, payloadOf } from './text'

const RECORD_ACTIONS = ['created', 'updated', 'deleted', 'restored', 'archived', 'unarchived', 'purged'] as const
const MAX_CHANGES = 6

type RecordAction = (typeof RECORD_ACTIONS)[number]

export function TimelineRecordLink({ record }: { record?: TimelineRecordReference | null }) {
  const t = useModuleTranslation(TIMELINE_MODULE_KEY)
  if (!record) return null
  const href = getRecordResource(record.record_type)?.detailPath?.(record.record_id)
  const label = record.title || t('timeline.relatedRecord')
  return href ? (
    <Link to={href} className="w-fit max-w-full truncate text-sm text-primary underline-offset-4 hover:underline">
      {label}
    </Link>
  ) : (
    <span className="truncate text-sm text-muted-foreground">{label}</span>
  )
}

export function TimelineLine({ text, item }: { text: string; item: TimelineItem }) {
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{text}</p>
      <TimelineRecordLink record={item.record} />
    </div>
  )
}

function displayValue(value: unknown): string {
  if (value === null || value === undefined || value === '') return '—'
  if (typeof value === 'object') return nameOf(value) ?? JSON.stringify(value)
  return String(value)
}

export function RecordChangeTimelineContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(TIMELINE_MODULE_KEY)
  const action = eventType(item).split('.').at(-1) ?? ''
  const knownAction = (RECORD_ACTIONS as readonly string[]).includes(action) ? (action as RecordAction) : null
  const changes = Array.isArray(payloadOf(item).changes) ? (payloadOf(item).changes as unknown[]) : []
  const schemaType = item.record?.record_type ?? asText(item.metadata.record_type) ?? ''
  const schema = useFieldSchema(schemaType, knownAction === 'updated' && changes.length > 0)
  const fieldLabels = useMemo(
    () => new Map((schema.data?.fields ?? []).map((field) => [field.key, field.label])),
    [schema.data],
  )
  const text =
    knownAction ? t(`timeline.actions.${knownAction}`) : t('timeline.recordChanged')
  return (
    <div className="grid min-w-0 gap-1">
      <TimelineLine text={text} item={item} />
      {knownAction === 'updated' && changes.length > 0 && (
        <ul className="grid gap-0.5 text-xs text-muted-foreground">
          {changes.slice(0, MAX_CHANGES).map((raw, index) => {
            const change = asObject(raw)
            const key = asText(change.field)
            return (
              <li key={index} className="break-words">
                {t('timeline.fieldChange', {
                  field: (key && fieldLabels.get(key)) || key || '?',
                  from: displayValue(change.old),
                  to: displayValue(change.new),
                })}
              </li>
            )
          })}
          {changes.length > MAX_CHANGES && <li>{t('timeline.moreChanges', { count: changes.length - MAX_CHANGES })}</li>}
        </ul>
      )}
    </div>
  )
}

export function AttachmentTimelineContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(TIMELINE_MODULE_KEY)
  const name = asText(payloadOf(item).file_name) ?? t('timeline.file')
  const text =
    eventType(item) === 'attachment.removed'
      ? t('timeline.attachmentRemoved', { name })
      : t('timeline.attachmentAdded', { name })
  return <TimelineLine text={text} item={item} />
}

export function EventTimelineContent({ item }: { item: TimelineItem }) {
  return <GenericTimelineContent item={{ ...item, title: eventType(item) }} />
}
