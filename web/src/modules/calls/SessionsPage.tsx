import { PageHeader } from '@/components/ui/page-header'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQuery } from '@tanstack/react-query'
import { PhoneCall } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { recordsKeys } from '@/features/records/api'
import { listAllViews } from '@/modules/saved_views/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { callsApi, callsKeys } from './api'
import { callSessionPath } from './constants'
import { callErrorKey } from './lib/errors'
import { Field } from '@/components/ui/field'

const CONTACT = 'people.person'

export function SessionsPage() {
  const { hasPermission } = useSession()
  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      {hasPermission('tasks.create') && <StartSessionCard />}
      <SessionList />
    </div>
  )
}

function viewScript(settings: unknown): string {
  if (settings && typeof settings === 'object' && 'call_script' in settings) {
    const script = (settings as Record<string, unknown>).call_script
    if (typeof script === 'string') return script
  }
  return ''
}

function StartSessionCard() {
  const { t } = useTranslation('calls')
  const formId = useId()
  const navigate = useNavigate()
  const views = useQuery({
    queryKey: recordsKeys.views(CONTACT),
    queryFn: () => listAllViews(CONTACT),
  })
  const [viewId, setViewId] = useState('')
  const [name, setName] = useState('')
  const [script, setScript] = useState<string | null>(null)
  const [viewError, setViewError] = useState(false)
  const [saving, setSaving] = useState(false)
  const selected = views.data?.find((view) => view.id === viewId)
  const scriptValue = script ?? viewScript(selected?.settings)

  const submit = async () => {
    if (!viewId) {
      setViewError(true)
      return
    }
    setSaving(true)
    try {
      const session = await callsApi.startSession({
        view_id: viewId,
        name: name.trim() || null,
        script: scriptValue.trim() || null,
      })
      navigate(callSessionPath(session.id))
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <Card className="h-fit">
      <CardHeader>
        <CardTitle>{t('start.title')}</CardTitle>
        <CardDescription>{t('start.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form
          id={formId}
          className="grid gap-4"
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            void submit()
          }}
        >
          <Field id={`${formId}-view`} label={t('start.view')}>
            {views.isPending ? (
              <Skeleton className="h-9 w-full" />
            ) : views.isError ? (
              <ErrorState onRetry={() => void views.refetch()} />
            ) : (
              <NativeSelect
                id={`${formId}-view`}
                value={viewId}
                aria-invalid={viewError ? true : undefined}
                onChange={(event) => {
                  setViewId(event.target.value)
                  setScript(null)
                  setViewError(false)
                }}
              >
                <option value="">{t('start.pickView')}</option>
                {views.data.map((view) => (
                  <option key={view.id} value={view.id}>
                    {view.name}
                  </option>
                ))}
              </NativeSelect>
            )}
            {viewError && <SharedFieldError>{t('start.viewRequired')}</SharedFieldError>}
            <p className="text-xs text-muted-foreground">{t('start.viewHint')}</p>
          </Field>
          <Field label={t('start.name')}>
            {(fieldControl) => <Input
              {...fieldControl}
              value={name}
              placeholder={selected?.name ?? ''}
              onChange={(event) => setName(event.target.value)}
            />}
          </Field>
          <Field label={t('start.script')}>
            {(control) => <Textarea
              {...control}
              rows={5}
              value={scriptValue}
              placeholder={t('start.scriptPlaceholder')}
              onChange={(event) => setScript(event.target.value)}
            />}
          </Field>
          <Button type="submit" disabled={saving}>
            <PhoneCall aria-hidden="true" />
            {t('start.submit')}
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}

function SessionList() {
  const { t } = useTranslation('calls')
  const formatting = useRecordFormatting()
  const sessions = useQuery({
    queryKey: callsKeys.sessions(),
    queryFn: () => callsApi.sessions(),
  })
  return (
    <section className="grid h-fit gap-3">
      <PageHeader level="section" title={t('sessions.title')} />
      {sessions.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : sessions.isError ? (
        <ErrorState onRetry={() => void sessions.refetch()} />
      ) : sessions.data.data.length === 0 ? (
        <EmptyState icon={PhoneCall} title={t('sessions.emptyTitle')} description={t('sessions.emptyDescription')} />
      ) : (
        <ul className="grid gap-2">
          {sessions.data.data.map((session) => {
            const done = session.status === 'completed'
            return (
              <li key={session.id}>
                <Link
                  to={callSessionPath(session.id)}
                  className="flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2 hover:bg-muted/50"
                >
                  <div className="grid min-w-40 flex-1 gap-0.5">
                    <span className="truncate text-sm font-medium">{session.name}</span>
                    <span className="text-xs text-muted-foreground">
                      {t('sessions.startedAt', { date: formatDateTime(session.created_at, formatting) })}
                    </span>
                  </div>
                  <span className="text-xs text-muted-foreground tabular-nums">
                    {t('sessions.people', { count: session.total })}
                  </span>
                  <Badge variant={done ? 'secondary' : 'success'}>{t(`sessions.status.${session.status}`)}</Badge>
                </Link>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
