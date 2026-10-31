import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Download, FileSignature, Loader2, Plus, Trash2, UserPlus } from 'lucide-react'
import { useState } from 'react'
import { api, isApiError, type components } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { fileDownloadUrl, useAttachments } from '@/features/files'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import {
  ESIGN_KEY,
  SEND_PERMISSION,
  esignApi,
  esignKeys,
  useEsignRequests,
  useEsignStatus,
  useInvalidateEsign,
  type EsignRequest,
  type EsignRequestStatus,
  type EsignSignerInput,
  type EsignSignerStatus,
} from './api'
import { Field } from '@/components/ui/field'

type Deal = components['schemas']['DealResponse']
type Contact = components['schemas']['ContactResponse']

const KNOWN_ERRORS = [
  'esign_not_configured',
  'esign_file_not_attached',
  'esign_file_not_pdf',
  'esign_file_too_large',
  'esign_provider_rejected',
  'esign_provider_unauthorized',
  'esign_provider_unavailable',
  'esign_request_closed',
  'deal_not_found',
  'permission_denied',
]

const REQUEST_VARIANTS: Record<EsignRequestStatus, BadgeProps['variant']> = {
  queued: 'outline',
  sending: 'secondary',
  cancel_pending: 'warning',
  cancelling: 'warning',
  failed: 'destructive',
  delivery_unknown: 'warning',
  sent: 'secondary',
  completed: 'success',
  refused: 'destructive',
  cancelled: 'outline',
  expired: 'warning',
}

const SIGNER_VARIANTS: Record<EsignSignerStatus, BadgeProps['variant']> = {
  pending: 'outline',
  signed: 'success',
  refused: 'destructive',
}

function notifyKnown(error: unknown, translate: (key: string) => string) {
  if (isApiError(error) && KNOWN_ERRORS.includes(error.code)) {
    notifyError(error, { title: translate(`errors.${error.code}`) })
  } else {
    notifyError(error)
  }
}

