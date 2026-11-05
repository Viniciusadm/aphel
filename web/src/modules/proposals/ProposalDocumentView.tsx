import { Fragment } from 'react'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {PROPOSALS_KEY, type ProposalBlock, type ProposalItemsTable, type ProposalSpan} from '@/sdk'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

function Spans({ spans }: { spans: ProposalSpan[] }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  return (
    <>
      {spans.map((span, index) => {
        if (span.variable && span.text === '') {
          return (
            <span
              key={index}
              title={span.variable}
              className="rounded bg-muted px-1 text-xs text-muted-foreground italic"
            >
              {t('preview.empty')}
            </span>
          )
        }
        const placeholder = span.variable !== null && span.variable !== undefined && span.text === `[${span.variable}]`
        return (
          <span
            key={index}
            title={span.variable ?? undefined}
            className={cn(
              span.bold && 'font-semibold',
              placeholder && 'rounded bg-primary/10 px-1 font-mono text-xs text-primary',
            )}
          >
            {span.text}
          </span>
        )
      })}
    </>
  )
}

function ItemsTableView({ table }: { table: ProposalItemsTable }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  return (
    <div className="grid gap-2">
      <Table className="min-w-[480px]">
        <TableHeader>
          <TableRow>
            <TableHead className="px-2 py-1.5 font-semibold">{t('preview.item')}</TableHead>
            <TableHead className="px-2 py-1.5 text-right font-semibold">{t('preview.quantity')}</TableHead>
            <TableHead className="px-2 py-1.5 text-right font-semibold">{t('preview.unitPrice')}</TableHead>
            <TableHead className="px-2 py-1.5 text-right font-semibold">{t('preview.discount')}</TableHead>
            <TableHead className="px-2 py-1.5 text-right font-semibold">{t('preview.total')}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {table.rows.length === 0 ? (
            <TableRow>
              <TableCell colSpan={5} className="px-2 text-muted-foreground">
                {table.available ? t('preview.noItems') : t('preview.itemsUnavailable')}
              </TableCell>
            </TableRow>
          ) : (
            table.rows.map((row, index) => (
              <TableRow key={index}>
                <TableCell className="px-2 py-1.5 break-words">{row.name}</TableCell>
                <TableCell className="px-2 py-1.5 text-right tabular-nums">{row.quantity}</TableCell>
                <TableCell className="px-2 py-1.5 text-right tabular-nums whitespace-nowrap">{row.unit_price}</TableCell>
                <TableCell className="px-2 py-1.5 text-right tabular-nums whitespace-nowrap">{row.discount}</TableCell>
                <TableCell className="px-2 py-1.5 text-right tabular-nums whitespace-nowrap">{row.total}</TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
      <dl className="ml-auto grid w-full max-w-64 grid-cols-[1fr_auto] gap-x-4 gap-y-0.5 text-sm">
        <dt className="text-muted-foreground">{t('preview.subtotal')}</dt>
        <dd className="text-right tabular-nums">{table.subtotal}</dd>
        <dt className="text-muted-foreground">{t('preview.discounts')}</dt>
        <dd className="text-right tabular-nums">{table.discount_total}</dd>
        <dt className="font-semibold">{t('preview.total')}</dt>
        <dd className="text-right font-semibold tabular-nums">{table.total}</dd>
      </dl>
    </div>
  )
}

function Block({ block }: { block: ProposalBlock }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  switch (block.type) {
    case 'heading': {
      const size = block.level === 1 ? 'text-xl' : block.level === 2 ? 'text-lg' : 'text-base'
      return (
        <p role="heading" aria-level={block.level + 1} className={cn('font-semibold break-words', size)}>
          <Spans spans={block.spans} />
        </p>
      )
    }
    case 'paragraph':
      return (
        <p className="text-sm leading-relaxed break-words">
          {block.lines.map((line, index) => (
            <Fragment key={index}>
              {index > 0 && <br />}
              <Spans spans={line} />
            </Fragment>
          ))}
        </p>
      )
    case 'list':
      return (
        <ul className="list-disc space-y-0.5 pl-5 text-sm break-words">
          {block.items.map((item, index) => (
            <li key={index}>
              <Spans spans={item} />
            </li>
          ))}
        </ul>
      )
    case 'items_table':
      return <ItemsTableView table={block.table} />
    case 'page_break':
      return (
        <div className="flex items-center gap-2 text-xs text-muted-foreground" role="separator">
          <span className="h-px flex-1 border-t border-dashed" />
          {t('preview.pageBreak')}
          <span className="h-px flex-1 border-t border-dashed" />
        </div>
      )
    case 'rule':
      return <hr className="border-border" />
  }
}

export function ProposalDocumentView({ blocks, className }: { blocks: ProposalBlock[]; className?: string }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  return (
    <div className={cn('grid min-w-0 gap-3 rounded-md border bg-card p-4 text-card-foreground', className)}>
      {blocks.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('preview.emptyDocument')}</p>
      ) : (
        blocks.map((block, index) => <Block key={index} block={block} />)
      )}
    </div>
  )
}
