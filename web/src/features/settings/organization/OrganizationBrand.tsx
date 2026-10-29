import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Brand } from '@/app/layout/Brand'
import { useSession } from '@/app/session/session-context'
import { cn } from '@/lib/utils'
import { organizationLogoUrl } from './api'

export function OrganizationBrand({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation('organization-settings')
  const { me } = useSession()
  const organization = me.organization
  const logoFileId = organization?.logo_file_id ?? null
  const [failedLogo, setFailedLogo] = useState<string | null>(null)

  if (!organization || !logoFileId || failedLogo === logoFileId) return <Brand compact={compact} />

  return (
    <div className={cn('flex min-w-0 items-center gap-2 font-semibold', compact && 'justify-center')}>
      <img
        src={organizationLogoUrl(logoFileId)}
        alt={t('logo.alt', { name: organization.name })}
        className="size-8 shrink-0 rounded-md object-contain"
        onError={() => setFailedLogo(logoFileId)}
      />
      {!compact && <span className="truncate text-base tracking-tight">{organization.name}</span>}
    </div>
  )
}