export function DealSignaturesTab({ recordId, readOnly }: RecordTabRenderProps) {
  const t = useModuleTranslation(ESIGN_KEY)
  const { hasPermission } = useSession()
  const requests = useEsignRequests(recordId)
  const status = useEsignStatus()
  const [sending, setSending] = useState(false)
  const canSend = !readOnly && hasPermission(SEND_PERMISSION)
  const configured = status.data?.configured ?? true

  const sendButton = canSend ? (
    <Button size="sm" onClick={() => setSending(true)} disabled={!configured}>
      <Plus aria-hidden="true" />
      {t('deal.send')}
    </Button>
  ) : undefined

  return (
    <section className="flex flex-col gap-3" aria-label={t('deal.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-0.5">
          <h2 className="text-base font-semibold">{t('deal.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('deal.description')}</p>
        </div>
        {sendButton}
      </div>
      <p className="rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground">{t('legalNotice')}</p>
      {status.data && !status.data.configured && (
        <p className="rounded-md border border-warning/50 bg-warning/10 px-3 py-2 text-sm">{t('deal.notConfigured')}</p>
      )}
      {status.data?.environment === 'sandbox' && (
        <p className="text-xs text-muted-foreground">{t('deal.sandbox')}</p>
      )}
      {requests.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : requests.isError ? (
        <ErrorState onRetry={() => void requests.refetch()} />
      ) : requests.data.length === 0 ? (
        <EmptyState
          icon={FileSignature}
          title={t('deal.emptyTitle')}
          description={t('deal.emptyDescription')}
          action={sendButton}
        />
      ) : (
        <ul className="grid gap-2">
          {requests.data.map((request) => (
            <RequestRow key={request.id} request={request} canCancel={canSend} />
          ))}
        </ul>
      )}
      {canSend && <SendDialog dealId={recordId} open={sending} onOpenChange={setSending} />}
    </section>
  )
}

function RequestRow({ request, canCancel }: { request: EsignRequest; canCancel: boolean }) {
  const t = useModuleTranslation(ESIGN_KEY)
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateEsign()
  const [confirming, setConfirming] = useState(false)

  const cancel = async () => {
    try {
      await esignApi.cancel(request.id)
      notifySuccess(t('deal.cancelled'))
      await invalidate()
    } catch (error) {
      notifyKnown(error, t)
      throw error
    }
  }

  return (
    <li className="grid gap-2 rounded-md border bg-card px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <FileSignature className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <p className="min-w-0 flex-1 truncate text-sm font-medium">{request.file_name}</p>
        <Badge variant={REQUEST_VARIANTS[request.status]}>{t(`status.${request.status}`)}</Badge>
      </div>
      {request.status === 'delivery_unknown' && <p className="text-xs text-muted-foreground">{t('deal.deliveryUnknown')}</p>}
      <p className="text-xs text-muted-foreground">
        {t('deal.sentAt', { date: formatDateTime(request.created_at, formatting) })}
        {request.completed_at && ` · ${t('deal.completedAt', { date: formatDateTime(request.completed_at, formatting) })}`}
      </p>
      <ul className="grid gap-1" aria-label={t('deal.signers')}>
        {request.signers.map((signer) => (
          <li key={signer.id} className="flex flex-wrap items-center gap-2 text-sm">
            <span className="min-w-0 truncate">{signer.name}</span>
            <span className="min-w-0 truncate text-xs text-muted-foreground">{signer.email}</span>
            <Badge variant={SIGNER_VARIANTS[signer.status]} className="ml-auto">
              {t(`signerStatus.${signer.status}`)}
            </Badge>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap justify-end gap-2">
        <Button asChild size="sm" variant="ghost">
          <a href={fileDownloadUrl(request.file_id)} download={request.file_name}>
            <Download aria-hidden="true" />
            {t('deal.original')}
          </a>
        </Button>
        {request.signed_file_id && (
          <Button asChild size="sm" variant="outline">
            <a href={fileDownloadUrl(request.signed_file_id)}>
              <Download aria-hidden="true" />
              {t('deal.signedFile')}
            </a>
          </Button>
        )}
        {canCancel && ['sent', 'queued'].includes(request.status) && (
          <Button size="sm" variant="outline" onClick={() => setConfirming(true)}>
            {t('deal.cancel')}
          </Button>
        )}
      </div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('confirmCancel.title')}
        description={t('confirmCancel.description', { name: request.file_name })}
        confirmLabel={t('confirmCancel.confirm')}
        variant="destructive"
        notifyOnError={false}
        onConfirm={cancel}
      />
    </li>
  )
}

interface SignerDraft extends EsignSignerInput {
  key: number
}

interface SendDialogProps {
  dealId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

function SendDialog({ dealId, open, onOpenChange }: SendDialogProps) {
  const t = useModuleTranslation(ESIGN_KEY)
  const queryClient = useQueryClient()
  const invalidate = useInvalidateEsign()
  const attachments = useAttachments({ recordType: 'deal', recordId: dealId })
  const deal = useQuery({
    queryKey: [...esignKeys.all, 'deal', dealId],
    queryFn: () => api.get<Deal>(`/v1/deals/${encodeURIComponent(dealId)}`),
    enabled: open,
  })
  const contactId = deal.data?.contact_id ?? null
  const contact = useQuery({
    queryKey: [...esignKeys.all, 'contact', contactId],
    queryFn: () => api.get<Contact>(`/v1/contacts/${encodeURIComponent(contactId!)}`),
    enabled: open && Boolean(contactId),
  })
  const [fileId, setFileId] = useState<string | null>(null)
  const [signers, setSigners] = useState<SignerDraft[]>([])
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)
  const [nextKey, setNextKey] = useState(1)

  const pdfs = (attachments.data?.data ?? []).filter(
    (attachment) =>
      attachment.file.mime_type === 'application/pdf' || attachment.file.original_name.toLowerCase().endsWith('.pdf'),
  )
  const chosenFile = fileId ?? pdfs[0]?.file.id ?? null
  const contactSuggestion =
    contact.data?.email && !signers.some((signer) => signer.contact_id === contact.data?.id) ? contact.data : null

  const close = (next: boolean) => {
    if (pending) return
    onOpenChange(next)
    if (!next) {
      setFileId(null)
      setSigners([])
      setMessage('')
    }
  }

  const addSigner = (signer: EsignSignerInput) => {
    setSigners((current) => [...current, { ...signer, key: nextKey }])
    setNextKey((key) => key + 1)
  }

  const updateSigner = (key: number, patch: Partial<EsignSignerInput>) =>
    setSigners((current) => current.map((signer) => (signer.key === key ? { ...signer, ...patch } : signer)))

  const removeSigner = (key: number) => setSigners((current) => current.filter((signer) => signer.key !== key))

  const valid =
    Boolean(chosenFile) &&
    signers.length > 0 &&
    signers.every((signer) => signer.name.trim() && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(signer.email.trim()))

  const send = async () => {
    if (!chosenFile || !valid) return
    setPending(true)
    try {
      await esignApi.create({
        deal_id: dealId,
        file_id: chosenFile,
        signers: signers.map(({ name, email, contact_id }) => ({
          name: name.trim(),
          email: email.trim(),
          contact_id,
        })),
        message: message.trim() || undefined,
      })
      notifySuccess(t('deal.sent'))
      await Promise.all([invalidate(), queryClient.invalidateQueries({ queryKey: ['files'] })])
      setPending(false)
      close(false)
    } catch (error) {
      setPending(false)
      notifyKnown(error, t)
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('dialog.title')}</DialogTitle>
          <DialogDescription>{t('dialog.description')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <Field id="esign-file" label={t('dialog.file')}>
            {attachments.isPending ? (
              <Skeleton className="h-9 w-full" />
            ) : attachments.isError ? (
              <ErrorState onRetry={() => void attachments.refetch()} />
            ) : pdfs.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t('dialog.noPdf')}</p>
            ) : (
              <Select value={chosenFile ?? undefined} onValueChange={setFileId} disabled={pending}>
                <SelectTrigger id="esign-file">
                  <SelectValue placeholder={t('dialog.filePlaceholder')} />
                </SelectTrigger>
                <SelectContent>
                  {pdfs.map((attachment) => (
                    <SelectItem key={attachment.id} value={attachment.file.id}>
                      {attachment.file.original_name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </Field>
          <fieldset className="grid gap-2">
            <legend className="mb-1 text-sm font-medium">{t('dialog.signers')}</legend>
            {signers.length === 0 && <p className="text-sm text-muted-foreground">{t('dialog.noSigners')}</p>}
            {signers.map((signer, index) => (
              <div key={signer.key} className="grid gap-2 rounded-md border p-2 sm:grid-cols-[1fr_1fr_auto]">
                <Input
                  aria-label={t('dialog.signerName', { index: index + 1 })}
                  placeholder={t('dialog.namePlaceholder')}
                  value={signer.name}
                  maxLength={255}
                  disabled={pending}
                  onChange={(event) => updateSigner(signer.key, { name: event.target.value })}
                />
                <Input
                  type="email"
                  aria-label={t('dialog.signerEmail', { index: index + 1 })}
                  placeholder={t('dialog.emailPlaceholder')}
                  value={signer.email}
                  maxLength={254}
                  disabled={pending}
                  onChange={(event) => updateSigner(signer.key, { email: event.target.value })}
                />
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  aria-label={t('dialog.removeSigner', { index: index + 1 })}
                  disabled={pending}
                  onClick={() => removeSigner(signer.key)}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            ))}
            <div className="flex flex-wrap gap-2">
              {contactSuggestion && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  disabled={pending}
                  onClick={() =>
                    addSigner({
                      name: contactSuggestion.name,
                      email: contactSuggestion.email ?? '',
                      contact_id: contactSuggestion.id,
                    })
                  }
                >
                  <UserPlus aria-hidden="true" />
                  {t('dialog.addContact', { name: contactSuggestion.name })}
                </Button>
              )}
              <Button
                type="button"
                size="sm"
                variant="ghost"
                disabled={pending}
                onClick={() => addSigner({ name: '', email: '' })}
              >
                <Plus aria-hidden="true" />
                {t('dialog.addSigner')}
              </Button>
            </div>
          </fieldset>
          <Field label={t('dialog.message')}>
            {(fieldControl) => <Textarea
              {...fieldControl}
              value={message}
              maxLength={1000}
              rows={3}
              disabled={pending}
              placeholder={t('dialog.messagePlaceholder')}
              onChange={(event) => setMessage(event.target.value)}
            />}
          </Field>
          <p className="text-xs text-muted-foreground">{t('dialog.privacy')}</p>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)} disabled={pending}>
            {t('dialog.close')}
          </Button>
          <Button onClick={() => void send()} disabled={!valid || pending}>
            {pending && <Loader2 className="animate-spin" aria-hidden="true" />}
            {pending ? t('dialog.sending') : t('dialog.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
