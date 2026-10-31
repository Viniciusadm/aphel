import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { AlertCircle, CheckCircle2, FileQuestion, Loader2 } from 'lucide-react'
import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { RecordFieldInput } from '@/components/fields'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  MANAGE_PERMISSION,
  PROPOSALS_KEY,
  PROPOSALS_PATH,
  proposalsApi,
  proposalsKeys,
  useInvalidateProposals,
  useProposalTemplate,
  useProposalVariables,
  type ProposalIssue,
  type ProposalTemplate,
  type ProposalVariable,
} from './api'
import { NoAccess } from './NoAccess'
import { ProposalDocumentView } from './ProposalDocumentView'
import { VariablePicker } from './VariablePicker'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Alert } from '@/components/ui/alert'

const DEBOUNCE_MS = 450
const SYNTAX_SAMPLES = [
  ['# / ##', 'editor.syntaxHeading'],
  ['**…**', 'editor.syntaxBold'],
  ['- item', 'editor.syntaxList'],
  ['---', 'editor.syntaxRule'],
  ['{{items_table}} · {{page_break}}', 'editor.syntaxBlocks'],
] as const

interface FormValues {
  key: string
  name: string
  description: string
  body: string
  isActive: boolean
  isDefault: boolean
}

function initialValues(template: ProposalTemplate | null): FormValues {
  return {
    key: template?.key ?? '',
    name: template?.name ?? '',
    description: template?.description ?? '',
    body: template?.body ?? '',
    isActive: template?.is_active ?? true,
    isDefault: template?.is_default ?? false,
  }
}

function sameValues(left: FormValues, right: FormValues): boolean {
  return (Object.keys(left) as (keyof FormValues)[]).every((key) => left[key] === right[key])
}

function useDebounced<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delay)
    return () => clearTimeout(timer)
  }, [value, delay])
  return debounced
}

export function TemplateEditorPage() {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const { templateId } = useParams()
  const { hasPermission } = useSession()
  const template = useProposalTemplate(templateId)

  if (!hasPermission(MANAGE_PERMISSION)) return <NoAccess />
  if (!templateId) return <EditorForm template={null} />
  if (template.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-96 w-full" />
      </div>
    )
  }
  if (template.isError) {
    if (isApiError(template.error) && template.error.status === 404) {
      return (
        <EmptyState
          icon={FileQuestion}
          title={t('editor.notFoundTitle')}
          description={t('editor.notFoundDescription')}
          action={
            <Button asChild variant="outline" size="sm">
              <Link to={PROPOSALS_PATH}>{t('editor.back')}</Link>
            </Button>
          }
        />
      )
    }
    return <ErrorState onRetry={() => void template.refetch()} />
  }
  return <EditorForm key={template.data.id} template={template.data} />
}

function issueOffset(body: string, issue: ProposalIssue): number {
  const lines = body.split('\n')
  const before = lines.slice(0, Math.max(0, issue.line - 1)).reduce((total, line) => total + line.length + 1, 0)
  return Math.min(body.length, before + Math.max(0, issue.column - 1))
}

