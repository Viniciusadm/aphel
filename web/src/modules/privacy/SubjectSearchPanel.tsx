import { Paperclip, Search, UserSearch } from 'lucide-react'
import { useId, useState, type FormEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { usePrivacyCatalog, useSubjectSearch, type SubjectSearch } from './api'
import { CreateRequestDialog, type RequestSelection } from './CreateRequestDialog'
import { localizedLabel, usePrivacyErrors } from './lib/shared'
import { RecordLink } from './RecordLink'
import { Field } from '@/components/ui/field'

interface SubjectSearchPanelProps {
  onRequestCreated: () => void
}

function recordKey(recordType: string, id: string) {
  return JSON.stringify(['record', recordType, id])
}

function itemKey(source: string, id: string) {
  return JSON.stringify(['item', source, id])
}

function allKeys(result: SubjectSearch): string[] {
  return [
    ...result.records.map((record) => recordKey(record.record_type, record.id)),
    ...result.items.map((item) => itemKey(item.source, item.id)),
  ]
}

function selectionFrom(keys: Set<string>): RequestSelection {
  const selection: RequestSelection = { records: [], items: [] }
  for (const key of keys) {
    const [kind, source, id] = JSON.parse(key) as [string, string, string]
    if (kind === 'record') selection.records.push({ record_type: source, id })
    else if (kind === 'item') selection.items.push({ source, id })
  }
  return selection
}

export function SubjectSearchPanel({ onRequestCreated }: SubjectSearchPanelProps) {
  const { t, i18n } = useTranslation('privacy')
  const catalog = usePrivacyCatalog()
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
                      <Badge variant="outline">{localizedLabel(catalog.data?.entities.find((entity) => entity.record_type === record.record_type)?.labels ?? {}, i18n.language, record.record_type)}</Badge>
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

          {result.sources.map((source) => {
            const items = result.items.filter((item) => item.source === source.key)
            if (items.length === 0) return null
            return <ResultGroup key={source.key} title={localizedLabel(source.labels, i18n.language, source.key)} count={items.length}>
              {items.map((item) => {
                const key = itemKey(item.source, item.id)
                return <SelectableItem key={key} id={key} checked={selected.has(key)} onChange={(checked) => toggle(key, checked)}>
                  {item.record
                    ? <RecordLink recordType={item.record.record_type} id={item.record.id} title={item.title} />
                    : <span className="font-medium break-words">{item.title}</span>}
                  {item.excerpt && <p className="text-sm break-words whitespace-pre-line">{item.excerpt}</p>}
                </SelectableItem>
              })}
            </ResultGroup>
          })}

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
