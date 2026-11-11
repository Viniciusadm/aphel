import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ExternalLink, Star, Trash2 } from 'lucide-react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { consoleApi, consoleKeys, type Tenant } from '../api'
import { notifyConsoleError, notifyConsoleSuccess } from '../errors'
import { hostSchema } from '../schemas'

const schema = z.object({ host: hostSchema })
type Values = z.infer<typeof schema>

export function TenantHostsCard({ tenant, editable }: { tenant: Tenant; editable: boolean }) {
  const { t } = useTranslation('console')
  const queryClient = useQueryClient()
  const store = (updated: Tenant) => {
    queryClient.setQueryData(consoleKeys.tenant(tenant.slug), updated)
    void queryClient.invalidateQueries({ queryKey: consoleKeys.tenants, exact: true })
  }
  const add = useMutation({
    mutationFn: (host: string) => consoleApi.addHost(tenant.slug, host),
    onSuccess: (updated) => {
      store(updated)
      reset()
      notifyConsoleSuccess(t('tenant.hostAdded'))
    },
    onError: notifyConsoleError,
  })
  const remove = useMutation({
    mutationFn: (host: string) => consoleApi.removeHost(tenant.slug, host),
    onSuccess: (updated) => {
      store(updated)
      notifyConsoleSuccess(t('tenant.hostRemoved'))
    },
    onError: notifyConsoleError,
  })
  const primary = useMutation({
    mutationFn: (host: string) => consoleApi.makePrimary(tenant.slug, host),
    onSuccess: (updated) => {
      store(updated)
      notifyConsoleSuccess(t('tenant.primaryChanged'))
    },
    onError: notifyConsoleError,
  })
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { host: '' } })
  const busy = add.isPending || remove.isPending || primary.isPending

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('tenant.hosts')}</CardTitle>
        <CardDescription>{t('tenant.hostsDescription')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <ul className="divide-y divide-border rounded-md border border-border">
          {tenant.hosts.map((host) => (
            <li key={host.host} className="flex flex-wrap items-center gap-2 px-3 py-2">
              <a
                href={`https://${host.host}`}
                target="_blank"
                rel="noreferrer"
                className="inline-flex min-w-0 items-center gap-1 font-medium hover:underline"
              >
                <span className="truncate">{host.host}</span>
                <ExternalLink className="size-3.5 shrink-0" aria-label={t('tenant.open')} />
              </a>
              {host.is_primary && <Badge tone="primary">{t('tenant.primary')}</Badge>}
              {editable && (
                <div className="ml-auto flex gap-1">
                  {!host.is_primary && (
                    <Button variant="ghost" size="sm" disabled={busy} onClick={() => primary.mutate(host.host)}>
                      <Star className="size-4" aria-hidden />
                      {t('tenant.makePrimary')}
                    </Button>
                  )}
                  {tenant.hosts.length > 1 && (
                    <Button variant="ghost" size="sm" disabled={busy} onClick={() => remove.mutate(host.host)}>
                      <Trash2 className="size-4" aria-hidden />
                      {t('tenant.removeHost')}
                    </Button>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
        {editable && (
          <form className="flex flex-wrap items-start gap-2" onSubmit={handleSubmit((values) => add.mutate(values.host))} noValidate>
            <Field
              className="min-w-56 flex-1"
              label={t('tenant.addHost')}
              hideLabel
              error={errors.host?.message && t(errors.host.message as 'validation.host')}
            >
              {(field) => (
                <Input {...field} placeholder={t('tenant.addHost')} inputMode="url" autoComplete="off" spellCheck={false} {...register('host')} />
              )}
            </Field>
            <Button type="submit" variant="outline" disabled={busy}>
              {t('tenant.add')}
            </Button>
          </form>
        )}
      </CardContent>
    </Card>
  )
}
