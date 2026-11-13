import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import {
  type FilterNode,
  type SummaryContext,
  describeNode,
  isFilterGroup,
  useFieldSchema,
  useFilterSchema,
  useOrganizationUsers,
  useRecordFormatting,
} from '@/sdk'

const MAX_VISIBLE = 3

/** Resumo somente leitura das condições de um segmento, em chips curtos. */
export function SegmentFilterSummary({ recordType, filter }: { recordType: string; filter: FilterNode }) {
  const { t } = useTranslation('records-table')
  const { t: ts } = useTranslation('segments')
  const base = useFieldSchema(recordType)
  const { schema } = useFilterSchema(base.data)
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  if (!schema) return null
  const nodes = isFilterGroup(filter) ? filter.conditions : [filter]
  const context: SummaryContext = { schema, t, formatting, users: users.data ?? [] }
  const visible = nodes.slice(0, MAX_VISIBLE)
  return (
    <div className="flex flex-wrap items-center gap-1.5" aria-label={ts('page.conditions')} role="group">
      {visible.map((node, index) => (
        <Badge key={index} tone="neutral" className="max-w-full rounded-full">
          <span className="truncate">{describeNode(node, context)}</span>
        </Badge>
      ))}
      {nodes.length > MAX_VISIBLE && (
        <span className="text-xs text-muted-foreground">{ts('page.moreConditions', { count: nodes.length - MAX_VISIBLE })}</span>
      )}
    </div>
  )
}
