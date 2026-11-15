import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Coins, Pencil, Plus, Trash2, X } from 'lucide-react'
import { useId, useMemo, useState } from 'react'
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
import { formatDate, NativeSelect, RequirePermission, SETTINGS_MANAGE, useRecordFormatting } from '@/sdk'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Pagination } from '@/components/ui/pagination'
import { currencyName, formatRate, selectableCurrencies } from './format'
import {currenciesApi, RATES_PER_PAGE, useCurrencySettings, useExchangeRates, useInvalidateCurrencies} from '@/sdk'
import {currencyErrorKey} from '@/sdk'
import { RatesProviderSettings } from './RatesProviderSettings'
import type {CurrencySettings, ExchangeRate} from '@/sdk'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'
import { Page } from '@/components/ui/page'

const RATE_PATTERN = /^\d+([.,]\d{1,10})?$/

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
        <div className="grid gap-6">
          <EnabledCurrencies settings={settings.data} />
          <RatesProviderSettings settings={settings.data} />
          <ExchangeRates settings={settings.data} />
        </div>
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
  const [code, setCode] = useState('')
  const [saving, setSaving] = useState(false)
  const enabled = settings.currencies.filter((currency) => !currency.is_base).map((currency) => currency.code)
  const options = useMemo(
    () => selectableCurrencies(formatting.locale).filter((option) => option.code !== settings.base && !enabled.includes(option.code)),
    [formatting.locale, settings.base, enabled],
  )

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
    if (!code) return
    if (await save([...enabled, code], t('enabled.added', { code }))) setCode('')
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('enabled.title')}</CardTitle>
        <CardDescription>{t('enabled.description', { base: currencyName(settings.base, formatting.locale) })}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <ul className="grid gap-2" aria-label={t('enabled.listLabel')}>
          {settings.currencies.map((currency) => (
            <li key={currency.code} className="flex flex-wrap items-center gap-3 rounded-md border bg-background px-3 py-2">
              <Coins className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
              <div className="grid min-w-0 flex-1 gap-0.5">
                <span className="text-sm font-medium">
                  {currencyName(currency.code, formatting.locale)} <span className="font-normal text-muted-foreground">({currency.code})</span>
                </span>
                {!currency.is_base && currency.current_rate && (
                  <span className="text-xs text-muted-foreground">
                    {t('enabled.currentRate', {
                      code: currency.code,
                      rate: formatRate(currency.current_rate.rate, formatting.locale),
                      base: settings.base,
                      date: formatDate(currency.current_rate.effective_date, formatting),
                    })}
                  </span>
                )}
              </div>
              {currency.is_base ? (
                <Badge variant="secondary">{t('enabled.base')}</Badge>
              ) : (
                !currency.current_rate && <Badge variant="warning">{t('enabled.noRate')}</Badge>
              )}
              {canEdit && !currency.is_base && (
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-8"
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
            <Field label={t('enabled.code')} className="w-full sm:w-80">
              {(control) => (
                <NativeSelect {...control} value={code} onChange={(event) => setCode(event.target.value)}>
                  <option value="">{t('enabled.choose')}</option>
                  {options.map((option) => (
                    <option key={option.code} value={option.code}>
                      {option.label}
                    </option>
                  ))}
                </NativeSelect>
              )}
            </Field>
            <Button type="submit" variant="outline" disabled={saving || !code}>
              <Plus aria-hidden="true" />
              {t('enabled.add')}
            </Button>
          </form>
        )}
      </CardContent>
    </Card>
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
  const current = new Map(settings.currencies.map((item) => [item.code, item.current_rate?.effective_date ?? null]))

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
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="grid gap-1.5">
            <CardTitle>{t('rates.title')}</CardTitle>
            <CardDescription>{t('rates.description', { base: settings.base })}</CardDescription>
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
                {currencyName(code, formatting.locale)} ({code})
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
      </CardHeader>
      <CardContent className="grid gap-4">
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
                  <TableCell>
                    {currencyName(rate.currency, formatting.locale)} <span className="text-muted-foreground">({rate.currency})</span>
                    {current.get(rate.currency) === rate.effective_date && (
                      <Badge variant="success" className="ml-2">{t('rates.current')}</Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-right tabular-nums">{formatRate(rate.rate, formatting.locale)}</TableCell>
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
          <Pagination page={page} perPage={RATES_PER_PAGE} total={rates.data.meta.total} onPageChange={setPage} />
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
      </CardContent>
    </Card>
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
