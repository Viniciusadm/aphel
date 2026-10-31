import { FileText, Paperclip, Search, UserSearch } from 'lucide-react'
import { useId, useState, type FormEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useSubjectSearch, type SubjectSearch } from './api'
import { CreateRequestDialog, type RequestSelection } from './CreateRequestDialog'
import { usePrivacyErrors } from './lib/shared'
import { RecordLink } from './RecordLink'
import { Field } from '@/components/ui/field'

interface SubjectSearchPanelProps {
  onRequestCreated: () => void
}

function recordKey(recordType: string, id: string) {
  return `record:${recordType}:${id}`
}

function noteKey(id: string) {
  return `note:${id}`
}

function rowKey(jobId: string, index: number) {
  return `row:${jobId}:${index}`
}

function allKeys(result: SubjectSearch): string[] {
  return [
    ...result.records.map((record) => recordKey(record.record_type, record.id)),
    ...result.activities.map((activity) => recordKey('activity', activity.id)),
    ...result.notes.map((note) => noteKey(note.id)),
    ...result.import_rows.map((row) => rowKey(row.import_job_id, row.row_index)),
  ]
}

function selectionFrom(keys: Set<string>): RequestSelection {
  const selection: RequestSelection = { records: [], notes: [], import_rows: [] }
  for (const key of keys) {
    const [kind, first, second] = key.split(':')
    if (kind === 'record' && first && second) selection.records.push({ record_type: first, id: second })
    else if (kind === 'note' && first) selection.notes.push(first)
    else if (kind === 'row' && first && second) selection.import_rows.push({ import_job_id: first, row_index: Number(second) })
  }
  return selection
}

