import { PackagePlus, Search } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Skeleton } from '@/components/ui/skeleton'
import { useProductPicker, type Product } from './api'
import { formatCurrency, useRecordFormatting } from '@/features/records/formatting'

export interface ProductPickerProps {
  currency: string
  disabled?: boolean
  onSelect: (product: Product) => void
}

export function ProductPicker({ currency, disabled, onSelect }: ProductPickerProps) {
  const { t } = useTranslation('products')
  const formatting = useRecordFormatting()
  const searchId = useId()
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const products = useProductPicker(search, open)
  const items = products.data?.data ?? []

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setSearch('')
      }}
    >
      <PopoverTrigger asChild>
        <Button size="sm" disabled={disabled}>
          <PackagePlus aria-hidden="true" />
          {t('items.addProduct')}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-2">
        <div className="relative">
          <Search
            className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden="true"
          />
          <Input
            id={searchId}
            autoFocus
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t('items.searchPlaceholder')}
            aria-label={t('items.searchPlaceholder')}
            className="pl-8"
          />
        </div>
        <div className="mt-2 max-h-72 overflow-y-auto" role="listbox" aria-label={t('items.catalog')}>
          {products.isPending ? (
            <div className="grid gap-1 p-1" aria-busy="true">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : products.isError ? (
            <p className="p-2 text-sm text-destructive">{t('items.catalogError')}</p>
          ) : items.length === 0 ? (
            <p className="p-2 text-sm text-muted-foreground">{t('items.catalogEmpty')}</p>
          ) : (
            items.map((product) => {
              const otherCurrency = product.currency !== currency
              return (
                <button
                  key={product.id}
                  type="button"
                  role="option"
                  aria-selected={false}
                  disabled={otherCurrency}
                  onClick={() => {
                    onSelect(product)
                    setOpen(false)
                    setSearch('')
                  }}
                  className="flex w-full min-w-0 items-center justify-between gap-3 rounded-md px-2 py-2 text-left text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <span className="grid min-w-0 gap-0.5">
                    <span className="truncate font-medium">{product.name}</span>
                    <span className="truncate text-xs text-muted-foreground">
                      {otherCurrency
                        ? t('items.otherCurrency', { currency: product.currency })
                        : [product.code, product.unit].filter(Boolean).join(' · ')}
                    </span>
                  </span>
                  <span className="shrink-0 tabular-nums text-muted-foreground">
                    {formatCurrency(product.price, formatting, product.currency)}
                  </span>
                </button>
              )
            })
          )}
        </div>
      </PopoverContent>
    </Popover>
  )
}
