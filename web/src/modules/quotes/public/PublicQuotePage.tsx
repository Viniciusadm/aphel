import { PageTitle } from '@/components/ui/page-title'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { CheckCircle2, LinkIcon, Printer, XCircle } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { isApiError } from '@/api'
import { PageLoader } from '@/components/feedback'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { usePublicQuote, usePublicQuoteAnswer } from './api'
import type { PublicQuote, PublicQuoteState } from './types'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'

const STATE_VARIANTS: Record<PublicQuoteState, BadgeProps['variant']> = {
  open: 'default',
  accepted: 'success',
  rejected: 'destructive',
  expired: 'secondary',
  superseded: 'secondary',
}

type ErrorKey = 'errors.quote_closed' | 'errors.quote_expired' | 'errors.quote_superseded' | 'errors.rateLimited' | 'errors.generic'

function errorKey(error: unknown): ErrorKey {
  if (!isApiError(error)) return 'errors.generic'
  if (error.code === 'quote_closed') return 'errors.quote_closed'
  if (error.code === 'quote_expired') return 'errors.quote_expired'
  if (error.code === 'quote_superseded') return 'errors.quote_superseded'
  if (error.status === 429) return 'errors.rateLimited'
  return 'errors.generic'
}

function useFormat(currency: string) {
  const { i18n } = useTranslation()
  const locale = i18n.language || 'pt-BR'
  const money = new Intl.NumberFormat(locale, { style: 'currency', currency })
  return {
    money: (value: string) => money.format(Number(value)),
    number: (value: string) => new Intl.NumberFormat(locale, { maximumFractionDigits: 4 }).format(Number(value)),
    date: (value: string) => new Intl.DateTimeFormat(locale, { dateStyle: 'long', timeZone: 'UTC' }).format(new Date(`${value}T00:00:00Z`)),
    dateTime: (value: string) => new Intl.DateTimeFormat(locale, { dateStyle: 'long', timeStyle: 'short' }).format(new Date(value)),
  }
}

