import { KeyRound } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import {
  apiTokensApi,
  tokenStatus,
  useApiTokens,
  useInvalidateApiTokens,
  type ApiToken,
} from '@/features/settings/api-tokens/api'
import { SectionHeader } from '@/features/settings/components/SectionHeader'

export function ApiTokensSection() {
  const { t } = useTranslation('security')
  const tokens = useApiTokens('mine')
  const invalidate = useInvalidateApiTokens()
  const [revoking, setRevoking] = useState<ApiToken | null>(null)
  const activeTokens = tokens.data?.filter((token) => tokenStatus(token) === 'active') ?? []

  const revoke = async (token: ApiToken) => {
    await apiTokensApi.revoke(token.id)
    await invalidate()
    notifySuccess(t('apiTokens.revoked', { name: token.name }))
  }

  return (
    <section className="flex flex-col gap-4">
      <SectionHeader
        title={t('apiTokens.title')}
        description={t('apiTokens.description')}
        action={
          <Button asChild size="sm" variant="outline">
            <Link to="/account/api-tokens">{t('apiTokens.manage')}</Link>
          </Button>
        }
      />
      {tokens.isPending ? (
        <Skeleton className="h-14 w-full" aria-busy="true" />
      ) : tokens.isError ? (
        <ErrorState onRetry={() => void tokens.refetch()} />
      ) : activeTokens.length === 0 ? (
        <EmptyState icon={KeyRound} title={t('apiTokens.empty')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('apiTokens.listLabel')}>
          {activeTokens.map((token) => (
            <TokenRow key={token.id} token={token} onRevoke={() => setRevoking(token)} />
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={revoking !== null}
        onOpenChange={(open) => !open && setRevoking(null)}
        title={t('apiTokens.revokeTitle')}
        description={t('apiTokens.revokeDescription', { name: revoking?.name ?? '' })}
        confirmLabel={t('apiTokens.revokeConfirm')}
        variant="destructive"
        onConfirm={() => (revoking ? revoke(revoking) : undefined)}
      />
    </section>
  )
}

function TokenRow({ token, onRevoke }: { token: ApiToken; onRevoke: () => void }) {
  const { t } = useTranslation('security')
  const formatting = useRecordFormatting()

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-words">{token.name}</span>
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{token.token_prefix}…</code>
        </div>
        <p className="text-xs text-muted-foreground">
          {token.last_used_at
            ? t('apiTokens.lastUsed', { date: formatDateTime(token.last_used_at, formatting) })
            : t('apiTokens.neverUsed')}
          {' · '}
          {token.expires_at
            ? t('apiTokens.expiresAt', { date: formatDateTime(token.expires_at, formatting) })
            : t('apiTokens.neverExpires')}
        </p>
      </div>
      <Button
        size="sm"
        variant="outline"
        className="self-start sm:self-center"
        aria-label={t('apiTokens.revokeLabel', { name: token.name })}
        onClick={onRevoke}
      >
        {t('apiTokens.revoke')}
      </Button>
    </li>
  )
}
