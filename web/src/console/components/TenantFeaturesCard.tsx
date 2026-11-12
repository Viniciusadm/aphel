import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Plug } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ToggleField } from '@/components/ui/toggle-field'
import { consoleApi, consoleKeys, type Tenant } from '../api'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'
import { tenantPluginsPath } from '../paths'

export function TenantFeaturesCard({ tenant }: { tenant: Tenant }) {
  const { t } = useTranslation('console')
  const queryClient = useQueryClient()
  const update = useMutation({
    mutationFn: (enabled: boolean) => consoleApi.updateFeatures(tenant.slug, enabled),
    onSuccess: (updated) => {
      queryClient.setQueryData(consoleKeys.tenant(tenant.slug), updated)
      void queryClient.invalidateQueries({ queryKey: consoleKeys.tenants, exact: true })
      notifyConsoleSuccess(t('features.saved'))
    },
    onError: notifyConsoleError,
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('features.title')}</CardTitle>
        <CardDescription>{t('features.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <ToggleField
          type="switch"
          label={t('features.pluginManagement')}
          description={t('features.pluginManagementHelp')}
          checked={tenant.plugin_management}
          disabled={update.isPending || tenant.status === 'deleting'}
          onCheckedChange={(enabled) => update.mutate(enabled)}
        />
        {tenant.status === 'active' && (
          <div>
            <Button asChild variant="outline">
              <Link to={tenantPluginsPath(tenant.slug)}>
                <Plug className="size-4" aria-hidden />
                {t('features.managePlugins')}
              </Link>
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
