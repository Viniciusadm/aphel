import { Check, Copy, KeyRound, Plus } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
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
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { USERS_MANAGE } from '@/features/settings/lib/permissions'
import {
  MAX_API_TOKEN_NAME_LENGTH,
  apiTokensApi,
  expirationDate,
  groupScopes,
  tokenStatus,
  useApiTokenScopes,
  useApiTokens,
  useInvalidateApiTokens,
  type ApiToken,
  type ApiTokenOwner,
  type CreatedApiToken,
} from './api'
import { ApiTokenLimitsPanel } from './ApiTokenLimitsPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { NativeSelect } from '@/components/ui/native-select'

const EXPIRATION_OPTIONS = ['30', '90', '365', 'never'] as const
type ExpirationOption = (typeof EXPIRATION_OPTIONS)[number]

export function ApiTokensPage() {
  const { t } = useTranslation('api-tokens')
  const { hasPermission } = useSession()
  const canSeeAll = hasPermission(USERS_MANAGE)
  const [owner, setOwner] = useState<ApiTokenOwner>('mine')
  const tokens = useApiTokens(owner)
  const invalidate = useInvalidateApiTokens()
  const [creating, setCreating] = useState(false)
  const [created, setCreated] = useState<CreatedApiToken | null>(null)
  const [revoking, setRevoking] = useState<ApiToken | null>(null)

  const revoke = async (token: ApiToken) => {
    await apiTokensApi.revoke(token.id)
    await invalidate()
    notifySuccess(t('revoked', { name: token.name }))
  }

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button size="sm" onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      {canSeeAll && (
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={owner === 'all'}
            onCheckedChange={(checked) => setOwner(checked === true ? 'all' : 'mine')}
          />
          {t('showAll')}
        </label>
      )}
      {tokens.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : tokens.isError ? (
        <ErrorState onRetry={() => void tokens.refetch()} />
      ) : tokens.data.length === 0 ? (
        <EmptyState icon={KeyRound} title={t('emptyTitle')} description={t('emptyDescription')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('listLabel')}>
          {tokens.data.map((token) => (
            <TokenRow key={token.id} token={token} showOwner={owner === 'all'} onRevoke={() => setRevoking(token)} />
          ))}
        </ul>
      )}
      <Dialog
        open={creating || created !== null}
        onOpenChange={(open) => {
          if (!open) {
            setCreating(false)
            setCreated(null)
          }
        }}
      >
        <DialogContent>
          {created ? (
            <SecretView token={created} onClose={() => setCreated(null)} />
          ) : (
            <CreateTokenForm
              onCancel={() => setCreating(false)}
              onCreated={(token) => {
                setCreating(false)
                setCreated(token)
                void invalidate()
              }}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={revoking !== null}
        onOpenChange={(open) => !open && setRevoking(null)}
        title={t('revokeTitle')}
        description={t('revokeDescription', { name: revoking?.name ?? '' })}
        confirmLabel={t('revokeConfirm')}
        variant="destructive"
        onConfirm={() => (revoking ? revoke(revoking) : undefined)}
      />
      <ApiTokenLimitsPanel />
    </Page>
  )
}

interface TokenRowProps {
  token: ApiToken
  showOwner: boolean
  onRevoke: () => void
}

function TokenRow({ token, showOwner, onRevoke }: TokenRowProps) {
  const { t } = useTranslation('api-tokens')
  const formatting = useRecordFormatting()
  const status = tokenStatus(token)

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-words">{token.name}</span>
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{token.token_prefix}…</code>
          <Badge variant={status === 'active' ? 'secondary' : 'outline'}>{t(`status.${status}`)}</Badge>
        </div>
        <div className="flex flex-wrap gap-1">
          {token.scopes.map((scope) => (
            <Badge key={scope} variant="outline" className="font-mono text-[11px]">
              {scope}
            </Badge>
          ))}
        </div>
        <p className="text-xs text-muted-foreground">
          {showOwner && token.created_by_name ? `${t('createdBy', { name: token.created_by_name })} · ` : ''}
          {t('createdAt', { date: formatDateTime(token.created_at, formatting) })}
          {' · '}
          {token.expires_at
            ? t('expiresAt', { date: formatDateTime(token.expires_at, formatting) })
            : t('neverExpires')}
          {' · '}
          {token.last_used_at
            ? t('lastUsed', {
                date: formatDateTime(token.last_used_at, formatting),
                ip: token.last_used_ip ?? t('unknownIp'),
              })
            : t('neverUsed')}
        </p>
      </div>
      {status === 'active' && (
        <Button
          size="sm"
          variant="outline"
          className="self-start"
          aria-label={t('revokeLabel', { name: token.name })}
          onClick={onRevoke}
        >
          {t('revoke')}
        </Button>
      )}
    </li>
  )
}

interface CreateTokenFormProps {
  onCancel: () => void
  onCreated: (token: CreatedApiToken) => void
}

