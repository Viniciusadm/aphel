import { Badge } from '@/components/ui/badge'
import { X } from 'lucide-react'
import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useOrganizationUsers } from '../api'
import { useRecordFormatting } from '../formatting'
import { isFilterCondition, isFilterGroup, type FieldSchema, type FilterNode } from '../types'
import { findExtension, useFilterSchema } from './extensions'
import { removeTopLevelNode, topLevelNodes } from './model'
import { describeNode, type SummaryContext } from './summary'

export interface FilterChipsProps {
  schema: FieldSchema
  filter: FilterNode | null
  onChange: (filter: FilterNode | null) => void
  className?: string
}

export function FilterChips({ schema: baseSchema, filter, onChange, className }: FilterChipsProps) {
  const { t } = useTranslation('records-table')
  const { schema, extensions } = useFilterSchema(baseSchema)
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  const nodes = topLevelNodes(filter)
  if (nodes.length === 0) return null

  const context: SummaryContext = { schema, t, formatting, users: users.data ?? [] }
  const joiner = filter && isFilterGroup(filter) && filter.op === 'or' ? t('filters.or') : t('filters.and')

  return (
    <div className={cn('flex flex-wrap items-center gap-1.5', className)} aria-label={t('filters.title')} role="group">
      {nodes.map((node, index) => {
        const summary = describeNode(node, context)
        const extension = isFilterCondition(node) ? findExtension(extensions, node.field) : undefined
        const Summary = extension?.Summary
        return (
          <Fragment key={`${index}-${summary}`}>
            {index > 0 && <span className="text-xs text-muted-foreground">{joiner}</span>}
            <Badge tone="neutral" className="max-w-full gap-1 rounded-full py-0.5 pr-1 pl-2.5">
              <span className="truncate">
                {Summary && isFilterCondition(node) ? (
                  <Summary recordType={schema.record_type} condition={node} label={extension.field.label} />
                ) : (
                  summary
                )}
              </span>
              <button
                type="button"
                className="inline-flex size-5 shrink-0 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                aria-label={t('filters.removeChip', { summary })}
                onClick={() => onChange(removeTopLevelNode(filter, index))}
              >
                <X className="size-3" aria-hidden />
              </button>
            </Badge>
          </Fragment>
        )
      })}
      <Button variant="link" size="sm" className="h-auto px-1 py-0 text-xs" onClick={() => onChange(null)}>
        {t('filters.clearAll')}
      </Button>
    </div>
  )
}