export function SubjectSearchPanel({ onRequestCreated }: SubjectSearchPanelProps) {
  const { t } = useTranslation('privacy')
  const formatting = useRecordFormatting()
  const inputId = useId()
  const hintId = useId()
  const search = useSubjectSearch()
  const notifyFailure = usePrivacyErrors()
  const [term, setTerm] = useState('')
  const [searchedTerm, setSearchedTerm] = useState('')
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [creating, setCreating] = useState(false)
  const result = search.data

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const value = term.trim()
    if (value.length < 3) return
    search.mutate(value, {
      onSuccess: (data) => {
        setSearchedTerm(value)
        setSelected(new Set(allKeys(data)))
      },
      onError: notifyFailure,
    })
  }

  const toggle = (key: string, checked: boolean) =>
    setSelected((current) => {
      const next = new Set(current)
      if (checked) next.add(key)
      else next.delete(key)
      return next
    })

  const total = result ? allKeys(result).length + result.attachments.length : 0

  return (
    <div className="grid min-w-0 gap-4">
      <form role="search" onSubmit={submit}>
        <Field id={inputId} label={t('search.label')}>
        <div className="flex min-w-0 gap-2">
          <Input
            id={inputId}
            value={term}
            autoComplete="off"
            spellCheck={false}
            minLength={3}
            maxLength={255}
            placeholder={t('search.placeholder')}
            aria-describedby={hintId}
            onChange={(event) => setTerm(event.target.value)}
          />
          <Button type="submit" disabled={search.isPending || term.trim().length < 3} aria-busy={search.isPending}>
            <Search aria-hidden="true" />
            <span className="sr-only sm:not-sr-only">{search.isPending ? t('search.searching') : t('search.submit')}</span>
          </Button>
        </div>
        <p id={hintId} className="text-xs text-muted-foreground">
          {t('search.hint')}
        </p>
              </Field>
      </form>

      {!result ? (
        <EmptyState icon={UserSearch} title={t('search.introTitle')} description={t('search.intro')} />
      ) : total === 0 ? (
        <EmptyState icon={UserSearch} title={t('search.empty')} description={t('search.emptyDescription')} />
      ) : (
        <div className="grid min-w-0 gap-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-sm" aria-live="polite">
              {t('search.summary', { label: result.label, kind: t(`subjectKinds.${result.kind as 'email'}`) })}
            </p>
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" onClick={() => setSelected(new Set(allKeys(result)))}>
                {t('search.selectAll')}
              </Button>
              <Button variant="outline" size="sm" onClick={() => setSelected(new Set())}>
                {t('search.selectNone')}
              </Button>
            </div>
          </div>

          {result.records.length > 0 && (
            <ResultGroup title={t('search.groups.records')} count={result.records.length}>
              {result.records.map((record) => {
                const key = recordKey(record.record_type, record.id)
                return (
                  <SelectableItem key={key} id={key} checked={selected.has(key)} onChange={(checked) => toggle(key, checked)}>
                    <div className="flex flex-wrap items-center gap-2">
                      <Badge variant="outline">{t(`recordTypes.${record.record_type as 'contact'}`)}</Badge>
                      <RecordLink recordType={record.record_type} id={record.id} title={record.title || record.id} />
                      {record.archived && <Badge variant="secondary">{t('search.archived')}</Badge>}
                      {record.deleted && <Badge variant="warning">{t('search.deleted')}</Badge>}
                    </div>
                    {record.subtitle && <p className="text-sm break-all text-muted-foreground">{record.subtitle}</p>}
                    {record.matched_fields.length > 0 && (
                      <p className="text-xs text-muted-foreground">
                        {t('search.matched', { fields: record.matched_fields.join(', ') })}
                      </p>
                    )}
                  </SelectableItem>
                )
              })}
            </ResultGroup>
          )}

          {result.activities.length > 0 && (
            <ResultGroup title={t('search.groups.activities')} count={result.activities.length}>
              {result.activities.map((activity) => {
                const key = recordKey('activity', activity.id)
                return (
                  <SelectableItem key={key} id={key} checked={selected.has(key)} onChange={(checked) => toggle(key, checked)}>
                    <span className="font-medium break-words">{activity.title}</span>
                    <p className="text-xs text-muted-foreground">
                      {t(`search.reason.${activity.reason as 'mention'}`)}
                      {activity.due_at ? ` · ${formatDateTime(activity.due_at, formatting)}` : ''}
                    </p>
                  </SelectableItem>
                )
              })}
            </ResultGroup>
          )}

          {result.notes.length > 0 && (
            <ResultGroup title={t('search.groups.notes')} count={result.notes.length}>
              {result.notes.map((note) => {
                const key = noteKey(note.id)
                return (
                  <SelectableItem key={key} id={key} checked={selected.has(key)} onChange={(checked) => toggle(key, checked)}>
                    <p className="text-sm break-words whitespace-pre-line">{note.excerpt}</p>
                    <p className="flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
                      <FileText className="size-3" aria-hidden="true" />
                      {t(`search.reason.${note.reason as 'mention'}`)} · {formatDateTime(note.created_at, formatting)}
                    </p>
                  </SelectableItem>
                )
              })}
            </ResultGroup>
          )}

          {result.attachments.length > 0 && (
            <ResultGroup title={t('search.groups.attachments')} count={result.attachments.length}>
              <p className="text-xs text-muted-foreground">{t('search.attachmentsNote')}</p>
              <ul className="grid gap-2">
                {result.attachments.map((attachment) => (
                  <li key={attachment.id} className="flex min-w-0 items-center gap-2 text-sm">
                    <Paperclip className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                    <span className="min-w-0 break-all">{attachment.file_name}</span>
                  </li>
                ))}
              </ul>
            </ResultGroup>
          )}

          {result.import_rows.length > 0 && (
            <ResultGroup title={t('search.groups.importRows')} count={result.import_rows.length}>
              {result.import_rows.map((row) => {
                const key = rowKey(row.import_job_id, row.row_index)
                return (
                  <SelectableItem key={key} id={key} checked={selected.has(key)} onChange={(checked) => toggle(key, checked)}>
                    <span className="font-medium break-all">{row.file_name ?? row.import_job_id}</span>
                    <p className="text-xs text-muted-foreground">
                      {t('search.row', { row: row.row_index + 1 })} · {formatDateTime(row.created_at, formatting)}
                    </p>
                  </SelectableItem>
                )
              })}
            </ResultGroup>
          )}

          <div className="sticky bottom-0 flex flex-wrap items-center justify-between gap-2 rounded-lg border bg-background p-3">
            <span className="text-sm text-muted-foreground" aria-live="polite">
              {t('search.selected', { count: selected.size })}
            </span>
            <Button disabled={selected.size === 0} onClick={() => setCreating(true)}>
              {t('search.createRequest')}
            </Button>
          </div>
        </div>
      )}

      {creating && (
        <CreateRequestDialog
          term={searchedTerm}
          selection={selectionFrom(selected)}
          onClose={() => setCreating(false)}
          onCreated={() => {
            setCreating(false)
            onRequestCreated()
          }}
        />
      )}
    </div>
  )
}

function ResultGroup({ title, count, children }: { title: string; count: number; children: ReactNode }) {
  return (
    <Card className="min-w-0">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          {title}
          <Badge variant="secondary">{count}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid min-w-0 gap-2">{children}</CardContent>
    </Card>
  )
}

interface SelectableItemProps {
  id: string
  checked: boolean
  onChange: (checked: boolean) => void
  children: ReactNode
}

function SelectableItem({ id, checked, onChange, children }: SelectableItemProps) {
  const domId = useId()
  return (
    <div className="flex min-w-0 items-start gap-3 rounded-md border px-3 py-2" data-key={id}>
      <Checkbox
        id={domId}
        className="mt-1"
        checked={checked}
        aria-labelledby={`${domId}-content`}
        onCheckedChange={(value) => onChange(value === true)}
      />
      <div id={`${domId}-content`} className="grid min-w-0 flex-1 gap-1">
        {children}
      </div>
    </div>
  )
}
