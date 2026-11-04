import { NativeSelect } from '@/components/ui/native-select'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { CircleCheck, MoreHorizontal, Copy, Download, ExternalLink, FilePlus2, FileText, PenLine, Save, Send, ShieldCheck, Trash2 } from 'lucide-react'
import { isValidElement, useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, PageLoader, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { Slot } from '@/kernel/modules/Slot'
import { useActivePriceLists } from '@/modules/pricing/api'
import { formatCurrency, formatDate, formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { quotePath, quotePdfUrl, quotesApi, useQuote, useQuoteMutations } from './api'
import { notifyQuoteError } from './errors'
import { ESIGN_MODULE, PRICE_LISTS_MODULE, PROPOSALS_MODULE, generateQuotePdf } from './modules'
import { DISCOUNT_TYPES, editableFromItem, itemRequest, type EditableItem } from './items'
import { QuoteItemsEditor } from './QuoteItemsEditor'
import { QuoteStatusBadge } from './QuoteStatusBadge'
import { AcceptQuoteDialog } from './AcceptQuoteDialog'
import { SendQuoteDialog } from './SendQuoteDialog'
import { SignatureDialog } from './SignatureDialog'
import { SubmitQuoteDialog } from './SubmitQuoteDialog'
import type { QuoteDetail, QuoteDiscountType, QuoteRequest } from './types'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Field } from '@/components/ui/field'

const NO_PRICE_LIST = '__none__'
const QUOTE_RECORD_TYPE = 'quotes.quote'

interface HeaderDraft {
  title: string
  valid_until: string
  price_list_id: string
  terms: string
  notes: string
  discount_type: QuoteDiscountType
  discount_value: string
}

function headerFrom(quote: QuoteDetail): HeaderDraft {
  return {
    title: quote.title,
    valid_until: quote.valid_until ?? '',
    price_list_id: quote.price_list_id ?? '',
    terms: quote.terms ?? '',
    notes: quote.notes ?? '',
    discount_type: quote.discount_type,
    discount_value: quote.discount_type === 'none' ? '' : quote.discount_value,
  }
}

export function QuoteEditorPage() {
  const { t } = useTranslation('quotes')
  const { id = '' } = useParams()
  const query = useQuote(id)
  if (query.isPending) return <PageLoader />
  if (query.isError) return <ErrorState title={t('editor.loadError')} onRetry={() => void query.refetch()} />
  return <QuoteEditor key={`${query.data.id}:${query.data.updated_at}`} quote={query.data} />
}

function QuoteEditor({ quote }: { quote: QuoteDetail }) {
  const { t } = useTranslation('quotes')
  const ids = { title: useId(), validUntil: useId(), priceList: useId(), terms: useId(), notes: useId(), discount: useId() }
  const formatting = useRecordFormatting()
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const proposalsActive = useIsModuleActive(PROPOSALS_MODULE)
  const esignActive = useIsModuleActive(ESIGN_MODULE)
  const mutations = useQuoteMutations()
  const priceListsActive = useIsModuleActive(PRICE_LISTS_MODULE)
  const priceLists = useActivePriceLists(priceListsActive)
  const [header, setHeader] = useState<HeaderDraft>(() => headerFrom(quote))
  const [items, setItems] = useState<EditableItem[]>(() => quote.items.map(editableFromItem))
  const [dirty, setDirty] = useState(false)
  const [dialog, setDialog] = useState<'submit' | 'send' | 'accept' | 'signature' | 'version' | 'delete' | null>(null)

  const editable = quote.is_current && (quote.status === 'draft' || quote.status === 'approved') && hasPermission('quotes.update')
  const money = (value: string | null | undefined) => formatCurrency(value ?? '0', formatting, quote.currency)
  const currentVersion = quote.versions.find((version) => version.is_current)
  const proposalsEnabled = proposalsActive && hasPermission('proposals.generate')
  const esignEnabled = esignActive && hasPermission('esign.send')
  const availableLists = (priceLists.data ?? []).filter((list) => list.currency === quote.currency)

  const patchHeader = (patch: Partial<HeaderDraft>) => {
    setHeader((current) => ({ ...current, ...patch }))
    setDirty(true)
  }

  const request = (nextItems: EditableItem[], reprice = false): QuoteRequest => ({
    title: header.title,
    valid_until: header.valid_until || null,
    price_list_id: header.price_list_id || null,
    terms: header.terms || null,
    notes: header.notes || null,
    discount_type: header.discount_type,
    discount_value: header.discount_type === 'none' ? null : header.discount_value || '0',
    items: nextItems.map(itemRequest),
    reprice,
  })

  const save = async (nextItems: EditableItem[] = items, reprice = false) => {
    try {
      await mutations.update.mutateAsync({ quoteId: quote.id, request: request(nextItems, reprice), version: quote.updated_at })
      setDirty(false)
      notifySuccess(t('editor.saved'))
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const addProduct = (productId: string) => {
    const next = [...items, { key: crypto.randomUUID(), product_id: productId, quantity: '1' }]
    setItems(next)
    void save(next)
  }

  const copyLink = async () => {
    try {
      const { url } = await quotesApi.link(quote.id)
      await navigator.clipboard.writeText(url)
      notifySuccess(t('actions.linkCopied'))
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const generatePdf = async () => {
    try {
      await generateQuotePdf(quote.deal_id, quote.id, null)
      notifySuccess(t('actions.pdfGenerated'))
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const createVersion = async () => {
    try {
      const next = await mutations.createVersion.mutateAsync(quote.id)
      notifySuccess(t('actions.versionCreated'))
      void navigate(quotePath(next.id))
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const remove = async () => {
    try {
      await mutations.remove.mutateAsync(quote.id)
      notifySuccess(t('actions.deleted'))
      void navigate(`/deals/${encodeURIComponent(quote.deal_id)}`)
    } catch (error) {
      notifyQuoteError(error)
    }
  }

  const actions: ReactNode[] = []
  if (editable && dirty) {
    actions.push(
      <Button key="save" onClick={() => void save()} disabled={mutations.update.isPending}>
        <Save aria-hidden="true" />
        {t('editor.save')}
      </Button>,
    )
  }
  if (quote.is_current && !dirty) {
    if (quote.status === 'draft' && hasPermission('quotes.update')) {
      actions.push(
        <Button key="submit" variant="outline" onClick={() => setDialog('submit')}>
          <ShieldCheck aria-hidden="true" />
          {t('actions.submit')}
        </Button>,
      )
    }
    if (['draft', 'approved', 'sent'].includes(quote.status) && hasPermission('quotes.send')) {
      actions.push(
        <Button key="send" onClick={() => setDialog('send')}>
          <Send aria-hidden="true" />
          {quote.status === 'sent' ? t('actions.resend') : t('actions.send')}
        </Button>,
      )
    }
    if (['draft', 'approved', 'sent'].includes(quote.status) && hasPermission('quotes.update')) {
      actions.push(
        <Button key="accept" variant="outline" onClick={() => setDialog('accept')}>
          <CircleCheck aria-hidden="true" />
          {t('actions.accept')}
        </Button>,
      )
    }
    if (quote.public_url) {
      actions.push(
        <Button key="copy" variant="outline" onClick={() => void copyLink()}>
          <Copy aria-hidden="true" />
          {t('actions.copyLink')}
        </Button>,
        <Button key="open" variant="ghost" asChild>
          <a href={quote.public_url} target="_blank" rel="noreferrer">
            <ExternalLink aria-hidden="true" />
            {t('actions.openPublic')}
          </a>
        </Button>,
      )
    }
    actions.push(
      <Button key="download" variant="outline" asChild>
        <a href={quotePdfUrl(quote.id)} download>
          <Download aria-hidden="true" />
          {t('actions.downloadPdf')}
        </a>
      </Button>,
    )
    if (proposalsEnabled) {
      actions.push(
        <Button key="pdf" variant="outline" onClick={() => void generatePdf()}>
          <FileText aria-hidden="true" />
          {t('actions.pdf')}
        </Button>,
      )
    }
    if (esignEnabled && ['approved', 'sent', 'accepted'].includes(quote.status)) {
      actions.push(
        <Button key="sign" variant="outline" onClick={() => setDialog('signature')}>
          <PenLine aria-hidden="true" />
          {t('actions.signature')}
        </Button>,
      )
    }
    if (quote.status !== 'accepted' && quote.status !== 'draft' && hasPermission('quotes.create')) {
      actions.push(
        <Button key="version" variant="outline" onClick={() => setDialog('version')}>
          <FilePlus2 aria-hidden="true" />
          {t('actions.newVersion')}
        </Button>,
      )
    }
  }
  if (hasPermission('quotes.delete') && quote.status !== 'accepted') {
    actions.push(
      <Button key="delete" variant="ghost" onClick={() => setDialog('delete')} aria-label={t('actions.delete')}>
        <Trash2 aria-hidden="true" />
        {t('actions.delete')}
      </Button>,
    )
  }

  const primary = ['save', 'send', 'submit', 'accept'].map((key) => actions.find((action) => isValidElement(action) && action.key === key)).find(Boolean)
  const secondary = actions.filter((action) => action !== primary)

  return (
    <Page>
      <UnsavedChangesGuard when={dirty} />
      <PageHeader
        back={{ to: `/deals/${encodeURIComponent(quote.deal_id)}`, label: t('editor.back') }}
        title={quote.title}
        badges={<QuoteStatusBadge status={quote.status} />}
        description={<p>{t('numberVersion', { number: quote.number, version: quote.version })}</p>}
        actions={<>
          {primary}
          {secondary.length > 0 && <DropdownMenu>
            <DropdownMenuTrigger asChild><Button variant="ghost" size="icon" aria-label={t('actions.more')}><MoreHorizontal aria-hidden /></Button></DropdownMenuTrigger>
            <DropdownMenuContent align="end">{secondary.map((action, index) => isValidElement(action) ? <DropdownMenuItem key={action.key ?? index} asChild>{action}</DropdownMenuItem> : null)}</DropdownMenuContent>
          </DropdownMenu>}
        </>}
      />

      {!quote.is_current && currentVersion && (
        <div className="flex flex-wrap items-center gap-3 rounded-lg border bg-muted/40 p-3 text-sm">
          {t('editor.oldVersion')}
          <Button size="sm" variant="outline" asChild>
            <Link to={quotePath(currentVersion.id)}>{t('editor.goCurrent')}</Link>
          </Button>
        </div>
      )}
      {quote.is_current && !editable && quote.status !== 'pending_approval' && (
        <p className="text-sm text-muted-foreground">{t('editor.readOnly')}</p>
      )}

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="grid gap-6">
          <Card>
            <CardHeader>
              <CardTitle>{t('editor.header')}</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <Field label={t('editor.fields.title')} className="sm:col-span-2">
                {(control) => <Input placeholder={t('placeholders.title')} {...control} value={header.title} disabled={!editable} onChange={(event) => patchHeader({ title: event.target.value })} />}
              </Field>
              <Field label={t('editor.fields.validUntil')}>
                {(control) => <Input
                  {...control}
                  type="date"
                  value={header.valid_until}
                  disabled={!editable}
                  onChange={(event) => patchHeader({ valid_until: event.target.value })}
                />}
              </Field>
              <Field label={t('editor.fields.priceList')}>
                {(fieldControl) => <NativeSelect value={header.price_list_id || NO_PRICE_LIST}
                  disabled={!editable}
                  onValueChange={(value) => patchHeader({ price_list_id: value === NO_PRICE_LIST ? '' : value })} id={fieldControl.id}>
                    <option value={NO_PRICE_LIST}>{t('editor.fields.noPriceList')}</option>
                    {availableLists.map((list) => (
                      <option key={list.id} value={list.id}>
                        {list.name}
                      </option>
                    ))}
                  </NativeSelect>}
              </Field>
              <Field id={ids.discount} label={t('editor.fields.totalDiscount')}>
                <div className="flex gap-2">
                  <NativeSelect value={header.discount_type}
                    disabled={!editable}
                    onValueChange={(value) => patchHeader({ discount_type: value as QuoteDiscountType })} aria-label={t('editor.fields.discountType')} className="w-40">
                      {DISCOUNT_TYPES.map((type) => (
                        <option key={type} value={type}>
                          {t(`editor.discountTypes.${type}`)}
                        </option>
                      ))}
                    </NativeSelect>
                  <Input placeholder={t('placeholders.discount')}
                    id={ids.discount}
                    inputMode="decimal"
                    value={header.discount_value}
                    disabled={!editable || header.discount_type === 'none'}
                    onChange={(event) => patchHeader({ discount_value: event.target.value })}
                  />
                </div>
              </Field>
              <Field label={t('editor.fields.currency')}>
                {(control) => <Input placeholder={t('placeholders.currency')} {...control} value={quote.currency} disabled readOnly />}
              </Field>
              <Field label={t('editor.fields.terms')} className="sm:col-span-2">
                {(control) => <Textarea placeholder={t('placeholders.terms')} {...control} rows={3} value={header.terms} disabled={!editable} onChange={(event) => patchHeader({ terms: event.target.value })} />}
              </Field>
              <Field label={t('editor.fields.notes')} className="sm:col-span-2">
                {(control) => <Textarea placeholder={t('placeholders.notes')} {...control} rows={3} value={header.notes} disabled={!editable} onChange={(event) => patchHeader({ notes: event.target.value })} />}
              </Field>
            </CardContent>
          </Card>

          <QuoteItemsEditor
            currency={quote.currency}
            items={items}
            editable={editable}
            onChange={(next) => {
              setItems(next)
              setDirty(true)
            }}
            onAddProduct={addProduct}
          />
          {editable && header.price_list_id && items.some((item) => item.product_id) && (
            <Button className="w-fit" variant="outline" size="sm" onClick={() => void save(items, true)}>
              {t('editor.items.reprice')}
            </Button>
          )}
        </div>

        <div className="grid content-start gap-6">
          <Card>
            <CardContent className="grid gap-2 pt-6 text-sm">
              <Row label={t('editor.totals.subtotal')} value={money(quote.subtotal)} />
              <Row label={t('editor.totals.itemsDiscount')} value={`− ${money(quote.items_discount)}`} />
              <Row label={t('editor.totals.totalDiscount')} value={`− ${money(quote.total_discount)}`} />
              <Row label={t('editor.totals.tax')} value={money(quote.tax_total)} />
              <div className="flex items-center justify-between border-t pt-2 text-base font-semibold">
                <span>{t('editor.totals.total')}</span>
                <span className="tabular-nums">{money(quote.total)}</span>
              </div>
              <p className="text-xs text-muted-foreground">
                {t('editor.totals.maxDiscount', { percent: quote.max_discount_percent })}
                {quote.discount_limit != null && ` · ${t('editor.totals.limit', { percent: quote.discount_limit })}`}
              </p>
              {quote.exceeds_discount_limit && <p className="text-xs text-warning-foreground">{t('editor.totals.exceeds')}</p>}
              {dirty && <p className="text-xs text-muted-foreground">{t('editor.totals.serverNote')}</p>}
            </CardContent>
          </Card>

          {(quote.acceptance || quote.status === 'rejected') && (
            <Card>
              <CardHeader>
                <CardTitle>{t('editor.acceptance')}</CardTitle>
              </CardHeader>
              <CardContent className="grid gap-2 text-sm">
                {quote.acceptance && (
                  <>
                    <p>
                      {quote.acceptance.email
                        ? t('editor.acceptedBy', {
                            name: quote.acceptance.name,
                            email: quote.acceptance.email,
                            date: formatDateTime(quote.accepted_at, formatting),
                          })
                        : t('editor.acceptedByNoEmail', {
                            name: quote.acceptance.name,
                            date: formatDateTime(quote.accepted_at, formatting),
                          })}
                    </p>
                    {quote.acceptance.recorded_by && (
                      <p className="text-xs text-muted-foreground">{t('editor.acceptedManually')}</p>
                    )}
                    {quote.acceptance.ip && (
                      <p className="text-xs text-muted-foreground">{t('editor.acceptedFrom', { ip: quote.acceptance.ip })}</p>
                    )}
                    {quote.deal_sync_error ? (
                      <SharedFieldError>{t('editor.syncError', { error: quote.deal_sync_error })}</SharedFieldError>
                    ) : quote.deal_synced_at ? (
                      <p className="text-xs text-muted-foreground">{t('editor.synced')}</p>
                    ) : (
                      <p className="text-xs text-muted-foreground">{t('editor.syncPending')}</p>
                    )}
                  </>
                )}
                {quote.status === 'rejected' && (
                  <p>
                    {quote.rejection_reason
                      ? t('editor.rejectedReason', { reason: quote.rejection_reason })
                      : t('editor.rejectedNoReason')}
                  </p>
                )}
              </CardContent>
            </Card>
          )}

          <Card>
            <CardHeader>
              <CardTitle>{t('editor.versions')}</CardTitle>
            </CardHeader>
            <CardContent>
              <ul className="grid gap-1 text-sm">
                {quote.versions.map((version) => (
                  <li key={version.id}>
                    <Link
                      to={quotePath(version.id)}
                      className={`flex items-center justify-between gap-2 rounded-md px-2 py-1 hover:bg-muted ${version.id === quote.id ? 'bg-muted font-medium' : ''}`}
                    >
                      <span>
                        {t('editor.versionLabel', { version: version.version })}
                        {version.is_current && ` (${t('editor.current')})`}
                      </span>
                      <span className="flex items-center gap-2">
                        <span className="text-xs text-muted-foreground">{formatDate(version.created_at, formatting)}</span>
                        <QuoteStatusBadge status={version.status} />
                      </span>
                    </Link>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>

          <Slot name="quotes.editor.sidebar" props={{ recordType: QUOTE_RECORD_TYPE, recordId: quote.id }} />
        </div>
      </div>

      {dialog === 'submit' && <SubmitQuoteDialog quote={quote} open onOpenChange={(open) => !open && setDialog(null)} />}
      {dialog === 'accept' && <AcceptQuoteDialog quote={quote} open onOpenChange={(open) => !open && setDialog(null)} />}
      {dialog === 'send' && <SendQuoteDialog quote={quote} open onOpenChange={(open) => !open && setDialog(null)} />}
      {dialog === 'signature' && (
        <SignatureDialog
          quote={quote}
          proposalsEnabled={proposalsActive}
          open
          onOpenChange={(open) => !open && setDialog(null)}
        />
      )}
      <ConfirmDialog
        open={dialog === 'version'}
        onOpenChange={(open) => !open && setDialog(null)}
        title={t('actions.newVersion')}
        description={t('actions.newVersionConfirm')}
        confirmLabel={t('actions.confirm')}
        cancelLabel={t('actions.cancel')}
        pending={mutations.createVersion.isPending}
        onConfirm={createVersion}
      />
      <ConfirmDialog
        open={dialog === 'delete'}
        onOpenChange={(open) => !open && setDialog(null)}
        title={t('actions.delete')}
        description={t('actions.deleteConfirm')}
        confirmLabel={t('actions.confirm')}
        cancelLabel={t('actions.cancel')}
        variant="destructive"
        pending={mutations.remove.isPending}
        onConfirm={remove}
      />
    </Page>
  )
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-muted-foreground">{label}</span>
      <span className="tabular-nums">{value}</span>
    </div>
  )
}