function EditorForm({ template }: { template: ProposalTemplate | null }) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const navigate = useNavigate()
  const invalidate = useInvalidateProposals()
  const formId = useId()
  const bodyRef = useRef<HTMLTextAreaElement>(null)
  const leaving = useRef(false)
  const [saved, setSaved] = useState<FormValues>(() => initialValues(template))
  const [values, setValues] = useState<FormValues>(() => initialValues(template))
  const [keyError, setKeyError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [dealId, setDealId] = useState<string | null>(null)
  const variables = useProposalVariables()
  const body = useDebounced(values.body, DEBOUNCE_MS)
  const dirty = !sameValues(values, saved)

  const validation = useQuery({
    queryKey: [...proposalsKeys.all, 'validate', body],
    queryFn: ({ signal }) => proposalsApi.validate(body, signal),
    enabled: body.trim().length > 0,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  })
  const preview = useQuery({
    queryKey: [...proposalsKeys.all, 'preview', body, dealId],
    queryFn: ({ signal }) => proposalsApi.preview({ body, deal_id: dealId ?? undefined }, signal),
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  })

  const issues = useMemo(() => validation.data?.issues ?? [], [validation.data])
  const checking = values.body !== body || validation.isFetching
  const change = <K extends keyof FormValues>(key: K, value: FormValues[K]) =>
    setValues((previous) => ({ ...previous, [key]: value }))

  const insertVariable = (variable: ProposalVariable) => {
    const textarea = bodyRef.current
    const current = values.body
    const start = textarea?.selectionStart ?? current.length
    const end = textarea?.selectionEnd ?? current.length
    const token = `{{${variable.key}}}`
    let insertion = token
    if (variable.block) {
      const before = current.slice(0, start)
      const after = current.slice(end)
      insertion = `${before && !before.endsWith('\n') ? '\n' : ''}${token}${after.startsWith('\n') ? '' : '\n'}`
    }
    const next = current.slice(0, start) + insertion + current.slice(end)
    change('body', next)
    requestAnimationFrame(() => {
      if (!textarea) return
      const caret = start + insertion.length
      textarea.focus()
      textarea.setSelectionRange(caret, caret)
    })
  }

  const focusIssue = (issue: ProposalIssue) => {
    const textarea = bodyRef.current
    if (!textarea) return
    const offset = issueOffset(values.body, issue)
    textarea.focus()
    textarea.setSelectionRange(offset, offset)
  }

  const submit = async () => {
    setKeyError(null)
    setSaving(true)
    try {
      if (template) {
        const updated = await proposalsApi.updateTemplate(template.id, {
          name: values.name,
          description: values.description.trim() || null,
          body: values.body,
          is_active: values.isActive,
          is_default: values.isDefault,
        })
        const next = initialValues(updated)
        setSaved(next)
        setValues(next)
        notifySuccess(t('editor.saved', { name: updated.name }))
        await invalidate()
      } else {
        const created = await proposalsApi.createTemplate({
          key: values.key.trim() || undefined,
          name: values.name,
          description: values.description.trim() || undefined,
          body: values.body,
          is_active: values.isActive,
          is_default: values.isDefault,
        })
        notifySuccess(t('editor.created', { name: created.name }))
        await invalidate()
        leaving.current = true
        navigate(`${PROPOSALS_PATH}/${created.id}`, { replace: true })
      }
    } catch (error) {
      if (isApiError(error) && error.code === 'proposal_template_key_taken') {
        setKeyError(t('editor.keyTaken'))
      } else if (isApiError(error) && error.code === 'proposal_template_invalid') {
        notifyError(error, { title: t('editor.invalid') })
      } else {
        notifyError(error)
      }
    } finally {
      setSaving(false)
    }
  }

  const canSave = values.name.trim().length > 0 && values.body.trim().length > 0 && !saving && (dirty || !template)

  return (
    <Page>
      <UnsavedChangesGuard when={() => dirty && !leaving.current} />
      <PageHeader
        back={{ to: PROPOSALS_PATH, label: t('editor.back') }}
        title={template ? t('editor.editTitle') : t('editor.createTitle')}
        actions={
          <Button type="submit" form={formId} disabled={!canSave}>
            {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
            {saving ? t('editor.saving') : t('editor.save')}
          </Button>
        }
      />

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <form
          id={formId}
          className="grid min-w-0 content-start gap-4 rounded-lg border bg-card p-4"
          onSubmit={(event) => {
            event.preventDefault()
            if (canSave) void submit()
          }}
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-2.5">
              <Label htmlFor={`${formId}-name`}>{t('editor.name')}</Label>
              <Input placeholder={t('editor.namePlaceholder')}
                id={`${formId}-name`}
                value={values.name}
                maxLength={160}
                required
                onChange={(event) => change('name', event.target.value)}
              />
            </div>
            <div className="grid gap-2.5">
              <Label htmlFor={`${formId}-key`}>{t('editor.key')}</Label>
              <Input placeholder={t('editor.keyPlaceholder')}
                id={`${formId}-key`}
                value={values.key}
                maxLength={64}
                disabled={Boolean(template)}
                className="font-mono"
                aria-invalid={keyError ? true : undefined}
                aria-describedby={`${formId}-key-help`}
                onChange={(event) => change('key', event.target.value.toLowerCase())}
              />
              <p id={`${formId}-key-help`} className={keyError ? 'text-xs text-destructive' : 'text-xs text-muted-foreground'}>
                {keyError ?? t('editor.keyHelp')}
              </p>
            </div>
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={`${formId}-description`}>{t('editor.descriptionLabel')}</Label>
            <Input placeholder={t('editor.descriptionPlaceholder')}
              id={`${formId}-description`}
              value={values.description}
              maxLength={500}
              onChange={(event) => change('description', event.target.value)}
            />
          </div>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <label className="flex items-center gap-2 text-sm">
              <Checkbox
                checked={values.isActive}
                onCheckedChange={(checked) => {
                  const active = checked === true
                  setValues((previous) => ({ ...previous, isActive: active, isDefault: active && previous.isDefault }))
                }}
              />
              {t('editor.active')}
            </label>
            <label className="flex items-center gap-2 text-sm" title={t('editor.defaultHelp')}>
              <Checkbox
                checked={values.isDefault}
                disabled={!values.isActive}
                onCheckedChange={(checked) => change('isDefault', checked === true)}
              />
              {t('editor.default')}
            </label>
          </div>
          <div className="grid gap-1.5">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <Label htmlFor={`${formId}-body`}>{t('editor.body')}</Label>
              <VariablePicker
                variables={variables.data ?? []}
                disabled={variables.isPending}
                onPick={insertVariable}
              />
            </div>
            <Textarea placeholder={t('editor.bodyPlaceholder')}
              ref={bodyRef}
              id={`${formId}-body`}
              value={values.body}
              spellCheck
              aria-invalid={issues.length > 0 ? true : undefined}
              aria-describedby={`${formId}-issues`}
              className="min-h-[420px] font-mono text-[13px] leading-relaxed"
              onChange={(event) => change('body', event.target.value)}
            />
            <IssuesPanel
              id={`${formId}-issues`}
              issues={issues}
              checking={checking && values.body.trim().length > 0}
              empty={values.body.trim().length === 0}
              onFocusIssue={focusIssue}
            />
          </div>
          <details className="rounded-md border px-3 py-2 text-sm">
            <summary className="cursor-pointer font-medium">{t('editor.syntaxTitle')}</summary>
            <ul className="mt-2 grid gap-1 text-muted-foreground">
              {SYNTAX_SAMPLES.map(([sample, label]) => (
                <li key={label} className="flex flex-wrap gap-x-2">
                  <code className="rounded bg-muted px-1 text-foreground">{sample}</code>
                  <span>{t(label)}</span>
                </li>
              ))}
              <li>{t('editor.syntaxEscape')}</li>
            </ul>
          </details>
        </form>

        <section className="grid min-w-0 content-start gap-3 lg:sticky lg:top-4 lg:self-start" aria-label={t('preview.title')}>
          <div className="grid gap-1.5 rounded-lg border bg-card p-4">
            <h2 className="text-base font-semibold">{t('preview.title')}</h2>
            <Label htmlFor={`${formId}-deal`} className="text-sm">
              {t('preview.deal')}
            </Label>
            <RecordFieldInput
              id={`${formId}-deal`}
              targetRecordType="deal"
              value={dealId}
              onChange={setDealId}
            />
            <p className="text-xs text-muted-foreground">{t('preview.hint')}</p>
          </div>
          {preview.isError ? (
            <ErrorState description={t('preview.error')} onRetry={() => void preview.refetch()} />
          ) : preview.data ? (
            <ProposalDocumentView
              blocks={preview.data.blocks}
              className={preview.isFetching ? 'opacity-70 transition-opacity' : undefined}
            />
          ) : (
            <Skeleton className="h-64 w-full" />
          )}
        </section>
      </div>
    </Page>
  )
}

interface IssuesPanelProps {
  id: string
  issues: ProposalIssue[]
  checking: boolean
  empty: boolean
  onFocusIssue: (issue: ProposalIssue) => void
}

function IssuesPanel({ id, issues, checking, empty, onFocusIssue }: IssuesPanelProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  if (empty) return <div id={id} />
  if (checking && issues.length === 0) {
    return (
      <p id={id} className="flex items-center gap-1.5 text-xs text-muted-foreground" aria-live="polite">
        <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
        {t('editor.checking')}
      </p>
    )
  }
  if (issues.length === 0) {
    return (
      <p id={id} className="flex items-center gap-1.5 text-xs text-success" aria-live="polite">
        <CheckCircle2 className="size-3.5" aria-hidden="true" />
        {t('editor.valid')}
      </p>
    )
  }
  return (
    <Alert id={id} className="grid gap-1" aria-live="polite">
      <p className="flex items-center gap-1.5 text-xs font-medium text-destructive">
        <AlertCircle className="size-3.5" aria-hidden="true" />
        {t('editor.issues', { count: issues.length })}
      </p>
      <ul className="grid gap-0.5">
        {issues.map((issue, index) => (
          <li key={`${issue.line}-${issue.column}-${index}`}>
            <button
              type="button"
              className="w-full rounded px-1 py-0.5 text-left text-xs hover:bg-destructive/10"
              aria-label={t('editor.goToIssue', { line: issue.line, column: issue.column })}
              onClick={() => onFocusIssue(issue)}
            >
              <span className="font-medium">{t('editor.issueAt', { line: issue.line, column: issue.column })}</span>
              {': '}
              {t(`issues.${issue.code}`, { variable: issue.variable ?? '' })}
            </button>
          </li>
        ))}
      </ul>
    </Alert>
  )
}
