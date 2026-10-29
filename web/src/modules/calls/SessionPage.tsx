import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, CheckCircle2, ChevronLeft, Circle, Phone, PhoneCall, SkipForward } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { api } from '@/api'
import { ConfirmDialog } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { recordTimelineSource } from '@/modules/timeline'
import { contactResource } from '@/modules/contacts/resource'
import { recordsKeys } from '@/features/records/api'
import { recordPath, recordTitle } from '@/features/records/resources'
import { Timeline } from '@/features/records/timeline'
import type { RecordItem } from '@/features/records/types'
import { cn } from '@/lib/utils'
import { callsApi, callsKeys, useCallOutcomes, useInvalidateCalls, useTelephonyProviders } from './api'
import { CallForm } from './components/CallForm'
import { EMPTY_CALL_FORM, type CallFormValues } from './components/call-form-values'
import { CALLS_PATH } from './constants'
import { callErrorKey } from './lib/errors'
import { joinDuration } from './lib/format'
import type { CallSession, CallSessionItem, StartedCall } from './types'

const CONTACT = 'contact'

export function SessionPage() {
  const { t } = useTranslation('calls')
  const { id = '' } = useParams()
  const session = useQuery({
    queryKey: callsKeys.session(id),
    queryFn: () => callsApi.session(id),
  })

  if (session.isPending) {
    return (
      <div className="grid gap-3" aria-busy="true">
        <Skeleton className="h-10 w-1/2" />
        <Skeleton className="h-64 w-full" />
      </div>
    )
  }
  if (session.isError) return <ErrorState onRetry={() => void session.refetch()} />
  return (
    <div className="flex w-full flex-col gap-4">
      <Link to={CALLS_PATH} className="inline-flex w-fit items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
        <ArrowLeft className="size-4" aria-hidden="true" />
        {t('session.back')}
      </Link>
      <SessionWorkspace session={session.data} />
    </div>
  )
}

