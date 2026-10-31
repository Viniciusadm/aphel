import { Boxes, Pencil, Plus, Trash2, X } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { toast } from '@/components/ui/toast-store'
import { formatCurrency, formatNumber, useRecordFormatting } from '@/features/records/formatting'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useActivePriceLists } from '../api'
import { fieldErrors } from '../errors'
import { bundlesApi, useInvalidateBundle, useProductBundle } from './api'
import { bundleErrorKey } from './errors'
import type { BundlePricing, ProductBundle } from './types'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'

const PRODUCT_RECORD_TYPE = 'product'
const PRICING: BundlePricing[] = ['fixed', 'sum']
const QUANTITY_PATTERN = /^\d+([.,]\d{1,4})?$/
const QUANTITY_DECIMALS = 4

interface DraftComponent {
  key: string
  productId: string | null
  quantity: string
}

let draftSequence = 0

function nextKey(): string {
  draftSequence += 1
  return `component-${draftSequence}`
}

export interface BundlePanelProps {
  productId: string
  readOnly: boolean
}

export function BundlePanel({ productId, readOnly }: BundlePanelProps) {
  const { t } = useTranslation('price_lists')
  const [priceListId, setPriceListId] = useState('')
  const [editing, setEditing] = useState(false)
  const bundle = useProductBundle(productId, priceListId)

  if (bundle.isPending) return <PageLoader />
  if (bundle.isError) return <ErrorState onRetry={() => void bundle.refetch()} />
  if (editing) {
    return <BundleForm productId={productId} bundle={bundle.data} onClose={() => setEditing(false)} />
  }
  if (!bundle.data) {
    return (
      <EmptyState
        icon={Boxes}
        title={t('bundles.empty')}
        description={t('bundles.emptyHint')}
        action={
          readOnly ? undefined : (
            <Button size="sm" onClick={() => setEditing(true)}>
              <Plus aria-hidden="true" />
              {t('bundles.create')}
            </Button>
          )
        }
      />
    )
  }
  return (
    <BundleView
      productId={productId}
      bundle={bundle.data}
      readOnly={readOnly}
      priceListId={priceListId}
      onPriceListChange={setPriceListId}
      onEdit={() => setEditing(true)}
    />
  )
}

interface BundleViewProps {
  productId: string
  bundle: ProductBundle
  readOnly: boolean
  priceListId: string
  onPriceListChange: (priceListId: string) => void
  onEdit: () => void
}

