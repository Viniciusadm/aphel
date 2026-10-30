import { ShieldCheck } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { InstalledPlugin } from './types'

interface PluginGrantStatusProps {
  plugin: InstalledPlugin
  pending: boolean
  onReapply: () => void
}

export function PluginGrantStatus({ plugin, pending, onReapply }: PluginGrantStatusProps) {
  const { t } = useTranslation('modules')
  if (plugin.grants.length === 0) return null
  return (
    <div className="grid gap-2 text-xs">
      <span className="font-medium">{t('plugins.grantsTitle')}</span>
      <ul className="grid gap-1">
        {plugin.grants.map((status) => {
          const role = status.label ?? status.role
          const complete = status.role_exists && status.granted >= status.expected
          return (
            <li key={status.role} className={complete ? 'text-muted-foreground' : 'text-warning'}>
              {status.role_exists
                ? t('plugins.grantsCount', { role, granted: status.granted, expected: status.expected })
                : t('plugins.grantsRoleMissing', { role })}
            </li>
          )
        })}
      </ul>
      {plugin.ungranted_roles.length > 0 && (
        <p role="note" className="text-warning">
          {t('plugins.grantsIncomplete')}
        </p>
      )}
      <div>
        <Button size="sm" variant="outline" onClick={onReapply} disabled={pending}>
          <ShieldCheck aria-hidden="true" />
          {t('plugins.reapplyGrants')}
        </Button>
      </div>
    </div>
  )
}
