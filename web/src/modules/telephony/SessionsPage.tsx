import { useQuery } from '@tanstack/react-query'
import { PhoneCall, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { notifyError } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Textarea } from '@/components/ui/textarea'
import { formatDateTime, ListSkeleton, NativeSelect, useRecordFormatting, useSavedViews } from '@/sdk'
import { callsApi, callsKeys } from './api'
import { callSessionPath } from './constants'
import { callErrorKey } from './lib/errors'

const CONTACT = 'people.person'

export function SessionsPage() {
  const { t } = useTranslation('telephony')
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const [starting, setStarting] = useState(false)
  const canStart = hasPermission('tasks.create')
  const sessions = useQuery({ queryKey: callsKeys.sessions(), queryFn: () => callsApi.sessions() })

  const startButton = canStart && (
    <Button onClick={() => setStarting(true)}>
      <Plus aria-hidden="true" />
      {t('sessions.new')}
    </Button>
  )

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} actions={startButton} />
      {sessions.isPending ? (
        <ListSkeleton rows={3} />
      ) : sessions.isError ? (
        <ErrorState onRetry={() => void sessions.refetch()} />
      ) : sessions.data.data.length === 0 ? (
        <EmptyState
          icon={PhoneCall}
          title={t('sessions.emptyTitle')}
          description={t('sessions.emptyDescription')}
          action={startButton}
        />
      ) : (
        <Table>
          <caption className="sr-only">{t('sessions.title')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('sessions.columns.name')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('sessions.columns.startedAt')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('sessions.columns.people')}</TableHead>
              <TableHead scope="col">{t('sessions.columns.status')}</TableHead>
              <TableHead scope="col" className="text-right">
                <span className="sr-only">{t('sessions.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sessions.data.data.map((session) => (
              <TableRow key={session.id}>
                <TableCell className="max-w-64">
                  <Link to={callSessionPath(session.id)} className="block truncate font-medium underline-offset-4 hover:underline">
                    {session.name}
                  </Link>
                </TableCell>
                <TableCell className="hidden tabular-nums sm:table-cell">{formatDateTime(session.created_at, formatting)}</TableCell>
                <TableCell className="hidden tabular-nums md:table-cell">{session.total}</TableCell>
                <TableCell>
                  <Badge variant={session.status === 'completed' ? 'secondary' : 'success'}>
                    {t(`sessions.status.${session.status}`)}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <Button asChild size="sm" variant="outline">
                    <Link to={callSessionPath(session.id)}>
                      {t(session.status === 'completed' ? 'sessions.view' : 'sessions.continue')}
                    </Link>
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      <Dialog open={starting} onOpenChange={setStarting}>
        <DialogContent>{starting && <StartSessionForm onClose={() => setStarting(false)} />}</DialogContent>
      </Dialog>
    </Page>
  )
}

function viewScript(settings: unknown): string {
  if (settings && typeof settings === 'object' && 'call_script' in settings) {
    const script = (settings as Record<string, unknown>).call_script
    if (typeof script === 'string') return script
  }
  return ''
}

function StartSessionForm({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('telephony')
  const navigate = useNavigate()
  const views = useSavedViews(CONTACT)
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
      void navigate(callSessionPath(session.id))
    } catch (error) {
      notifyError(error, { title: t(callErrorKey(error)) })
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogForm
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={t('start.title')}
      description={t('start.description')}
      cancelLabel={t('form.cancel')}
      submitLabel={t('start.submit')}
      onCancel={onClose}
      pending={saving}
      submitDisabled={saving}
    >
      <Field label={t('start.view')} description={t('start.viewHint')} error={viewError ? t('start.viewRequired') : undefined}>
        {(control) =>
          views.isPending ? (
            <Skeleton className="h-9 w-full" />
          ) : views.isError ? (
            <ErrorState onRetry={() => void views.refetch()} />
          ) : (
            <NativeSelect
              {...control}
              value={viewId}
              onChange={(event) => {
                setViewId(event.target.value)
                setScript(null)
                setViewError(false)
              }}
            >
              <option value="">{t('start.pickView')}</option>
              {(views.data ?? []).map((view) => (
                <option key={view.id} value={view.id}>
                  {view.name}
                </option>
              ))}
            </NativeSelect>
          )
        }
      </Field>
      <Field label={t('start.name')}>
        {(control) => (
          <Input {...control} value={name} placeholder={selected?.name ?? ''} onChange={(event) => setName(event.target.value)} />
        )}
      </Field>
      <Field label={t('start.script')}>
        {(control) => (
          <Textarea
            {...control}
            rows={5}
            value={scriptValue}
            placeholder={t('start.scriptPlaceholder')}
            onChange={(event) => setScript(event.target.value)}
          />
        )}
      </Field>
    </DialogForm>
  )
}