function BundleView({ productId, bundle, readOnly, priceListId, onPriceListChange, onEdit }: BundleViewProps) {
  const { t } = useTranslation('price_lists')
  const formatting = useRecordFormatting()
  const priceLists = useActivePriceLists()
  const invalidate = useInvalidateBundle(productId)
  const [removing, setRemoving] = useState(false)
  const selectId = useId()

  const remove = async () => {
    try {
      await bundlesApi.remove(productId)
      await invalidate()
      toast({ title: t('bundles.removed'), variant: 'success' })
      setRemoving(false)
    } catch (error) {
      toast({ title: t(bundleErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-base font-semibold">{t('bundles.title')}</h3>
            <Badge variant="secondary">{t(`bundles.pricing.${bundle.pricing}`)}</Badge>
          </div>
          <p className="text-sm text-muted-foreground">
            {t('bundles.price', { price: formatCurrency(bundle.unit_price, formatting, bundle.currency) })}
          </p>
        </div>
        {!readOnly && (
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={onEdit}>
              <Pencil aria-hidden="true" />
              {t('bundles.edit')}
            </Button>
            <Button size="sm" variant="outline" onClick={() => setRemoving(true)}>
              <Trash2 aria-hidden="true" />
              {t('bundles.remove')}
            </Button>
          </div>
        )}
      </div>
      {(priceLists.data?.length ?? 0) > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <Label htmlFor={selectId} className="text-sm text-muted-foreground">
            {t('bundles.priceList')}
          </Label>
          <NativeSelect id={selectId} value={priceListId} onChange={(event) => onPriceListChange(event.target.value)}>
            <option value="">{t('bundles.catalogPrices')}</option>
            {(priceLists.data ?? []).map((list) => (
              <option key={list.id} value={list.id}>
                {t('bundles.priceListOption', { name: list.name, currency: list.currency })}
              </option>
            ))}
          </NativeSelect>
        </div>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">
              {t('bundles.component')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('bundles.quantity')}
            </TableHead>
            <TableHead scope="col" className="text-right">
              {t('bundles.unitPrice')}
            </TableHead>
            <TableHead scope="col" className="hidden text-right sm:table-cell">
              {t('bundles.total')}
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {bundle.components.map((component) => (
            <TableRow key={component.product_id}>
              <TableCell>
                <span className="font-medium">{component.name}</span>
                {component.code && <span className="ml-2 font-mono text-xs text-muted-foreground">{component.code}</span>}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {formatNumber(component.quantity, formatting, QUANTITY_DECIMALS)}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {formatCurrency(component.unit_price, formatting, bundle.currency)}
              </TableCell>
              <TableCell className="hidden text-right tabular-nums sm:table-cell">
                {formatCurrency(component.total, formatting, bundle.currency)}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <p className="text-xs text-muted-foreground">{t(`bundles.pricingHint.${bundle.pricing}`)}</p>
      <ConfirmDialog
        open={removing}
        onOpenChange={setRemoving}
        title={t('bundles.removeTitle')}
        description={t('bundles.removeDescription')}
        confirmLabel={t('bundles.remove')}
        variant="destructive"
        onConfirm={remove}
      />
    </div>
  )
}

interface BundleFormProps {
  productId: string
  bundle: ProductBundle | null
  onClose: () => void
}

function BundleForm({ productId, bundle, onClose }: BundleFormProps) {
  const { t } = useTranslation('price_lists')
  const formId = useId()
  const invalidate = useInvalidateBundle(productId)
  const [pricing, setPricing] = useState<BundlePricing>(bundle?.pricing ?? 'sum')
  const [drafts, setDrafts] = useState<DraftComponent[]>(
    bundle
      ? bundle.components.map((component) => ({
          key: component.product_id,
          productId: component.product_id,
          quantity: component.quantity,
        }))
      : [{ key: nextKey(), productId: null, quantity: '1' }],
  )
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)

  const update = (key: string, patch: Partial<DraftComponent>) => {
    setDrafts(drafts.map((draft) => (draft.key === key ? { ...draft, ...patch } : draft)))
    setErrors({})
  }

  const save = async () => {
    const found: Record<string, string> = {}
    if (drafts.length === 0) found.components = t('bundles.needsComponent')
    drafts.forEach((draft, index) => {
      if (!draft.productId) found[`${index}.product_id`] = t('bundles.productRequired')
      if (!QUANTITY_PATTERN.test(draft.quantity.trim()) || Number(draft.quantity.replace(',', '.')) <= 0)
        found[`${index}.quantity`] = t('bundles.invalidQuantity')
    })
    if (Object.keys(found).length > 0) {
      setErrors(found)
      return
    }
    setSaving(true)
    try {
      await bundlesApi.save(
        productId,
        {
          pricing,
          components: drafts.map((draft) => ({
            product_id: draft.productId ?? '',
            quantity: draft.quantity.trim().replace(',', '.'),
          })),
        },
        bundle?.updated_at,
      )
      await invalidate()
      toast({ title: t('bundles.saved'), variant: 'success' })
      onClose()
    } catch (error) {
      setErrors(
        Object.fromEntries(
          fieldErrors(error).map((entry) => [
            entry.field.startsWith('components.') ? entry.field.slice('components.'.length) : entry.field,
            entry.message,
          ]),
        ),
      )
      toast({ title: t(bundleErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      id={formId}
      className="grid gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <UnsavedChangesGuard when={!saving} />
      <div className="grid gap-1">
        <h3 className="text-base font-semibold">{t(bundle ? 'bundles.editTitle' : 'bundles.createTitle')}</h3>
        <p className="text-sm text-muted-foreground">{t('bundles.formDescription')}</p>
      </div>
      <Field label={t('bundles.pricingLabel')} className="sm:max-w-xs">
        {(control) => <>
          <NativeSelect
          {...control}
          value={pricing}
          onChange={(event) => setPricing(event.target.value as BundlePricing)}
        >
          {PRICING.map((value) => (
            <option key={value} value={value}>
              {t(`bundles.pricing.${value}`)}
            </option>
          ))}
        </NativeSelect>
        <p className="text-xs text-muted-foreground">{t(`bundles.pricingHint.${pricing}`)}</p>
        </>}
      </Field>
      <fieldset className="grid gap-2">
        <legend className="mb-1.5 text-sm font-medium">{t('bundles.components')}</legend>
        {drafts.map((draft, index) => (
          <div key={draft.key} className="grid gap-1">
            <div className="flex items-start gap-2">
              <div className="min-w-0 flex-1">
                <RecordFieldInput
                  targetRecordType={PRODUCT_RECORD_TYPE}
                  value={draft.productId}
                  clearable={false}
                  invalid={Boolean(errors[`${index}.product_id`])}
                  aria-label={t('bundles.componentOf', { position: index + 1 })}
                  onChange={(value) => update(draft.key, { productId: value })}
                />
              </div>
              <Input placeholder={t('placeholders.quantity')}
                className="w-24"
                inputMode="decimal"
                value={draft.quantity}
                aria-label={t('bundles.quantityOf', { position: index + 1 })}
                aria-invalid={errors[`${index}.quantity`] ? true : undefined}
                onChange={(event) => update(draft.key, { quantity: event.target.value })}
              />
              <Button
                type="button"
                size="icon"
                variant="ghost"
                aria-label={t('bundles.removeComponent', { position: index + 1 })}
                onClick={() => setDrafts(drafts.filter((current) => current.key !== draft.key))}
              >
                <X aria-hidden="true" />
              </Button>
            </div>
            {(errors[`${index}.product_id`] || errors[`${index}.quantity`]) && (
              <p className="text-sm text-destructive">
                {errors[`${index}.product_id`] ?? errors[`${index}.quantity`]}
              </p>
            )}
          </div>
        ))}
        {(errors.components || errors.pricing) && (
          <p className="text-sm text-destructive">{errors.components ?? errors.pricing}</p>
        )}
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="w-fit"
          onClick={() => setDrafts([...drafts, { key: nextKey(), productId: null, quantity: '1' }])}
        >
          <Plus aria-hidden="true" />
          {t('bundles.addComponent')}
        </Button>
      </fieldset>
      <p className="text-xs text-muted-foreground">{t('bundles.nestingHint')}</p>
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" disabled={saving} onClick={onClose}>
          {t('bundles.cancel')}
        </Button>
        <Button type="submit" disabled={saving}>
          {t('bundles.save')}
        </Button>
      </div>
    </form>
  )
}