function CreateTokenForm({ onCancel, onCreated }: CreateTokenFormProps) {
  const { t } = useTranslation('api-tokens')
  const formId = useId()
  const scopes = useApiTokenScopes()
  const [name, setName] = useState('')
  const [selected, setSelected] = useState<string[]>([])
  const [expiration, setExpiration] = useState<ExpirationOption>('90')
  const [saving, setSaving] = useState(false)
  const [nameError, setNameError] = useState<string | null>(null)
  const [scopesError, setScopesError] = useState<string | null>(null)

  const toggle = (scope: string, checked: boolean) => {
    setScopesError(null)
    setSelected((current) => (checked ? [...current.filter((item) => item !== scope), scope] : current.filter((item) => item !== scope)))
  }

  const submit = async () => {
    const trimmed = name.trim()
    const missingName = !trimmed
    const missingScopes = selected.length === 0
    setNameError(missingName ? t('form.nameRequired') : null)
    setScopesError(missingScopes ? t('form.scopesRequired') : null)
    if (missingName || missingScopes) return
    setSaving(true)
    try {
      const token = await apiTokensApi.create({
        name: trimmed,
        scopes: selected,
        expires_at: expirationDate(expiration === 'never' ? null : Number(expiration)),
      })
      onCreated(token)
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t('form.title')}</DialogTitle>
        <DialogDescription>{t('form.description')}</DialogDescription>
      </DialogHeader>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <div className="grid gap-2.5">
          <Label htmlFor={`${formId}-name`}>{t('form.name')}</Label>
          <Input
            id={`${formId}-name`}
            value={name}
            maxLength={MAX_API_TOKEN_NAME_LENGTH}
            autoFocus
            placeholder={t('form.namePlaceholder')}
            aria-invalid={nameError ? true : undefined}
            aria-describedby={nameError ? `${formId}-name-error` : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(null)
            }}
          />
          {nameError && (
            <p id={`${formId}-name-error`} className="text-xs text-destructive">
              {nameError}
            </p>
          )}
        </div>
        <fieldset className="grid gap-2" aria-describedby={scopesError ? `${formId}-scopes-error` : undefined}>
          <legend className="mb-1 text-sm font-medium">{t('form.scopes')}</legend>
          {scopes.isPending ? (
            <Skeleton className="h-24 w-full" />
          ) : scopes.isError ? (
            <ErrorState onRetry={() => void scopes.refetch()} />
          ) : (
            <div className="grid max-h-64 gap-1 overflow-y-auto rounded-md border p-2">
              {groupScopes(scopes.data).map((group) => (
                <div key={group.resource} className="flex flex-wrap items-center justify-between gap-2 py-1">
                  <span className="text-sm">{t(`resources.${group.resource}`, { defaultValue: group.resource })}</span>
                  <div className="flex gap-4">
                    {[group.read, group.write].map((scope) =>
                      scope ? (
                        <label key={scope.scope} className="flex items-center gap-1.5 text-sm">
                          <Checkbox
                            checked={selected.includes(scope.scope)}
                            aria-label={scope.scope}
                            onCheckedChange={(checked) => toggle(scope.scope, checked === true)}
                          />
                          {t(`access.${scope.access as 'read' | 'write'}`)}
                        </label>
                      ) : null,
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
          {scopesError && (
            <p id={`${formId}-scopes-error`} className="text-xs text-destructive">
              {scopesError}
            </p>
          )}
        </fieldset>
        <div className="grid gap-2.5">
          <Label htmlFor={`${formId}-expiration`}>{t('form.expiration')}</Label>
          <NativeSelect
            id={`${formId}-expiration`}
            value={expiration}
            onChange={(event) => setExpiration(event.target.value as ExpirationOption)}
          >
            {EXPIRATION_OPTIONS.map((option) => (
              <option key={option} value={option}>
                {t(`form.expirationOptions.${option}`)}
              </option>
            ))}
          </NativeSelect>
        </div>
      </form>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('form.submit')}
        </Button>
      </DialogFooter>
    </>
  )
}

function SecretView({ token, onClose }: { token: CreatedApiToken; onClose: () => void }) {
  const { t } = useTranslation('api-tokens')
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(token.secret)
      setCopied(true)
    } catch {
      notifyError(new Error(t('secret.copyFailed')))
    }
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t('secret.title', { name: token.name })}</DialogTitle>
        <DialogDescription>{t('secret.description')}</DialogDescription>
      </DialogHeader>
      <div className="grid gap-2">
        <code data-testid="api-token-secret" className="block rounded-md bg-muted p-3 text-xs break-all">
          {token.secret}
        </code>
        <Button type="button" variant="outline" size="sm" className="self-start" onClick={() => void copy()}>
          {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied ? t('secret.copied') : t('secret.copy')}
        </Button>
        <pre className="overflow-x-auto rounded-md border p-3 text-xs">
          {`curl -H "Authorization: Bearer ${token.token_prefix}_…" \\\n  ${window.location.origin}/api/v1/contacts`}
        </pre>
      </div>
      <DialogFooter>
        <Button type="button" onClick={onClose}>
          {t('secret.done')}
        </Button>
      </DialogFooter>
    </>
  )
}