function Layout({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation('quotes', { keyPrefix: 'public' })
  return (
    <div className="flex min-h-dvh flex-col bg-muted/40 text-foreground print:bg-white">
      <main className="flex flex-1 items-start justify-center px-4 py-8 sm:py-12 print:p-0">
        <div className="w-full max-w-4xl rounded-xl border bg-card text-card-foreground shadow-xs print:border-0 print:shadow-none">
          {children}
        </div>
      </main>
      <footer className="pb-6 text-center text-xs text-muted-foreground print:hidden">{t('poweredBy')}</footer>
    </div>
  )
}

export function PublicQuotePage() {
  const { t } = useTranslation('quotes', { keyPrefix: 'public' })
  const { token = '' } = useParams()
  const query = usePublicQuote(token)
  if (query.isPending) return <PageLoader />
  if (query.isError) {
    return (
      <Layout>
        <div className="p-6">
          <EmptyState icon={LinkIcon} title={t('invalid')} className="border-0" />
        </div>
      </Layout>
    )
  }
  return (
    <Layout>
      <QuoteDocument token={token} quote={query.data} />
    </Layout>
  )
}

function QuoteDocument({ token, quote }: { token: string; quote: PublicQuote }) {
  const { t } = useTranslation('quotes', { keyPrefix: 'public' })
  const format = useFormat(quote.currency)
  const accent = quote.organization.primary_color ?? undefined

  return (
    <article className="grid gap-6 p-6 sm:p-10">
      <header className="flex flex-wrap items-start justify-between gap-4 border-b pb-6" style={{ borderColor: accent }}>
        <div className="grid gap-1">
          <p className="text-sm font-semibold" style={{ color: accent }}>
            {quote.organization.name}
          </p>
          <PageTitle>{quote.title}</PageTitle>
          <p className="text-sm text-muted-foreground">{t('number', { number: quote.number, version: quote.version })}</p>
          {(quote.contact_name || quote.company_name) && (
            <p className="text-sm">
              {t('preparedFor', { name: [quote.contact_name, quote.company_name].filter(Boolean).join(' — ') })}
            </p>
          )}
          {quote.owner_name && (
            <p className="text-sm text-muted-foreground">
              {t('preparedBy', { name: quote.owner_name })}
              {quote.owner_email && ` · ${quote.owner_email}`}
            </p>
          )}
        </div>
        <div className="grid justify-items-end gap-2">
          <Badge variant={STATE_VARIANTS[quote.state]}>{t(`state.${quote.state}`)}</Badge>
          {quote.valid_until && <p className="text-sm text-muted-foreground">{t('validUntil', { date: format.date(quote.valid_until) })}</p>}
          <Button size="sm" variant="outline" className="print:hidden" onClick={() => window.print()}>
            <Printer aria-hidden="true" />
            {t('print')}
          </Button>
        </div>
      </header>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="pr-3">{t('columns.item')}</TableHead>
            <TableHead className="text-right">{t('columns.quantity')}</TableHead>
            <TableHead className="text-right">{t('columns.unitPrice')}</TableHead>
            <TableHead className="text-right">{t('columns.discount')}</TableHead>
            <TableHead className="text-right">{t('columns.tax')}</TableHead>
            <TableHead className="pl-3 text-right">{t('columns.total')}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {quote.items.map((item, index) => (
            <TableRow key={`${item.name}-${index}`} className="align-top break-inside-avoid">
              <TableCell className="pr-3">
                <p className="font-medium">{item.name}</p>
                {item.description && <p className="text-xs whitespace-pre-line text-muted-foreground">{item.description}</p>}
                {item.components.length > 0 && (
                  <p className="text-xs text-muted-foreground">
                    {t('includes', {
                      components: item.components.map((component) => `${format.number(component.quantity)}× ${component.name}`).join(', '),
                    })}
                  </p>
                )}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {format.number(item.quantity)}
                {item.unit && ` ${item.unit}`}
              </TableCell>
              <TableCell className="text-right tabular-nums">{format.money(item.unit_price)}</TableCell>
              <TableCell className="text-right tabular-nums">
                {Number(item.discount_amount) > 0 ? `− ${format.money(item.discount_amount)}` : '—'}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {Number(item.tax_percent) > 0 ? `${format.number(item.tax_percent)}%` : '—'}
              </TableCell>
              <TableCell className="pl-3 text-right font-medium tabular-nums">{format.money(item.total)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      <dl className="ml-auto grid w-full max-w-xs gap-1 text-sm">
        <Total label={t('totals.subtotal')} value={format.money(quote.subtotal)} />
        {Number(quote.items_discount) > 0 && <Total label={t('totals.itemsDiscount')} value={`− ${format.money(quote.items_discount)}`} />}
        {Number(quote.total_discount) > 0 && <Total label={t('totals.totalDiscount')} value={`− ${format.money(quote.total_discount)}`} />}
        {Number(quote.tax_total) > 0 && <Total label={t('totals.tax')} value={format.money(quote.tax_total)} />}
        <div className="flex justify-between border-t pt-2 text-base font-semibold">
          <dt>{t('totals.total')}</dt>
          <dd className="tabular-nums">{format.money(quote.total)}</dd>
        </div>
      </dl>

      {quote.terms && (
        <section className="grid gap-1 break-inside-avoid">
          <h2 className="text-sm font-semibold">{t('terms')}</h2>
          <p className="text-sm whitespace-pre-line">{quote.terms}</p>
        </section>
      )}
      {quote.notes && (
        <section className="grid gap-1 break-inside-avoid">
          <h2 className="text-sm font-semibold">{t('notes')}</h2>
          <p className="text-sm whitespace-pre-line">{quote.notes}</p>
        </section>
      )}

      <Answer token={token} quote={quote} />
    </article>
  )
}

function Total({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="tabular-nums">{value}</dd>
    </div>
  )
}

function Answer({ token, quote }: { token: string; quote: PublicQuote }) {
  const { t } = useTranslation('quotes', { keyPrefix: 'public' })
  const format = useFormat(quote.currency)
  const answer = usePublicQuoteAnswer(token)
  const [name, setName] = useState(quote.contact_name ?? '')
  const [email, setEmail] = useState('')
  const [rejecting, setRejecting] = useState(false)
  const [reason, setReason] = useState('')
  const error = answer.accept.error ?? answer.reject.error

  if (quote.state === 'accepted') {
    return (
      <p className="flex items-center gap-2 rounded-lg border border-success/40 bg-success/10 p-4 text-sm">
        <CheckCircle2 className="size-5 shrink-0" aria-hidden="true" />
        {t('accept.done', { date: quote.accepted_at ? format.dateTime(quote.accepted_at) : '', name: quote.accepted_name ?? '' })}
      </p>
    )
  }
  if (quote.state === 'rejected') {
    return (
      <p className="flex items-center gap-2 rounded-lg border p-4 text-sm">
        <XCircle className="size-5 shrink-0" aria-hidden="true" />
        {t('reject.done', { date: quote.rejected_at ? format.dateTime(quote.rejected_at) : '' })}
      </p>
    )
  }
  if (quote.state === 'expired') {
    return <p className="rounded-lg border p-4 text-sm">{t('expired', { name: quote.owner_name ?? quote.organization.name })}</p>
  }
  if (quote.state === 'superseded') {
    return <p className="rounded-lg border p-4 text-sm">{t('superseded')}</p>
  }

  return (
    <section className="grid gap-4 rounded-lg border p-4 print:hidden">
      {rejecting ? (
        <form
          className="grid gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            answer.reject.mutate(reason.trim() || null)
          }}
        >
          <h2 className="text-base font-semibold">{t('reject.title')}</h2>
          <Field label={t('reject.reason')}>
            {(control) => <Textarea placeholder={t('placeholders.reason')} {...control} rows={3} value={reason} onChange={(event) => setReason(event.target.value)} />}
          </Field>
          <div className="flex flex-wrap gap-2">
            <Button type="submit" variant="destructive" disabled={answer.reject.isPending}>
              {t('reject.submit')}
            </Button>
            <Button type="button" variant="ghost" onClick={() => setRejecting(false)}>
              {t('accept.title')}
            </Button>
          </div>
        </form>
      ) : (
        <form
          className="grid gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            answer.accept.mutate({ name: name.trim(), email: email.trim() })
          }}
        >
          <div className="grid gap-1">
            <h2 className="text-base font-semibold">{t('accept.title')}</h2>
            <p className="text-sm text-muted-foreground">{t('accept.description')}</p>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label={t('accept.name')}>
              {(control) => <Input placeholder={t('placeholders.name')} {...control} required autoComplete="name" value={name} onChange={(event) => setName(event.target.value)} />}
            </Field>
            <Field label={t('accept.email')}>
              {(control) => <Input placeholder={t('placeholders.email')}
                {...control}
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />}
            </Field>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button type="submit" disabled={answer.accept.isPending || !name.trim() || !email.trim()}>
              {t('accept.submit')}
            </Button>
            <Button type="button" variant="outline" onClick={() => setRejecting(true)}>
              {t('reject.open')}
            </Button>
          </div>
        </form>
      )}
      {error && (
        <SharedFieldError>
          {t(errorKey(error))}
        </SharedFieldError>
      )}
    </section>
  )
}
