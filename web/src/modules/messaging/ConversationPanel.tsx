import { NativeSelect } from '@/components/ui/native-select'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { AlertCircle, ArrowLeft, Check, CheckCheck, Clock, Link2, Loader2, Send } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { UserFieldInput } from '@/components/fields'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { formatDateTime, useOrganizationUsers, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { Slot } from '@/kernel/modules/Slot'
import { chordText, useActionKeys, useKeybindingProps } from '@/kernel/keybindings'
import {
  CONVERSATION_STATUSES,
  MESSAGING_KEY,
  REPLY_PERMISSION,
  messagingApi,
  messagingKeys,
  peerLabel,
  useConversation,
  type Conversation,
  type ConversationStatus,
  type Message,
} from './api'
import { KNOWN_ERRORS, errorText } from './errors'
import { LinkDialog } from './LinkDialog'
import { ChannelIcon, LinkedRecordLabel, StatusBadge } from './shared'
import { Field } from '@/components/ui/field'

export function ConversationPanel({ conversationId, onBack }: { conversationId: string; onBack: () => void }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const queryClient = useQueryClient()
  const detail = useConversation(conversationId)
  const markedRead = useRef(false)
  const unread = detail.data?.conversation.unread_count ?? 0

  useEffect(() => {
    if (unread === 0 || markedRead.current) return
    markedRead.current = true
    void messagingApi
      .markRead(conversationId)
      .then(() => queryClient.invalidateQueries({ queryKey: messagingKeys.conversations() }))
      .catch(() => undefined)
  }, [unread, conversationId, queryClient])

  if (detail.isPending) {
    return (
      <div className="grid w-full gap-3 p-4" aria-busy="true">
        <Skeleton className="h-10 w-1/2" />
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
    )
  }
  if (detail.isError) {
    return (
      <div className="w-full p-4">
        <ErrorState onRetry={() => void detail.refetch()} />
      </div>
    )
  }
  const { conversation, messages, total_messages } = detail.data
  return (
    <div className="flex min-h-0 w-full flex-col">
      <ConversationHeader conversation={conversation} onBack={onBack} />
      <MessageList messages={messages} hiddenCount={Math.max(0, total_messages - messages.length)} />
      <Composer conversation={conversation} />
      <span className="sr-only">{t('panel.title', { name: peerLabel(conversation) })}</span>
    </div>
  )
}

function ConversationHeader({ conversation, onBack }: { conversation: Conversation; onBack: () => void }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const queryClient = useQueryClient()
  const { hasPermission } = useSession()
  const canReply = hasPermission(REPLY_PERMISSION)
  const [linking, setLinking] = useState(false)
  const [busy, setBusy] = useState(false)

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true)
    try {
      await action()
      notifySuccess(success)
      await queryClient.invalidateQueries({ queryKey: messagingKeys.conversations() })
    } catch (error) {
      const known = errorText(t, error)
      if (known) notifyError(error, { title: known })
      else notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  return (
    <header className="grid gap-2 border-b bg-muted/30 p-3">
      <div className="flex min-w-0 items-start gap-2">
        <Button variant="ghost" size="icon" className="md:hidden" onClick={onBack} aria-label={t('panel.back')}>
          <ArrowLeft />
        </Button>
        <Avatar name={peerLabel(conversation)} size="lg" className="size-10 text-sm" />
        <div className="grid min-w-0 flex-1 gap-0.5">
          <h2 className="truncate text-base font-semibold">{peerLabel(conversation)}</h2>
          <p className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
            <ChannelIcon kind={conversation.channel.kind} className="size-3.5" />
            <span className="truncate">
              {conversation.peer} · {conversation.channel.name}
            </span>
          </p>
          {conversation.subject && <p className="truncate text-sm">{conversation.subject}</p>}
        </div>
        <StatusBadge status={conversation.status} />
      </div>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
        <span className="flex min-w-0 items-center gap-1">
          <span className="text-muted-foreground">{t('links.contact')}:</span>
          {conversation.contact ? (
            <LinkedRecordLabel recordType="people.person" record={conversation.contact} />
          ) : (
            <span className="text-muted-foreground">{t('links.none')}</span>
          )}
        </span>
        <span className="flex min-w-0 items-center gap-1">
          <span className="text-muted-foreground">{t('links.deal')}:</span>
          {conversation.deal ? (
            <LinkedRecordLabel recordType="deals.deal" record={conversation.deal} />
          ) : (
            <span className="text-muted-foreground">{t('links.none')}</span>
          )}
        </span>
        {canReply && (
          <Button size="sm" variant="ghost" onClick={() => setLinking(true)}>
            <Link2 aria-hidden="true" />
            {t('links.edit')}
          </Button>
        )}
      </div>
      {canReply && (
        <div className="grid gap-2 sm:grid-cols-2">
          <Field label={t('panel.status')}>
            {(fieldControl) => <NativeSelect value={conversation.status}
              disabled={busy}
              onValueChange={(value) =>
                void run(
                  () => messagingApi.setStatus(conversation.id, value as ConversationStatus),
                  t('panel.statusChanged', { status: t(`status.${value}`) }),
                )
              } id={fieldControl.id}>
                {CONVERSATION_STATUSES.map((status) => (
                  <option key={status} value={status}>
                    {t(`status.${status}`)}
                  </option>
                ))}
              </NativeSelect>}
          </Field>
          <Field label={t('panel.assignee')}>
            {(fieldControl) => <UserFieldInput
              id={fieldControl.id}
              value={conversation.assignee_user_id ?? null}
              disabled={busy}
              onChange={(userId) =>
                void run(
                  () => messagingApi.assign(conversation.id, userId),
                  userId ? t('panel.assigned') : t('panel.unassigned'),
                )
              }
            />}
          </Field>
        </div>
      )}
      {canReply && <LinkDialog conversation={conversation} open={linking} onOpenChange={setLinking} />}
    </header>
  )
}

function StatusIcon({ message }: { message: Message }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const label = t(`messageStatus.${message.status}`)
  const className = 'size-3.5'
  switch (message.status) {
    case 'queued':
    case 'sending':
      return <Clock className={className} aria-label={label} />
    case 'sent':
      return <Check className={className} aria-label={label} />
    case 'delivered':
      return <CheckCheck className={className} aria-label={label} />
    case 'read':
      return <CheckCheck className={cn(className, 'text-primary')} aria-label={label} />
    case 'failed':
      return <AlertCircle className={cn(className, 'text-destructive')} aria-label={label} />
    default:
      return null
  }
}

function MessageBubble({ message }: { message: Message }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  const outgoing = message.direction === 'out'
  const author = outgoing
    ? (users.data?.find((user) => user.id === message.sent_by_user_id)?.display_name ?? t('panel.team'))
    : message.sender_name || message.sender
  const placeholder =
    message.body.trim() === '' ? t(`messageTypes.${message.message_type}`, { defaultValue: t('messageTypes.other') }) : null
  return (
    <li className={cn('flex', outgoing ? 'justify-end' : 'justify-start')}>
      <div
        className={cn(
          'grid max-w-[85%] gap-1 rounded-lg border px-3 py-2 text-sm sm:max-w-[75%]',
          outgoing ? 'border-primary/20 bg-primary/10' : 'bg-card shadow-xs',
        )}
      >
        <span className="text-xs font-medium text-muted-foreground">{author}</span>
        {message.subject && <span className="text-xs font-semibold">{message.subject}</span>}
        {placeholder ? (
          <span className="text-muted-foreground italic">{placeholder}</span>
        ) : (
          <p className="break-words whitespace-pre-wrap">{message.body}</p>
        )}
        <span className="flex items-center justify-end gap-1 text-[11px] text-muted-foreground">
          {formatDateTime(message.created_at, formatting)}
          {outgoing && <StatusIcon message={message} />}
        </span>
        {message.status === 'failed' && (
          <span className="text-xs text-destructive">
            {message.error_code && KNOWN_ERRORS.includes(message.error_code)
              ? t(`errors.${message.error_code}`)
              : t('panel.sendFailed', { reason: message.error_message ?? message.error_code ?? '' })}
          </span>
        )}
      </div>
    </li>
  )
}

function MessageList({ messages, hiddenCount }: { messages: Message[]; hiddenCount: number }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const end = useRef<HTMLDivElement>(null)
  const last = messages.at(-1)?.id

  useEffect(() => {
    end.current?.scrollIntoView({ block: 'end' })
  }, [last])

  return (
    <div className="min-h-[240px] flex-1 overflow-y-auto p-3" aria-live="polite">
      {hiddenCount > 0 && (
        <p className="mb-2 text-center text-xs text-muted-foreground">{t('panel.olderHidden', { count: hiddenCount })}</p>
      )}
      <ul className="grid gap-2" aria-label={t('panel.messages')}>
        {messages.map((message) => (
          <MessageBubble key={message.id} message={message} />
        ))}
      </ul>
      <div ref={end} />
    </div>
  )
}

function Composer({ conversation }: { conversation: Conversation }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const formatting = useRecordFormatting()
  const queryClient = useQueryClient()
  const { hasPermission } = useSession()
  const [value, setValue] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 10_000)
    return () => clearInterval(timer)
  }, [])

  const send = async () => {
    if (!value.trim()) return
    setSending(true)
    setError(null)
    try {
      await messagingApi.reply(conversation.id, value)
      setValue('')
      await queryClient.invalidateQueries({ queryKey: messagingKeys.conversations() })
    } catch (caught) {
      const known = errorText(t, caught)
      setError(known)
      if (!known) notifyError(caught)
    } finally {
      setSending(false)
    }
  }

  const sendKeys = useKeybindingProps<HTMLTextAreaElement>({ 'messaging.send': () => void send() })
  const sendChord = useActionKeys('messaging.send')[0]

  if (!hasPermission(REPLY_PERMISSION)) {
    return <p className="border-t p-3 text-sm text-muted-foreground">{t('composer.readOnly')}</p>
  }
  if (!conversation.channel.is_active || !conversation.channel.available) {
    return <p className="border-t p-3 text-sm text-muted-foreground">{t('composer.channelInactive')}</p>
  }
  const windowEnd = conversation.reply_window_ends_at ? Date.parse(conversation.reply_window_ends_at) : null
  const windowClosed = conversation.channel.reply_window_seconds !== null && (windowEnd === null || windowEnd < now)
  if (windowClosed) {
    return <p className="border-t p-3 text-sm text-muted-foreground">{t('composer.windowClosed')}</p>
  }
  const contact = conversation.contact?.accessible ? conversation.contact : null

  return (
    <form
      className="grid gap-2 border-t p-3"
      onSubmit={(event) => {
        event.preventDefault()
        void send()
      }}
    >
      <Field id={`reply-${conversation.id}`} label={t('composer.label')} hideLabel>
      <Textarea
        id={`reply-${conversation.id}`}
        value={value}
        rows={3}
        maxLength={conversation.channel.max_body_chars}
        placeholder={
          sendChord
            ? t(conversation.channel.uses_references ? 'composer.emailPlaceholder' : 'composer.placeholder', { keys: chordText(sendChord) })
            : t(conversation.channel.uses_references ? 'composer.emailPlaceholderPlain' : 'composer.placeholderPlain')
        }
        onChange={(event) => setValue(event.target.value)}
        onKeyDown={sendKeys.onKeyDown}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? `reply-error-${conversation.id}` : undefined}
      />
      {error && (
        <SharedFieldError id={`reply-error-${conversation.id}`}>
          {error}
        </SharedFieldError>
      )}
      </Field>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-1">
          <Slot
            name="composer.tools"
            props={{
              context: conversation.channel.uses_references ? 'email' : 'message',
              subject: contact ? { recordType: 'people.person', recordId: contact.id } : undefined,
              value,
              onChange: setValue,
            }}
          />
        </div>
        <div className="ml-auto flex items-center gap-2">
          {windowEnd !== null && (
            <span className="hidden text-xs text-muted-foreground sm:inline">
              {t('composer.windowUntil', { date: formatDateTime(new Date(windowEnd).toISOString(), formatting) })}
            </span>
          )}
          <Button type="submit" size="sm" disabled={sending || !value.trim()}>
            {sending ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Send aria-hidden="true" />}
            {t('composer.send')}
          </Button>
        </div>
      </div>
    </form>
  )
}
