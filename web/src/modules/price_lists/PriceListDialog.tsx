import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useSegments } from '@/modules/segments/api'
import { useCurrencySettings } from '@/modules/currencies/api'
import { priceListsApi, useInvalidatePriceLists } from './api'
import { fieldErrors, priceListErrorKey } from './errors'
import type { PriceList, PriceListAppliesTo, PriceListRequest } from './types'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

const APPLIES_TO: PriceListAppliesTo[] = ['all', 'segment', 'companies']
const COMPANY_RECORD_TYPE = 'companies.company'

export interface PriceListDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  list: PriceList | null
  onSaved?: (list: PriceList) => void
}

export function PriceListDialog({ open, onOpenChange, list, onSaved }: PriceListDialogProps) {
  const { t } = useTranslation('price_lists')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t(list ? 'form.editTitle' : 'form.newTitle')}</DialogTitle>
          <DialogDescription>{t('form.description')}</DialogDescription>
        </DialogHeader>
        {open && (
          <PriceListForm
            key={list?.id ?? 'new'}
            list={list}
            onClose={() => onOpenChange(false)}
            onSaved={onSaved}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface PriceListFormProps {
  list: PriceList | null
  onClose: () => void
  onSaved?: (list: PriceList) => void
}

function PriceListForm({ list, onClose, onSaved }: PriceListFormProps) {
  const { t } = useTranslation('price_lists')
  const formId = useId()
  const invalidate = useInvalidatePriceLists()
  const currencies = useCurrencySettings()
  const segments = useSegments(COMPANY_RECORD_TYPE)
  const [name, setName] = useState(list?.name ?? '')
  const [key, setKey] = useState(list?.key ?? '')
  const [description, setDescription] = useState(list?.description ?? '')
  const [currency, setCurrency] = useState(list?.currency ?? '')
  const [appliesTo, setAppliesTo] = useState<PriceListAppliesTo>(list?.applies_to ?? 'all')
  const [segmentId, setSegmentId] = useState(list?.segment_id ?? '')
  const [companyIds, setCompanyIds] = useState<string[]>(list?.company_ids ?? [])
  const [validFrom, setValidFrom] = useState(list?.valid_from ?? '')
  const [validTo, setValidTo] = useState(list?.valid_to ?? '')
  const [isActive, setIsActive] = useState(list?.is_active ?? true)
  const [isDefault, setIsDefault] = useState(list?.is_default ?? false)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)
  const codes = currencies.data?.currencies.map((item) => item.code) ?? []
  const selectedCurrency = currency || currencies.data?.base || ''

  const submit = async () => {
    if (!name.trim()) {
      setErrors({ name: t('form.required') })
      return
    }
    const body: PriceListRequest = {
      name: name.trim(),
      description: description.trim() || null,
      currency: selectedCurrency || undefined,
      applies_to: appliesTo,
      segment_id: appliesTo === 'segment' ? segmentId || null : null,
      company_ids: appliesTo === 'companies' ? companyIds : [],
      valid_from: validFrom || null,
      valid_to: validTo || null,
      is_active: isActive,
      is_default: appliesTo === 'all' && isDefault,
    }
    if (key.trim() && key.trim() !== list?.key) body.key = key.trim()
    setSaving(true)
    try {
      const saved = list
        ? await priceListsApi.update(list.id, body, list.updated_at)
        : await priceListsApi.create(body)
      await invalidate()
      toast({ title: t(list ? 'form.updated' : 'form.created'), variant: 'success' })
      onSaved?.(saved)
      onClose()
    } catch (error) {
      const fields = fieldErrors(error)
      setErrors(Object.fromEntries(fields.map((entry) => [entry.field.split('.')[0], entry.message])))
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  const errorFor = (field: string) =>
    errors[field] ? (
      <SharedFieldError id={`${formId}-${field}-error`}>
        {errors[field]}
      </SharedFieldError>
    ) : null

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
        <Field label={t('form.name')}>
          {(control) => <>
            <Input placeholder={t('placeholders.name')}
            {...control}
            value={name}
            maxLength={140}
            autoFocus
            aria-invalid={errors.name ? true : undefined}
            onChange={(event) => setName(event.target.value)}
          />
          {errorFor('name')}
          </>}
        </Field>
        <Field label={t('form.key')}>
          {(control) => <>
            <Input
            {...control}
            className="font-mono"
            value={key}
            maxLength={64}
            placeholder={t('form.keyPlaceholder')}
            aria-invalid={errors.key ? true : undefined}
            onChange={(event) => setKey(event.target.value)}
          />
          {errorFor('key') ?? <p className="text-xs text-muted-foreground">{t('form.keyHint')}</p>}
          </>}
        </Field>
        <Field label={t('form.descriptionLabel')}>
          {(control) => <Textarea placeholder={t('placeholders.description')}
            {...control}
            value={description}
            maxLength={2000}
            rows={2}
            onChange={(event) => setDescription(event.target.value)}
          />}
        </Field>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t('form.currency')}>
            {(control) => <>
              <NativeSelect
              {...control}
              value={selectedCurrency}
              onChange={(event) => setCurrency(event.target.value)}
            >
              {codes.length === 0 && selectedCurrency && <option value={selectedCurrency}>{selectedCurrency}</option>}
              {codes.map((code) => (
                <option key={code} value={code}>
                  {code}
                </option>
              ))}
            </NativeSelect>
            {errorFor('currency')}
            </>}
          </Field>
          <Field label={t('form.appliesTo')}>
            {(control) => <NativeSelect
              {...control}
              value={appliesTo}
              onChange={(event) => setAppliesTo(event.target.value as PriceListAppliesTo)}
            >
              {APPLIES_TO.map((value) => (
                <option key={value} value={value}>
                  {t(`appliesTo.${value}`)}
                </option>
              ))}
            </NativeSelect>}
          </Field>
        </div>
        {appliesTo === 'segment' && (
          <Field label={t('form.segment')}>
            {(control) => <>
              <NativeSelect
              {...control}
              value={segmentId}
              aria-invalid={errors.segment_id ? true : undefined}
              onChange={(event) => setSegmentId(event.target.value)}
            >
              <option value="">{t('form.segmentPlaceholder')}</option>
              {(segments.data ?? []).map((segment) => (
                <option key={segment.id} value={segment.id}>
                  {segment.name}
                </option>
              ))}
            </NativeSelect>
            {errorFor('segment_id') ?? <p className="text-xs text-muted-foreground">{t('form.segmentHint')}</p>}
            </>}
          </Field>
        )}
        {appliesTo === 'companies' && (
          <fieldset className="grid gap-2">
            <legend className="mb-1.5 text-sm font-medium">{t('form.companies')}</legend>
            {companyIds.map((companyId, index) => (
              <div key={companyId} className="flex items-center gap-2">
                <div className="min-w-0 flex-1">
                  <RecordFieldInput
                    targetRecordType={COMPANY_RECORD_TYPE}
                    value={companyId}
                    clearable={false}
                    aria-label={t('form.company', { position: index + 1 })}
                    onChange={(value) => {
                      if (!value || companyIds.includes(value)) return
                      setCompanyIds(companyIds.map((current) => (current === companyId ? value : current)))
                    }}
                  />
                </div>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  aria-label={t('form.removeCompany')}
                  onClick={() => setCompanyIds(companyIds.filter((current) => current !== companyId))}
                >
                  <X aria-hidden="true" />
                </Button>
              </div>
            ))}
            <RecordFieldInput
              targetRecordType={COMPANY_RECORD_TYPE}
              value={null}
              aria-label={t('form.addCompany')}
              onChange={(value) => {
                if (value && !companyIds.includes(value)) setCompanyIds([...companyIds, value])
              }}
            />
            {errorFor('company_ids') ?? <p className="text-xs text-muted-foreground">{t('form.companiesHint')}</p>}
          </fieldset>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t('form.validFrom')}>
            {(control) => <Input
              {...control}
              type="date"
              value={validFrom}
              onChange={(event) => setValidFrom(event.target.value)}
            />}
          </Field>
          <Field label={t('form.validTo')}>
            {(control) => <>
              <Input
              {...control}
              type="date"
              value={validTo}
              aria-invalid={errors.valid_to ? true : undefined}
              onChange={(event) => setValidTo(event.target.value)}
            />
            {errorFor('valid_to')}
            </>}
          </Field>
        </div>
        <p className="-mt-2 text-xs text-muted-foreground">{t('form.validityHint')}</p>
        <ToggleField label={t('form.active')} checked={isActive} onCheckedChange={(checked) => setIsActive(checked === true)} />
        <div className="grid gap-1">
          <ToggleField label={t('form.default', { currency: selectedCurrency })} checked={appliesTo === 'all' && isDefault} onCheckedChange={(checked) => setIsDefault(checked === true)} disabled={appliesTo !== 'all'} />
          <p className="text-xs text-muted-foreground">{t('form.defaultHint')}</p>
        </div>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('form.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
