import { Percent } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import {priceListsApi, useDiscountLimits, useInvalidatePriceLists} from '@/sdk'
import { priceListErrorKey } from './errors'
import type {DiscountLimit} from '@/sdk'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { RequirePermission, SETTINGS_MANAGE } from '@/sdk'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const PERCENT_PATTERN = /^\d{1,3}([.,]\d{1,2})?$/

function normalize(value: string): string | null {
  const text = value.trim().replace(',', '.')
  return text ? text : null
}

export function DiscountLimitsPage() {
  const limits = useDiscountLimits()
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      {limits.isPending ? (
        <PageLoader />
      ) : limits.isError ? (
        <ErrorState onRetry={() => void limits.refetch()} />
      ) : (
        <DiscountLimitsForm key={limits.dataUpdatedAt} limits={limits.data} />
      )}
    </RequirePermission>
  )
}

function DiscountLimitsForm({ limits }: { limits: DiscountLimit[] }) {
  const { t } = useTranslation('pricing')
  const invalidate = useInvalidatePriceLists()
  const initial = Object.fromEntries(limits.map((limit) => [limit.role_id, limit.max_discount_percent ?? '']))
  const [values, setValues] = useState<Record<string, string>>(initial)
  const [invalid, setInvalid] = useState<string[]>([])
  const [saving, setSaving] = useState(false)
  const dirty = limits.some((limit) => normalize(values[limit.role_id] ?? '') !== (limit.max_discount_percent ?? null))

  const save = async () => {
    const wrong = limits
      .filter((limit) => {
        const value = normalize(values[limit.role_id] ?? '')
        return value !== null && (!PERCENT_PATTERN.test(value) || Number(value) > 100)
      })
      .map((limit) => limit.role_id)
    if (wrong.length > 0) {
      setInvalid(wrong)
      return
    }
    setSaving(true)
    try {
      await priceListsApi.setDiscountLimits(
        limits.map((limit) => ({
          role_id: limit.role_id,
          max_discount_percent: normalize(values[limit.role_id] ?? ''),
        })),
      )
      await invalidate()
      toast({ title: t('limits.saved'), variant: 'success' })
    } catch (error) {
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  if (limits.length === 0) {
    return (
      <Page>
        <PageHeader title={t('limits.title')} description={t('limits.description')} />
        <EmptyState icon={Percent} title={t('limits.empty')} />
      </Page>
    )
  }

  return (
    <Page>
      <UnsavedChangesGuard when={dirty && !saving} />
      <PageHeader
        title={t('limits.title')}
        description={t('limits.description')}
        actions={
          <Button type="submit" form="discount-limits-form" disabled={!dirty || saving}>
            {t('limits.save')}
          </Button>
        }
      />
      <form
        id="discount-limits-form"
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void save()
        }}
      >
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('limits.role')}</TableHead>
              <TableHead scope="col" className="w-48">
                {t('limits.maxDiscount')}
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {limits.map((limit) => {
              const inputId = `discount-limit-${limit.role_id}`
              const wrong = invalid.includes(limit.role_id)
              return (
                <TableRow key={limit.role_id} className="align-top">
                  <TableCell>
                    <label htmlFor={inputId} className="font-medium">
                      {limit.role_name}
                    </label>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Input
                        id={inputId}
                        inputMode="decimal"
                        className="w-28"
                        value={values[limit.role_id] ?? ''}
                        placeholder={t('limits.noLimit')}
                        aria-invalid={wrong ? true : undefined}
                        aria-describedby={wrong ? `${inputId}-error` : undefined}
                        onChange={(event) => {
                          setValues({ ...values, [limit.role_id]: event.target.value })
                          setInvalid(invalid.filter((id) => id !== limit.role_id))
                        }}
                      />
                      <span className="text-muted-foreground">%</span>
                    </div>
                    {wrong && (
                      <p id={`${inputId}-error`} className="mt-1 text-xs text-destructive">
                        {t('limits.invalid')}
                      </p>
                    )}
                  </TableCell>
                </TableRow>
              )
            })}
          </TableBody>
        </Table>
        <p className="text-xs text-muted-foreground">{t('limits.hint')}</p>
      </form>
    </Page>
  )
}
