import { SlidersHorizontal } from 'lucide-react'
import { Button } from './button'
import { Popover, PopoverContent, PopoverTrigger } from './popover'
import { PageSizeSelect } from './page-size-select'
import { PaginationSummary } from './pagination-summary'
import { useUiLabels } from './labels-context'
import { defaultUiLabels } from './default-labels'
import { useState, type ReactNode } from 'react'
import { EmptyState } from './empty-state'
import { NativeSelect } from './native-select'
import { Pagination } from './pagination'
import { SearchInput } from './search-input'

export interface CollectionViewProps<T> {
  items: readonly T[]
  searchLabel?: string
  emptyTitle: string
  getSearchText?: (item: T) => string
  filterLabel?: string
  allLabel?: string
  filters?: { value: string; label: string }[]
  getFilterValue?: (item: T) => string
  children: (items: T[]) => ReactNode
}

export function CollectionView<T>({ items, searchLabel, emptyTitle, getSearchText, filterLabel, allLabel, filters, getFilterValue, children }: CollectionViewProps<T>) {
  const labels = useUiLabels()
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState('')
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useState(25)
  const query = search.trim().toLocaleLowerCase()
  const activeFilter = filters?.some((option) => option.value === filter) ? filter : ''
  const filtered = items.filter((item) => (!query || (getSearchText?.(item) ?? '').toLocaleLowerCase().includes(query)) && (!activeFilter || getFilterValue?.(item) === activeFilter))
  const currentPage = Math.min(page, Math.max(1, Math.ceil(filtered.length / perPage)))
  return (
    <div className="grid min-w-0 gap-4">
      <div className="flex flex-wrap items-center gap-2">
        {searchLabel && <SearchInput wrapperClassName="min-w-48 flex-1" aria-label={searchLabel} placeholder={searchLabel} value={search} onValueChange={(value) => { setSearch(value); setPage(1) }} />}
        {filters && getFilterValue && <NativeSelect aria-label={filterLabel} value={activeFilter} onChange={(event) => { setFilter(event.target.value); setPage(1) }}>
          <option value="">{allLabel}</option>
          {filters.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
        </NativeSelect>}
        <Popover><PopoverTrigger asChild><Button variant="outline" size="sm" className="ml-auto"><SlidersHorizontal aria-hidden />{labels.displaySettings ?? defaultUiLabels().displaySettings}</Button></PopoverTrigger><PopoverContent align="end"><PageSizeSelect value={perPage} onChange={(value) => { setPerPage(value); setPage(1) }} /></PopoverContent></Popover>
      </div>
      <PaginationSummary page={currentPage} perPage={perPage} total={filtered.length} />
      {filtered.length ? children(filtered.slice((currentPage - 1) * perPage, currentPage * perPage)) : <EmptyState title={emptyTitle} />}
      <Pagination page={currentPage} perPage={perPage} total={filtered.length} onPageChange={setPage} />
    </div>
  )
}
