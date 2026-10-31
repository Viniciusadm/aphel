import { ShieldAlert, ShieldCheck, ShieldOff } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { TWO_FACTOR_SETUP_REQUIRED, useTwoFactorStatus, type TwoFactorStatus } from './api'
import { DisableDialog } from './DisableDialog'
import { OrganizationPolicy } from './OrganizationPolicy'
import { RegenerateDialog } from './RegenerateDialog'
import { SetupDialog } from './SetupDialog'
import { PageHeader } from '@/components/ui/page-header'

const LOW_RECOVERY_CODES = 3

type OpenDialog = 'setup' | 'disable' | 'regenerate' | null

export function TwoFactorSection() {
  const { t } = useTranslation('two-factor')
  const { me, hasPermission } = useSession()
  const status = useTwoFactorStatus()
  const [dialog, setDialog] = useState<OpenDialog>(null)
  const [searchParams] = useSearchParams()
  const redirectedForSetup = searchParams.get('reason') === TWO_FACTOR_SETUP_REQUIRED
  const mustEnable = !me.two_factor.enabled && (me.two_factor.required || redirectedForSetup)
  const onOpenChange = (open: boolean) => {
    if (!open) setDialog(null)
  }

  return (
    <div className="flex flex-col gap-8">
      <section className="flex flex-col gap-4">
        <PageHeader level="section" title={t('section.title')} description={t('section.description')} />
        {mustEnable && (
          <div
            role="alert"
            className="flex items-start gap-3 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm"
          >
            <ShieldAlert className="mt-0.5 size-5 shrink-0" aria-hidden />
            <div className="grid gap-1">
              <p className="font-medium">{t('required.title')}</p>
              <p className="text-muted-foreground">
                {t('required.description', { organization: me.organization?.name ?? '' })}
              </p>
            </div>
          </div>
        )}
        {status.isPending ? (
          <Skeleton className="h-28 w-full" />
        ) : status.isError ? (
          <ErrorState onRetry={() => void status.refetch()} />
        ) : (
          <StatusCard
            status={status.data}
            onEnable={() => setDialog('setup')}
            onDisable={() => setDialog('disable')}
            onRegenerate={() => setDialog('regenerate')}
          />
        )}
      </section>
      {hasPermission(SETTINGS_MANAGE) && <OrganizationPolicy />}
      <SetupDialog open={dialog === 'setup'} onOpenChange={onOpenChange} />
      <DisableDialog open={dialog === 'disable'} onOpenChange={onOpenChange} />
      <RegenerateDialog open={dialog === 'regenerate'} onOpenChange={onOpenChange} />
    </div>
  )
}

interface StatusCardProps {
  status: TwoFactorStatus
  onEnable: () => void
  onDisable: () => void
  onRegenerate: () => void
}

function StatusCard({ status, onEnable, onDisable, onRegenerate }: StatusCardProps) {
  const { t } = useTranslation('two-factor')
  const formatting = useRecordFormatting()
  const Icon = status.enabled ? ShieldCheck : ShieldOff
  const lowCodes = status.enabled && status.recovery_codes_remaining <= LOW_RECOVERY_CODES

  return (
    <Card>
      <CardContent className="flex flex-col gap-4 pt-6 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex min-w-0 items-start gap-3">
          <Icon
            className={status.enabled ? 'mt-0.5 size-6 shrink-0 text-success' : 'mt-0.5 size-6 shrink-0 text-muted-foreground'}
            aria-hidden
          />
          <div className="grid min-w-0 gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium">{t('status.authenticatorApp')}</span>
              <Badge variant={status.enabled ? 'success' : 'outline'}>
                {status.enabled ? t('status.enabled') : t('status.disabled')}
              </Badge>
            </div>
            {status.enabled ? (
              <>
                {status.enabled_at && (
                  <p className="text-xs text-muted-foreground">
                    {t('status.enabledAt', { date: formatDateTime(status.enabled_at, formatting) })}
                  </p>
                )}
                <p className={lowCodes ? 'text-xs font-medium text-destructive' : 'text-xs text-muted-foreground'}>
                  {t('status.recoveryCodesRemaining', { count: status.recovery_codes_remaining })}
                </p>
              </>
            ) : (
              <p className="text-sm text-muted-foreground">{t('status.disabledDescription')}</p>
            )}
          </div>
        </div>
        <div className="flex flex-wrap gap-2 sm:justify-end">
          {status.enabled ? (
            <>
              <Button type="button" variant="outline" size="sm" onClick={onRegenerate}>
                {t('status.regenerate')}
              </Button>
              <Button type="button" variant="outline" size="sm" className="text-destructive" onClick={onDisable}>
                {t('status.disable')}
              </Button>
            </>
          ) : (
            <Button type="button" size="sm" onClick={onEnable}>
              {t('status.enable')}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
