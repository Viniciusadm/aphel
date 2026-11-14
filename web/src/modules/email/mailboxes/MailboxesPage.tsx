import { AlertCircle, Inbox, Mail, Pencil, PlugZap, RefreshCw, Server, Trash2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { Notice } from '@/components/ui/notice'
import { useSession } from '@/app/session/session-context'
import { SETTINGS_MANAGE } from '@/sdk'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import {emailApi, useInvalidateMailboxes, useMailboxProviders, useMailboxes} from '@/sdk'
import { MailboxStatusBadge } from '../components/MailboxStatusBadge'
import { TemplatesPanel } from '../components/TemplatesPanel'
import {VisibilityBadge} from '@/sdk'
import {notifyEmailError, oauthErrorMessage} from '@/sdk'
import type {Mailbox, MailboxProviderInfo} from '@/sdk'
import { MailboxDialog, type MailboxDialogMode } from './MailboxDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Alert } from '@/components/ui/alert'


function useOAuthReturn() {
  const { t } = useTranslation('email')
  const [searchParams, setSearchParams] = useSearchParams()
  const handled = useRef(false)
  const providers = useMailboxProviders()
  const connected = searchParams.get('connected')
  const error = searchParams.get('error')

  useEffect(() => {
    if (handled.current || (!connected && !error)) return
    handled.current = true
    if (connected) {
      const label = providers.data?.find((provider) => provider.key === connected)?.label ?? connected
      notifySuccess(t('mailboxes.oauthConnected', { provider: label }))
    }
    if (error) toast({ title: t('mailboxes.oauthFailed'), description: oauthErrorMessage(error), variant: 'destructive' })
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.delete('connected')
        next.delete('error')
        return next
      },
      { replace: false },
    )
  }, [connected, error, providers.data, setSearchParams, t])
}

export function MailboxesPage() {
  const { t } = useTranslation('email')
  useOAuthReturn()
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)

  return (
    <Page>
      <PageHeader title={t('mailboxes.title')} description={t('mailboxes.description')} />
      <Notice>
        {canManage ? t('mailboxes.scopeAdmin') : t('mailboxes.scopeMember')}
        {canManage && (
          <>
            {' '}
            <Link to="/settings/email" className="font-medium underline underline-offset-4">
              {t('mailboxes.teamLink')}
            </Link>
          </>
        )}
      </Notice>
      <SectionWorkspace defaultValue="mailboxes">
        <SectionIndex>
          <SectionLink value="mailboxes">{t('mailboxes.tabs.mailboxes')}</SectionLink>
          <SectionLink value="templates">{t('mailboxes.tabs.templates')}</SectionLink>
        </SectionIndex>
        <SectionContent value="mailboxes">
          <MailboxesSection />
        </SectionContent>
        <SectionContent value="templates">
          <TemplatesPanel scope="personal" />
        </SectionContent>
      </SectionWorkspace>
    </Page>
  )
}

function MailboxesSection() {
  const { t } = useTranslation('email')
  const mailboxes = useMailboxes()
  const [dialog, setDialog] = useState<MailboxDialogMode | null>(null)
  const [removing, setRemoving] = useState<Mailbox | null>(null)
  const invalidate = useInvalidateMailboxes()

  return (
    <div className="grid gap-6">
      <ConnectProviders
        onCredentials={(provider) => setDialog({ kind: 'create', provider: provider.key, providerLabel: provider.label })}
      />
      <div className="grid gap-3">
        <h3 className="text-base font-semibold">{t('mailboxes.connectedTitle')}</h3>
        {mailboxes.isPending && (
          <div className="grid gap-2" aria-busy="true">
            <Skeleton className="h-28 w-full" />
            <Skeleton className="h-28 w-full" />
          </div>
        )}
        {mailboxes.isError && <ErrorState onRetry={() => void mailboxes.refetch()} />}
        {mailboxes.isSuccess && mailboxes.data.length === 0 && (
          <EmptyState icon={Inbox} title={t('mailboxes.emptyTitle')} description={t('mailboxes.emptyDescription')} />
        )}
        {mailboxes.isSuccess && mailboxes.data.length > 0 && (
          <ul className="grid gap-3">
            {mailboxes.data.map((mailbox) => (
              <MailboxCard
                key={mailbox.id}
                mailbox={mailbox}
                onEdit={() => setDialog({ kind: 'edit', mailbox })}
                onRemove={() => setRemoving(mailbox)}
              />
            ))}
          </ul>
        )}
      </div>
      <MailboxDialog mode={dialog} onOpenChange={(open) => !open && setDialog(null)} />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        variant="destructive"
        title={t('mailboxes.removeTitle', { address: removing?.address ?? '' })}
        description={t('mailboxes.removeDescription')}
        confirmLabel={t('mailboxes.removeConfirm')}
        notifyOnError={false}
        onConfirm={async () => {
          if (!removing) return
          try {
            await emailApi.removeMailbox(removing.id)
          } catch (error) {
            notifyEmailError(error)
            throw error
          }
          await invalidate()
          notifySuccess(t('mailboxes.removed'))
        }}
      />
    </div>
  )
}

