import { useMutation } from '@tanstack/react-query'
import { Loader2, MessageSquareText, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { useFieldSchema, useOrganizationUsers } from '@/features/records/api'

import { describeCondition, type SummaryContext } from '@/features/records/filters'
import { useRecordFormatting } from '@/features/records/formatting'
import { isFilterCondition, isFilterGroup, isFilterRelated, type FilterNode } from '@/features/records/types'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { SlotProps } from '@/kernel/modules/slots'
import { AI_KEY, aiApi, useAiStatus } from './api'
import { AiErrorMessage, AiFooter } from './AiNotice'
import { Field } from '@/components/ui/field'

const MAX_QUESTION = 500

export function QueryTool(props: SlotProps<'record.list.toolbar'>) {
  const t = useModuleTranslation(AI_KEY)
  const status = useAiStatus()
  const [open, setOpen] = useState(false)
  if (!status.data?.configured || !status.data.features.query) return null

  return (
    <>
      <Button type="button" variant="outline" size="sm" className="h-9" onClick={() => setOpen(true)}>
        <Sparkles aria-hidden="true" />
        <span className="hidden sm:inline">{t('query.button')}</span>
        <span className="sr-only sm:hidden">{t('query.button')}</span>
      </Button>
      {open && <QueryDialog {...props} onOpenChange={setOpen} />}
    </>
  )
}

interface QueryDialogProps extends SlotProps<'record.list.toolbar'> {
  onOpenChange: (open: boolean) => void
}

function QueryDialog({ recordType, filter, applyFilter, onOpenChange }: QueryDialogProps) {
  const t = useModuleTranslation(AI_KEY)
  const [question, setQuestion] = useState('')
  const query = useMutation({ mutationFn: (text: string) => aiApi.query(recordType, text) })

  const run = () => {
    const trimmed = question.trim()
    if (trimmed) query.mutate(trimmed)
  }

  const apply = (combine: boolean) => {
    if (!query.data) return
    applyFilter(combine && filter ? { op: 'and', conditions: [filter, query.data.filter] } : query.data.filter)
    notifySuccess(t('query.applied'))
    onOpenChange(false)
  }

  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('query.title')}</DialogTitle>
          <DialogDescription>{t('query.description')}</DialogDescription>
        </DialogHeader>
        <form
          className="grid gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            run()
          }}
        >
          <Field label={t('query.question')}>
            {(control) => <>
              <Textarea
              {...control}
              rows={2}
              maxLength={MAX_QUESTION}
              placeholder={t(`query.placeholder.${recordType}`, { defaultValue: t('query.placeholder.default') })}
              value={question}
              onChange={(event) => setQuestion(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && !event.shiftKey) {
                  event.preventDefault()
                  run()
                }
              }}
            />
            <p className="text-xs text-muted-foreground">{t('query.sends')}</p>
            </>}
          </Field>
          <Button type="submit" variant={query.data ? 'outline' : 'default'} disabled={query.isPending || !question.trim()}>
            {query.isPending ? <Loader2 className="animate-spin" aria-hidden="true" /> : <MessageSquareText aria-hidden="true" />}
            {t('query.generate')}
          </Button>
        </form>
        {query.isError && <AiErrorMessage error={query.error} />}
        {query.data && !query.isPending && (
          <div className="grid gap-2 rounded-md border bg-muted/30 p-3">
            {query.data.explanation && <p className="text-sm">{query.data.explanation}</p>}
            <p className="text-sm font-medium">{t('query.generatedFilter')}</p>
            <FilterPreview recordType={recordType} filter={query.data.filter} />
            <p className="text-sm text-muted-foreground">{t('query.total', { count: query.data.total })}</p>
            <details className="text-xs">
              <summary className="cursor-pointer text-muted-foreground select-none">{t('query.json')}</summary>
              <pre className="mt-1 max-h-48 overflow-auto rounded bg-background p-2 font-mono text-[11px] break-all whitespace-pre-wrap">
                {JSON.stringify(query.data.filter, null, 2)}
              </pre>
            </details>
            <AiFooter provider={query.data.provider} model={query.data.model} />
          </div>
        )}
        <DialogFooter className="gap-2">
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            {t('query.cancel')}
          </Button>
          {query.data && !query.isPending && filter && (
            <Button type="button" variant="outline" onClick={() => apply(true)}>
              {t('query.combine')}
            </Button>
          )}
          {query.data && !query.isPending && (
            <Button type="button" onClick={() => apply(false)}>
              {t(filter ? 'query.replace' : 'query.apply')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function FilterPreview({ recordType, filter }: { recordType: string; filter: FilterNode }) {
  const { t } = useTranslation('records-table')
  const schema = useFieldSchema(recordType)
  const users = useOrganizationUsers()
  const formatting = useRecordFormatting()
  if (!schema.data) return null
  const context: SummaryContext = { schema: schema.data, t, formatting, users: users.data ?? [] }
  return <FilterNodeView node={filter} context={context} />
}

function FilterNodeView({ node, context }: { node: FilterNode; context: SummaryContext }) {
  if (isFilterGroup(node)) {
    const joiner = node.op === 'or' ? context.t('filters.or') : context.t('filters.and')
    return (
      <ul className="grid gap-1 border-l-2 pl-3">
        {node.conditions.map((child, index) => (
          <li key={index} className="text-sm">
            {index > 0 && <span className="mr-1 text-xs text-muted-foreground uppercase">{joiner}</span>}
            <FilterNodeView node={child} context={context} />
          </li>
        ))}
      </ul>
    )
  }
  if (isFilterRelated(node)) {
    return (
      <span className="text-sm">
        {node.related.relationship} › <FilterNodeView node={node.related.filter} context={context} />
      </span>
    )
  }
  if (isFilterCondition(node)) {
    return <span className="rounded-full border bg-background px-2 py-0.5 text-sm">{describeCondition(node, context)}</span>
  }
  return null
}