function SessionWorkspace({ session }: { session: CallSession }) {
  const { t } = useTranslation('calls')
  const client = useQueryClient()
  const invalidate = useInvalidateCalls()
  const [confirmFinish, setConfirmFinish] = useState(false)
  const [busy, setBusy] = useState(false)
  const counts = session.counts ?? { pending: 0, logged: 0, skipped: 0 }
  const handled = counts.logged + counts.skipped
  const completed = session.status === 'completed'

  const apply = async (action: () => Promise<CallSession>) => {
    setBusy(true)
    try {
      const next = await action()
      client.setQueryData(callsKeys.session(session.id), next)
      await client.invalidateQueries({ queryKey: callsKeys.sessionItems(session.id) })
      await client.invalidateQueries({ queryKey: callsKeys.sessions() })
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setBusy(false)
    }
  }

  return (
    <>
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div className="grid gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">{session.name}</h1>
          <p className="text-sm text-muted-foreground tabular-nums">
            {t('session.progress', { done: handled, total: session.total })}
            {' · '}
            {t('session.counts', { logged: counts.logged, skipped: counts.skipped })}
          </p>
          <div className="h-1.5 w-64 max-w-full overflow-hidden rounded-full bg-muted" aria-hidden="true">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${session.total ? Math.round((handled / session.total) * 100) : 0}%` }}
            />
          </div>
        </div>
        {!completed && (
          <Button variant="outline" size="sm" disabled={busy} onClick={() => setConfirmFinish(true)}>
            {t('session.finish')}
          </Button>
        )}
      </header>
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div className="grid min-w-0 gap-4">
          {completed || !session.current ? (
            <EmptyState
              icon={CheckCircle2}
              title={t('session.completedTitle')}
              description={t('session.completedDescription', { logged: counts.logged, skipped: counts.skipped })}
              action={
                <Button asChild variant="outline">
                  <Link to={CALLS_PATH}>{t('session.backToSessions')}</Link>
                </Button>
              }
            />
          ) : (
            <CurrentContact
              key={`${session.current.position}-${session.current.contact_id}`}
              session={session}
              item={session.current}
              busy={busy}
              onSkip={() => void apply(() => callsApi.skip(session.id, session.position))}
              onBack={
                session.position > 0
                  ? () => void apply(() => callsApi.moveTo(session.id, session.position - 1))
                  : undefined
              }
              onLogged={async (next) => {
                client.setQueryData(callsKeys.session(session.id), next)
                await invalidate()
              }}
            />
          )}
        </div>
        <QueueList
          session={session}
          disabled={busy || completed}
          onPick={(position) => void apply(() => callsApi.moveTo(session.id, position))}
        />
      </div>
      <ConfirmDialog
        open={confirmFinish}
        onOpenChange={setConfirmFinish}
        title={t('session.finishTitle')}
        description={t('session.finishDescription', { pending: counts.pending })}
        confirmLabel={t('session.finish')}
        onConfirm={() => apply(() => callsApi.finish(session.id))}
      />
    </>
  )
}

interface CurrentContactProps {
  session: CallSession
  item: CallSessionItem
  busy: boolean
  onSkip: () => void
  onBack?: () => void
  onLogged: (session: CallSession) => Promise<void>
}

function textValue(record: RecordItem | undefined, key: string): string | null {
  const value = record?.[key]
  return typeof value === 'string' && value.trim() ? value : null
}

function CurrentContact({ session, item, busy, onSkip, onBack, onLogged }: CurrentContactProps) {
  const { t } = useTranslation('calls')
  const client = useQueryClient()
  const outcomes = useCallOutcomes()
  const providers = useTelephonyProviders()
  const contact = useQuery({
    queryKey: recordsKeys.detail(CONTACT, item.contact_id),
    queryFn: () => api.get<RecordItem>(recordPath(contactResource, item.contact_id)),
  })
  const [values, setValues] = useState<CallFormValues>(EMPTY_CALL_FORM)
  const [outcomeError, setOutcomeError] = useState(false)
  const [saving, setSaving] = useState(false)
  const [started, setStarted] = useState<StartedCall | null>(null)
  const [dialed, setDialed] = useState<string | null>(null)
  const phones = [textValue(contact.data, 'mobile'), textValue(contact.data, 'phone')].filter(
    (phone): phone is string => phone !== null,
  )
  const title = contact.data ? recordTitle(contactResource, contact.data) : (item.contact_name ?? t('session.unknownContact'))
  const detailHref = contactResource.detailPath?.(item.contact_id)
  const provider = providers.data?.[0]

  const dial = async (phone: string) => {
    setDialed(phone)
    if (!provider) return
    try {
      setStarted(
        await callsApi.start({ provider: provider.key, phone_number: phone, record_type: CONTACT, record_id: item.contact_id }),
      )
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    }
  }

  const submit = async () => {
    if (!values.outcomeKey) {
      setOutcomeError(true)
      return
    }
    setSaving(true)
    try {
      const result = await callsApi.logInSession(session.id, {
        position: item.position,
        direction: values.direction,
        outcome_key: values.outcomeKey,
        duration_seconds: joinDuration(values.minutes, values.seconds),
        phone_number: dialed ?? phones[0] ?? null,
        notes: values.notes.trim() || null,
        provider: started?.provider ?? null,
        provider_call_id: started?.provider_call_id ?? null,
      })
      await client.invalidateQueries({ queryKey: recordsKeys.detail(CONTACT, item.contact_id) })
      toast({ title: t('session.logged', { name: title }), variant: 'success' })
      await onLogged(result.session)
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row flex-wrap items-start justify-between gap-3">
          <div className="grid min-w-0 gap-1">
            <p className="text-xs text-muted-foreground tabular-nums">
              {t('session.position', { position: item.position + 1, total: session.total })}
            </p>
            <CardTitle className="truncate text-xl">
              {detailHref ? (
                <Link to={detailHref} className="hover:underline">
                  {title}
                </Link>
              ) : (
                title
              )}
            </CardTitle>
            {contact.isSuccess && (
              <p className="text-sm text-muted-foreground">
                {[textValue(contact.data, 'job_title'), textValue(contact.data, 'email')].filter(Boolean).join(' · ')}
              </p>
            )}
          </div>
          {item.status !== 'pending' && (
            <Badge variant="secondary">{t(`session.itemStatus.${item.status}`)}</Badge>
          )}
        </CardHeader>
        <CardContent className="grid gap-5">
          {contact.isPending ? (
            <Skeleton className="h-9 w-48" />
          ) : contact.isError ? (
            <p className="text-sm text-destructive">{t('session.contactUnavailable')}</p>
          ) : phones.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('session.noPhone')}</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {phones.map((phone) => (
                <Button key={phone} asChild={!provider} variant={dialed === phone ? 'default' : 'outline'} onClick={() => void dial(phone)}>
                  {provider ? (
                    <span>
                      <Phone aria-hidden="true" />
                      {t('session.call', { phone })}
                    </span>
                  ) : (
                    <a href={`tel:${phone}`}>
                      <Phone aria-hidden="true" />
                      {t('session.call', { phone })}
                    </a>
                  )}
                </Button>
              ))}
            </div>
          )}
          {session.script && (
            <details className="rounded-md border bg-muted/40 p-3" open>
              <summary className="cursor-pointer text-sm font-medium">{t('session.script')}</summary>
              <p className="mt-2 text-sm whitespace-pre-line">{session.script}</p>
            </details>
          )}
          <form
            className="grid gap-4"
            noValidate
            onSubmit={(event) => {
              event.preventDefault()
              void submit()
            }}
          >
            {outcomes.isPending ? (
              <Skeleton className="h-32 w-full" />
            ) : outcomes.isError ? (
              <ErrorState onRetry={() => void outcomes.refetch()} />
            ) : (
              <CallForm
                values={values}
                onChange={(next) => {
                  setValues(next)
                  if (next.outcomeKey) setOutcomeError(false)
                }}
                outcomes={outcomes.data}
                outcomeError={outcomeError}
                disabled={saving || busy}
              />
            )}
            <div className="flex flex-wrap gap-2">
              <Button type="submit" disabled={saving || busy || !outcomes.data}>
                <PhoneCall aria-hidden="true" />
                {t('session.logAndNext')}
              </Button>
              <Button type="button" variant="outline" disabled={saving || busy} onClick={onSkip}>
                <SkipForward aria-hidden="true" />
                {t('session.skip')}
              </Button>
              {onBack && (
                <Button type="button" variant="ghost" disabled={saving || busy} onClick={onBack}>
                  <ChevronLeft aria-hidden="true" />
                  {t('session.previous')}
                </Button>
              )}
            </div>
          </form>
        </CardContent>
      </Card>
      <section className="grid gap-2">
        <h2 className="text-lg font-semibold">{t('session.history')}</h2>
        <Timeline
          queryKey={recordsKeys.detail(CONTACT, item.contact_id)}
          fetchPage={recordTimelineSource(CONTACT, item.contact_id).fetchPage}
        />
      </section>
    </>
  )
}

function QueueList({
  session,
  disabled,
  onPick,
}: {
  session: CallSession
  disabled: boolean
  onPick: (position: number) => void
}) {
  const { t } = useTranslation('calls')
  const items = useQuery({
    queryKey: callsKeys.sessionItems(session.id),
    queryFn: () => callsApi.sessionItems(session.id),
  })
  return (
    <aside className="grid h-fit gap-2">
      <h2 className="text-sm font-semibold">{t('session.queue')}</h2>
      {items.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : items.isError ? (
        <ErrorState onRetry={() => void items.refetch()} />
      ) : (
        <ol className="grid max-h-[32rem] gap-1 overflow-y-auto rounded-md border p-1">
          {items.data.data.map((entry) => {
            const current = entry.position === session.position && session.status === 'active'
            return (
              <li key={entry.position}>
                <button
                  type="button"
                  disabled={disabled || current}
                  onClick={() => onPick(entry.position)}
                  aria-current={current ? 'step' : undefined}
                  className={cn(
                    'flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-muted disabled:cursor-default',
                    current && 'bg-muted font-medium',
                  )}
                >
                  {entry.status === 'logged' ? (
                    <CheckCircle2 className="size-4 shrink-0 text-success" aria-hidden="true" />
                  ) : entry.status === 'skipped' ? (
                    <SkipForward className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                  ) : (
                    <Circle className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                  )}
                  <span className="min-w-0 flex-1 truncate">{entry.contact_name ?? t('session.unknownContact')}</span>
                  <span className="sr-only">{t(`session.itemStatus.${entry.status}`)}</span>
                </button>
              </li>
            )
          })}
        </ol>
      )}
    </aside>
  )
}
