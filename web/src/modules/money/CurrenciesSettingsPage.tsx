import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Coins, Pencil, Plus, Trash2, X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
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
import { toast } from '@/components/ui/toast-store'
import { formatDate, formatNumber, NativeSelect, RequirePermission, SETTINGS_MANAGE, useRecordFormatting } from '@/sdk'
import { currenciesApi, RATES_PER_PAGE, useCurrencySettings, useExchangeRates, useInvalidateCurrencies } from './api'
import { currencyErrorKey } from './errors'
import { RatesProviderSettings } from './RatesProviderSettings'
import type { CurrencySettings, ExchangeRate } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'
import { Page } from '@/components/ui/page'

const CURRENCY_CODE = /^[A-Za-z]{3}$/
const RATE_PATTERN = /^\d+([.,]\d{1,10})?$/
const RATE_DECIMALS = 10

interface RateEditor {
  rate: ExchangeRate | null
}

export function CurrenciesSettingsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <CurrenciesSettings />
    </RequirePermission>
  )
}

function CurrenciesSettings() {
  const { t } = useTranslation('money')
  const settings = useCurrencySettings()
  return (
    <Page>
<PageHeader title={t('title')} description={t('description')} />
      {settings.isPending ? (
        <PageLoader />
      ) : settings.isError ? (
        <ErrorState onRetry={() => void settings.refetch()} />
      ) : (
        <>
          <EnabledCurrencies settings={settings.data} />
          <RatesProviderSettings settings={settings.data} />
          <ExchangeRates settings={settings.data} />
        </>
      )}
    </Page>
  )
}

function EnabledCurrencies({ settings }: { settings: CurrencySettings }) {
  const { t } = useTranslation('money')
  const { hasPermission } = useSession()
  const canEdit = hasPermission(SETTINGS_MANAGE)
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateCurrencies()
  const inputId = useId()
  const [code, setCode] = useState('')
  const [invalid, setInvalid] = useState(false)
  const [saving, setSaving] = useState(false)
  const enabled = settings.currencies.filter((currency) => !currency.is_base).map((currency) => currency.code)

  const save = async (next: string[], message: string) => {
    setSaving(true)
    try {
      await currenciesApi.setCurrencies(next)
      await invalidate()
      toast({ title: message, variant: 'success' })
      return true
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
      return false
    } finally {
      setSaving(false)
    }
  }

  const add = async () => {
    const normalized = code.trim().toUpperCase()
    if (!CURRENCY_CODE.test(normalized)) {
      setInvalid(true)
      return
    }
    if (normalized === settings.base || enabled.includes(normalized)) {
      setCode('')
      return
    }
    if (await save([...enabled, normalized], t('enabled.added', { code: normalized }))) setCode('')
  }

  return (
    <div className="grid gap-4">
      <div className="grid gap-1">
        <h3 className="text-base font-semibold">{t('enabled.title')}</h3>
        <p className="text-sm text-muted-foreground">{t('enabled.description', { base: settings.base })}</p>
      </div>
      <ul className="grid gap-2" aria-label={t('enabled.listLabel')}>
        {settings.currencies.map((currency) => (
          <li key={currency.code} className="flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2">
            <Coins className="size-4 text-muted-foreground" aria-hidden="true" />
            <span className="min-w-12 font-mono text-sm font-medium">{currency.code}</span>
            {currency.is_base ? (
              <Badge variant="secondary">{t('enabled.base')}</Badge>
            ) : currency.current_rate ? (
              <span className="text-sm text-muted-foreground">
                {t('enabled.currentRate', {
                  rate: formatNumber(currency.current_rate.rate, formatting, RATE_DECIMALS),
                  base: settings.base,
                  date: formatDate(currency.current_rate.effective_date, formatting),
                })}
              </span>
            ) : (
              <Badge variant="warning">{t('enabled.noRate')}</Badge>
            )}
            {canEdit && !currency.is_base && (
              <Button
                size="sm"
                variant="ghost"
                className="ml-auto"
                disabled={saving}
                aria-label={t('enabled.remove', { code: currency.code })}
                onClick={() =>
                  void save(
                    enabled.filter((item) => item !== currency.code),
                    t('enabled.removed', { code: currency.code }),
                  )
                }
              >
                <X aria-hidden="true" />
              </Button>
            )}
          </li>
        ))}
      </ul>
      {canEdit && (
        <form
          className="flex flex-wrap items-end gap-2"
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            void add()
          }}
        >
          <Field label={t('enabled.code')}>
            {(control) => <Input
              {...control}
              className="w-32 font-mono uppercase"
              maxLength={3}
              value={code}
              placeholder="USD"
              aria-invalid={invalid ? true : undefined}
              aria-describedby={invalid ? `${inputId}-error` : undefined}
              onChange={(event) => {
                setCode(event.target.value)
                setInvalid(false)
              }}
            />}
          </Field>
          <Button type="submit" variant="outline" disabled={saving || !code.trim()}>
            <Plus aria-hidden="true" />
            {t('enabled.add')}
          </Button>
          {invalid && (
            <SharedFieldError id={`${inputId}-error`} className="basis-full">
              {t('enabled.invalidCode')}
            </SharedFieldError>
          )}
        </form>
      )}
    </div>
  )
}

