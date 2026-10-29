import { Percent } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { priceListsApi, useDiscountLimits, useInvalidatePriceLists } from './api'
import { priceListErrorKey } from './errors'
import type { DiscountLimit } from './types'

const PERCENT_PATTERN = /^\d{1,3}([.,]\d{1,2})?$/

function normalize(value: string): string | null {
  const text = value.trim().replace(',', '.')
  return text ? text : null
}

export function DiscountLimitsPanel() {
  const limits = useDiscountLimits()
  if (limits.isPending) return <PageLoader />
  if (limits.isError) return <ErrorState onRetry={() => void limits.refetch()} />
  return <DiscountLimitsForm key={limits.dataUpdatedAt} limits={limits.data} />
}

function DiscountLimitsForm({ limits }: { limits: DiscountLimit[] }) {
  const { t } = useTranslation('price_lists')
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

  if (limits.length === 0) return <EmptyState icon={Percent} title={t('limits.empty')} />

  return (
    <form
      className="grid gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <UnsavedChangesGuard when={dirty && !saving} />
      <div className="grid gap-1">
        <h2 className="text-base font-semibold">{t('limits.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('limits.description')}</p>
      </div>
      <div className="relative overflow-x-auto rounded-lg border">
        <table className="w-full text-sm">
          <thead className="bg-muted/50 text-left text-muted-foreground">
            <tr>
              <th scope="col" className="px-4 py-2 font-medium">
                {t('limits.role')}
              </th>
              <th scope="col" className="w-48 px-4 py-2 font-medium">
                {t('limits.maxDiscount')}
              </th>
            </tr>
          </thead>
          <tbody>
            {limits.map((limit) => {
              const inputId = `discount-limit-${limit.role_id}`
              const wrong = invalid.includes(limit.role_id)
              return (
                <tr key={limit.role_id} className="border-t align-top">
                  <td className="px-4 py-2">
                    <label htmlFor={inputId} className="font-medium">
                      {limit.role_name}
                    </label>
                    <p className="font-mono text-xs text-muted-foreground">{limit.role_key}</p>
                  </td>
                  <td className="px-4 py-2">
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
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
      <p className="text-xs text-muted-foreground">{t('limits.hint')}</p>
      <div className="flex justify-end">
        <Button type="submit" disabled={!dirty || saving}>
          {t('limits.save')}
        </Button>
      </div>
    </form>
  )
}
