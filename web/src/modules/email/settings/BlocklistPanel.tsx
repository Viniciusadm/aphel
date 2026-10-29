import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Ban, Loader2, Plus, Trash2 } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { emailApi, emailKeys, useBlocklist } from '../api'
import { isValidAddress } from '../lib/format'
import { BLOCKLIST_KINDS, type BlocklistEntry, type BlocklistKind } from '../types'

const DOMAIN_PATTERN = /^(?!-)[a-z0-9-]+(\.[a-z0-9-]+)+$/i

function normalizeValue(kind: BlocklistKind, value: string): string {
  const trimmed = value.trim().toLowerCase()
  return kind === 'domain' ? trimmed.replace(/^@/, '') : trimmed
}

function isValidValue(kind: BlocklistKind, value: string): boolean {
  return kind === 'domain' ? DOMAIN_PATTERN.test(value) : isValidAddress(value)
}

export function BlocklistPanel() {
  const { t } = useTranslation('email')
  const formId = useId()
  const formatting = useRecordFormatting()
  const queryClient = useQueryClient()
  const blocklist = useBlocklist()
  const [kind, setKind] = useState<BlocklistKind>('domain')
  const [value, setValue] = useState('')
  const [reason, setReason] = useState('')
  const [submitted, setSubmitted] = useState(false)
  const [removing, setRemoving] = useState<BlocklistEntry | null>(null)
  const normalized = normalizeValue(kind, value)
  const valid = isValidValue(kind, normalized)

  const invalidate = () => queryClient.invalidateQueries({ queryKey: emailKeys.blocklist() })
  const add = useMutation({
    mutationFn: () => emailApi.addBlocklistEntry({ kind, value: normalized, reason: reason.trim() || null }),
    onSuccess: async () => {
      await invalidate()
      setValue('')
      setReason('')
      setSubmitted(false)
      notifySuccess(t('blocklist.added'))
    },
    onError: (error) => notifyError(error),
  })

  return (
    <div className="grid gap-4">
      <p className="text-sm text-muted-foreground">{t('blocklist.description')}</p>
      <form
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-[9rem_minmax(0,1fr)_minmax(0,1fr)_auto] sm:items-end"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (valid) add.mutate()
        }}
      >
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-kind`}>{t('blocklist.kind')}</Label>
          <NativeSelect id={`${formId}-kind`} value={kind} onChange={(event) => setKind(event.target.value as BlocklistKind)}>
            {BLOCKLIST_KINDS.map((option) => (
              <option key={option} value={option}>
                {t(`blocklist.kinds.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-value`}>{t('blocklist.value')}</Label>
          <Input
            id={`${formId}-value`}
            value={value}
            autoCapitalize="off"
            spellCheck={false}
            placeholder={t(`blocklist.placeholders.${kind}`)}
            aria-invalid={(submitted && !valid) || undefined}
            onChange={(event) => setValue(event.target.value)}
          />
        </div>
        <div className="grid content-start gap-1.5">
          <Label htmlFor={`${formId}-reason`}>{t('blocklist.reason')}</Label>
          <Input
            id={`${formId}-reason`}
            value={reason}
            maxLength={200}
            placeholder={t('blocklist.reasonPlaceholder')}
            onChange={(event) => setReason(event.target.value)}
          />
        </div>
        <Button type="submit" disabled={add.isPending}>
          {add.isPending ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Plus aria-hidden="true" />}
          {t('blocklist.add')}
        </Button>
        {submitted && !valid && (
          <p className="text-sm text-destructive sm:col-span-4">{t(`blocklist.invalid.${kind}`)}</p>
        )}
      </form>
      {blocklist.isPending && (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      )}
      {blocklist.isError && <ErrorState onRetry={() => void blocklist.refetch()} />}
      {blocklist.isSuccess && blocklist.data.length === 0 && (
        <EmptyState icon={Ban} title={t('blocklist.emptyTitle')} description={t('blocklist.emptyDescription')} />
      )}
      {blocklist.isSuccess && blocklist.data.length > 0 && (
        <ul className="grid divide-y rounded-lg border">
          {blocklist.data.map((entry) => (
            <li key={entry.id} className="flex items-center gap-3 p-3">
              <div className="grid min-w-0 flex-1 gap-0.5">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-sm break-all">{entry.value}</span>
                  <Badge variant="outline">{t(`blocklist.kinds.${entry.kind}`)}</Badge>
                </div>
                <p className="text-xs text-muted-foreground">
                  {entry.reason ? `${entry.reason} · ` : ''}
                  {formatDateTime(entry.created_at, formatting)}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t('blocklist.remove', { value: entry.value })}
                onClick={() => setRemoving(entry)}
              >
                <Trash2 />
              </Button>
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        variant="destructive"
        title={t('blocklist.removeTitle', { value: removing?.value ?? '' })}
        description={t('blocklist.removeDescription')}
        confirmLabel={t('actions.remove')}
        onConfirm={async () => {
          if (!removing) return
          await emailApi.removeBlocklistEntry(removing.id)
          await invalidate()
          notifySuccess(t('blocklist.removed'))
        }}
      />
    </div>
  )
}