function ExchangeRates({ settings }: { settings: CurrencySettings }) {
  const { t } = useTranslation('money')
  const { hasPermission } = useSession()
  const canEdit = hasPermission(SETTINGS_MANAGE)
  const formatting = useRecordFormatting()
  const invalidate = useInvalidateCurrencies()
  const [currency, setCurrency] = useState('')
  const [page, setPage] = useState(1)
  const [editor, setEditor] = useState<RateEditor | null>(null)
  const [removing, setRemoving] = useState<ExchangeRate | null>(null)
  const rates = useExchangeRates(currency, page)
  const foreign = settings.currencies.filter((item) => !item.is_base).map((item) => item.code)
  const lastPage = rates.data ? Math.max(1, Math.ceil(rates.data.meta.total / RATES_PER_PAGE)) : 1

  const remove = async () => {
    if (!removing) return
    try {
      await currenciesApi.deleteRate(removing.id)
      await invalidate()
      toast({ title: t('rates.deleted'), variant: 'success' })
      setRemoving(null)
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <h3 className="text-base font-semibold">{t('rates.title')}</h3>
          <p className="text-sm text-muted-foreground">{t('rates.description', { base: settings.base })}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <NativeSelect
            aria-label={t('rates.filter')}
            value={currency}
            onChange={(event) => {
              setCurrency(event.target.value)
              setPage(1)
            }}
          >
            <option value="">{t('rates.allCurrencies')}</option>
            {foreign.map((code) => (
              <option key={code} value={code}>
                {code}
              </option>
            ))}
          </NativeSelect>
          {canEdit && (
            <Button size="sm" disabled={foreign.length === 0} onClick={() => setEditor({ rate: null })}>
              <Plus aria-hidden="true" />
              {t('rates.new')}
            </Button>
          )}
        </div>
      </div>
      {rates.isPending ? (
        <PageLoader />
      ) : rates.isError ? (
        <ErrorState onRetry={() => void rates.refetch()} />
      ) : rates.data.data.length === 0 ? (
        <EmptyState
          icon={Coins}
          title={t('rates.empty')}
          description={foreign.length === 0 ? t('rates.emptyNoCurrencies') : t('rates.emptyHint')}
        />
      ) : (
        <>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">
                  {t('rates.columns.date')}
                </TableHead>
                <TableHead scope="col">
                  {t('rates.columns.currency')}
                </TableHead>
                <TableHead scope="col" className="text-right">
                  {t('rates.columns.rate', { base: settings.base })}
                </TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">
                  {t('rates.columns.source')}
                </TableHead>
                <TableHead scope="col" className="w-24">
                  <span className="sr-only">{t('rates.columns.actions')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rates.data.data.map((rate) => (
                <TableRow key={rate.id}>
                  <TableCell className="tabular-nums">{formatDate(rate.effective_date, formatting)}</TableCell>
                  <TableCell className="font-mono">{rate.currency}</TableCell>
                  <TableCell className="text-right tabular-nums">
                    {formatNumber(rate.rate, formatting, RATE_DECIMALS)}
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">
                    <Badge variant="outline">{t(`rates.sources.${rate.source}`)}</Badge>
                  </TableCell>
                  <TableCell>
                    {canEdit && (
                      <div className="flex justify-end gap-1">
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label={t('rates.edit')}
                          onClick={() => setEditor({ rate })}
                        >
                          <Pencil aria-hidden="true" />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label={t('rates.delete')}
                          onClick={() => setRemoving(rate)}
                        >
                          <Trash2 aria-hidden="true" />
                        </Button>
                      </div>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {lastPage > 1 && (
            <div className="flex items-center justify-end gap-2 text-sm">
              <Button size="sm" variant="outline" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                {t('rates.previous')}
              </Button>
              <span className="tabular-nums text-muted-foreground">{t('rates.page', { page, total: lastPage })}</span>
              <Button size="sm" variant="outline" disabled={page >= lastPage} onClick={() => setPage(page + 1)}>
                {t('rates.next')}
              </Button>
            </div>
          )}
        </>
      )}
      <RateDialog
        editor={editor}
        currencies={foreign}
        today={settings.today}
        onClose={() => setEditor(null)}
        onSaved={() => void invalidate()}
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('rates.deleteTitle')}
        description={t('rates.deleteDescription')}
        confirmLabel={t('rates.delete')}
        variant="destructive"
        onConfirm={remove}
      />
    </div>
  )
}

interface RateDialogProps {
  editor: RateEditor | null
  currencies: string[]
  today: string
  onClose: () => void
  onSaved: () => void
}

function RateDialog({ editor, currencies, today, onClose, onSaved }: RateDialogProps) {
  const { t } = useTranslation('money')
  const editing = editor?.rate ?? null
  return (
    <Dialog open={editor !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t(editing ? 'rates.editTitle' : 'rates.newTitle')}</DialogTitle>
          <DialogDescription>{t('rates.dialogDescription')}</DialogDescription>
        </DialogHeader>
        {editor && (
          <RateForm
            key={editing?.id ?? 'new'}
            editing={editing}
            currencies={currencies}
            today={today}
            onClose={onClose}
            onSaved={onSaved}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface RateFormProps {
  editing: ExchangeRate | null
  currencies: string[]
  today: string
  onClose: () => void
  onSaved: () => void
}

function RateForm({ editing, currencies, today, onClose, onSaved }: RateFormProps) {
  const { t } = useTranslation('money')
  const formId = useId()
  const [currency, setCurrency] = useState(editing?.currency ?? currencies[0] ?? '')
  const [date, setDate] = useState(editing?.effective_date ?? today)
  const [rate, setRate] = useState(editing?.rate ?? '')
  const [rateError, setRateError] = useState(false)
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    const normalized = rate.trim().replace(',', '.')
    if (!RATE_PATTERN.test(rate.trim()) || Number(normalized) <= 0) {
      setRateError(true)
      return
    }
    setSaving(true)
    try {
      if (editing) {
        await currenciesApi.updateRate(editing.id, { effective_date: date, rate: normalized }, editing.updated_at)
      } else {
        await currenciesApi.createRate({ currency, effective_date: date || null, rate: normalized })
      }
      toast({ title: t(editing ? 'rates.updated' : 'rates.created'), variant: 'success' })
      onSaved()
      onClose()
    } catch (error) {
      toast({ title: t(currencyErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <Field label={t('rates.columns.currency')}>
          {(control) => <NativeSelect
            {...control}
            value={currency}
            disabled={editing !== null}
            onChange={(event) => setCurrency(event.target.value)}
          >
            {(editing ? [editing.currency] : currencies).map((code) => (
              <option key={code} value={code}>
                {code}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('rates.columns.date')}>
          {(control) => <Input {...control} type="date" value={date} onChange={(event) => setDate(event.target.value)} />}
        </Field>
        <Field label={t('rates.rateLabel', { currency })}>
          {(control) => <>
            <Input placeholder={t('placeholders.rate')}
            {...control}
            inputMode="decimal"
            value={rate}
            autoFocus
            aria-invalid={rateError ? true : undefined}
            aria-describedby={rateError ? `${formId}-rate-error` : `${formId}-rate-hint`}
            onChange={(event) => {
              setRate(event.target.value)
              setRateError(false)
            }}
          />
          {rateError ? (
            <SharedFieldError id={`${formId}-rate-error`}>
              {t('rates.invalidRate')}
            </SharedFieldError>
          ) : (
            <p id={`${formId}-rate-hint`} className="text-xs text-muted-foreground">
              {t('rates.rateHint')}
            </p>
          )}
          </>}
        </Field>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('rates.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving || !currency}>
          {t('rates.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