function ConnectProviders({ onCredentials }: { onCredentials: (provider: MailboxProviderInfo) => void }) {
  const { t } = useTranslation('email')
  const providers = useMailboxProviders()

  return (
    <div className="grid gap-3 rounded-lg border p-4">
      <div className="grid gap-1">
        <h3 className="text-base font-semibold">{t('mailboxes.connectSection')}</h3>
        <p className="text-sm text-muted-foreground">{t('mailboxes.connectHint')}</p>
      </div>
      {providers.isPending && <Skeleton className="h-9 w-full sm:w-64" />}
      {providers.isError && <ErrorState onRetry={() => void providers.refetch()} />}
      {providers.isSuccess && providers.data.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('mailboxes.noProviders')}</p>
      )}
      {providers.isSuccess && providers.data.length > 0 && (
        <div className="flex flex-col gap-2 sm:flex-row sm:flex-wrap">
          {providers.data.map((provider) =>
            provider.connection.kind === 'oauth' ? (
              <Button key={provider.key} variant="outline" asChild>
                <a href={provider.connection.start_path}>
                  <PlugZap aria-hidden="true" />
                  {t('mailboxes.connectWith', { provider: provider.label })}
                </a>
              </Button>
            ) : (
              <Button key={provider.key} variant="outline" onClick={() => onCredentials(provider)}>
                <Server aria-hidden="true" />
                {t('mailboxes.connectWith', { provider: provider.label })}
              </Button>
            ),
          )}
        </div>
      )}
    </div>
  )
}

interface MailboxCardProps {
  mailbox: Mailbox
  onEdit: () => void
  onRemove: () => void
}

function MailboxCard({ mailbox, onEdit, onRemove }: MailboxCardProps) {
  const { t } = useTranslation('email')
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateMailboxes()
  const [busy, setBusy] = useState<'test' | 'sync' | null>(null)

  const test = async () => {
    setBusy('test')
    try {
      const result = await emailApi.testMailbox(mailbox.id)
      if (result.status === 'error') {
        toast({
          title: t('mailboxes.testFailed'),
          description: result.status_reason ?? undefined,
          variant: 'destructive',
        })
      } else {
        notifySuccess(t('mailboxes.testSucceeded'))
      }
    } catch (error) {
      notifyEmailError(error)
    } finally {
      setBusy(null)
      await invalidate()
    }
  }

  const sync = async () => {
    setBusy('sync')
    try {
      await emailApi.syncMailbox(mailbox.id)
      notifySuccess(t('mailboxes.syncQueued'))
    } catch (error) {
      notifyEmailError(error)
    } finally {
      setBusy(null)
      await invalidate()
    }
  }

  return (
    <li className="grid gap-3 rounded-lg border p-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex min-w-0 items-start gap-3">
          <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
            <Mail className="size-4" aria-hidden="true" />
          </span>
          <div className="grid min-w-0 gap-0.5">
            <p className="font-medium break-all">{mailbox.display_name || mailbox.address}</p>
            {mailbox.display_name && <p className="text-sm break-all text-muted-foreground">{mailbox.address}</p>}
            <p className="text-xs text-muted-foreground">{mailbox.provider_label}</p>
          </div>
        </div>
        <div className="flex flex-wrap gap-1.5">
          <MailboxStatusBadge status={mailbox.status} />
          <VisibilityBadge visibility={mailbox.default_visibility} />
          {mailbox.credentials_configured && <Badge variant="secondary">{t('mailboxes.credentialsConfigured')}</Badge>}
        </div>
      </div>
      {mailbox.status === 'error' && mailbox.status_reason && (
        <Alert className="flex items-start gap-2 text-destructive">
          <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <span className="break-words">{mailbox.status_reason}</span>
        </Alert>
      )}
      <dl className="grid gap-x-6 gap-y-1 text-sm sm:grid-cols-3">
        <div className="grid">
          <dt className="text-xs text-muted-foreground">{t('mailboxes.lastSynced')}</dt>
          <dd>{mailbox.last_synced_at ? formatDateTime(mailbox.last_synced_at, formatting) : t('mailboxes.never')}</dd>
        </div>
        <div className="grid">
          <dt className="text-xs text-muted-foreground">{t('mailboxes.historyDays')}</dt>
          <dd>{t('mailboxes.days', { count: mailbox.history_days })}</dd>
        </div>
        {mailbox.status === 'syncing' && mailbox.sync_started_at && (
          <div className="grid">
            <dt className="text-xs text-muted-foreground">{t('mailboxes.syncStarted')}</dt>
            <dd>{formatDateTime(mailbox.sync_started_at, formatting)}</dd>
          </div>
        )}
      </dl>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" size="sm" onClick={() => void test()} disabled={busy !== null}>
          <PlugZap aria-hidden="true" className={busy === 'test' ? 'animate-pulse' : undefined} />
          {t('mailboxes.test')}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void sync()}
          disabled={busy !== null || mailbox.status === 'syncing'}
        >
          <RefreshCw aria-hidden="true" className={busy === 'sync' ? 'animate-spin' : undefined} />
          {t('mailboxes.syncNow')}
        </Button>
        <Button variant="outline" size="sm" onClick={onEdit}>
          <Pencil aria-hidden="true" />
          {t('actions.edit')}
        </Button>
        <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive" onClick={onRemove}>
          <Trash2 aria-hidden="true" />
          {t('actions.remove')}
        </Button>
      </div>
    </li>
  )
}
