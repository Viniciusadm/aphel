import { Loader2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { cn } from '@/lib/utils'
import { useFormulaCatalog, type FormulaCatalog, type FormulaFunction } from './api'
import { functionDescription } from './messages'
import { SearchInput } from '@/components/ui/search-input'

export interface FormulaSnippet {
  text: string
  caret?: number
}

interface FormulaCatalogPanelProps {
  recordType: string
  onInsert: (snippet: FormulaSnippet) => void
}

const CATEGORY_ORDER = ['date', 'logic', 'text', 'number', 'aggregate'] as const
type Category = (typeof CATEGORY_ORDER)[number]
const RELATED_AGGREGATES = ['sum', 'avg', 'min', 'max'] as const

function isCategory(value: string): value is Category {
  return (CATEGORY_ORDER as readonly string[]).includes(value)
}

function matches(term: string, ...values: (string | null | undefined)[]): boolean {
  if (!term) return true
  return values.some((value) => value?.toLowerCase().includes(term))
}

function functionSnippet(fn: FormulaFunction): FormulaSnippet {
  const text = `${fn.name}()`
  return fn.signature.startsWith(text) ? { text } : { text, caret: fn.name.length + 1 }
}

const ITEM_CLASS =
  'flex w-full min-w-0 items-start gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted focus-visible:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none'
const CHIP_CLASS =
  'rounded border px-1.5 py-0.5 font-mono text-xs hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none'

export function FormulaCatalogPanel({ recordType, onInsert }: FormulaCatalogPanelProps) {
  const { t } = useTranslation('formulas')
  const catalog = useFormulaCatalog(recordType)
  const [search, setSearch] = useState('')
  const term = search.trim().toLowerCase()

  return (
    <section className="grid min-w-0 gap-2 rounded-md border p-3" aria-labelledby="formula-catalog-title">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 id="formula-catalog-title" className="text-sm font-medium">
          {t('catalog.title')}
        </h3>
        <SearchInput wrapperClassName="w-full sm:w-48"
            value={search}
            aria-label={t('catalog.search')}
            placeholder={t('catalog.search')}
            className="h-8"
            onValueChange={(value) => setSearch(value)}
          />
      </div>
      {catalog.isPending ? (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" aria-hidden="true" />
          {t('catalog.loading')}
        </p>
      ) : catalog.isError ? (
        <p role="alert" className="text-sm text-destructive">
          {t('catalog.loadError')}
        </p>
      ) : (
        <CatalogTabs catalog={catalog.data} term={term} onInsert={onInsert} />
      )}
    </section>
  )
}

interface CatalogTabsProps {
  catalog: FormulaCatalog
  term: string
  onInsert: (snippet: FormulaSnippet) => void
}

function CatalogTabs({ catalog, term, onInsert }: CatalogTabsProps) {
  const { t, i18n } = useTranslation('formulas')
  const fields = catalog.fields.filter((field) => matches(term, field.key, field.label))
  const relations = catalog.relations.flatMap((relation) => {
    const relationMatches = matches(term, relation.key, relation.label)
    const relatedFields = relationMatches
      ? relation.fields
      : relation.fields.filter((field) => matches(term, field.key, field.label))
    return relationMatches || relatedFields.length > 0 ? [{ ...relation, fields: relatedFields }] : []
  })
  const functions = catalog.functions.filter((fn) =>
    matches(term, fn.name, fn.signature, functionDescription(fn.name, t, (key) => i18n.exists(key))),
  )
  const grouped = new Map<string, FormulaFunction[]>()
  for (const fn of functions) grouped.set(fn.category, [...(grouped.get(fn.category) ?? []), fn])
  const categories = [...grouped.keys()].sort(
    (left, right) =>
      (isCategory(left) ? CATEGORY_ORDER.indexOf(left) : CATEGORY_ORDER.length) -
      (isCategory(right) ? CATEGORY_ORDER.indexOf(right) : CATEGORY_ORDER.length),
  )
  const empty = <p className="px-2 py-1.5 text-sm text-muted-foreground">{t('catalog.empty')}</p>
  const listClass = 'grid max-h-60 min-w-0 content-start gap-0.5 overflow-y-auto pr-1'

  return (
    <Tabs defaultValue="fields" className="grid min-w-0 gap-2">
      <TabsList>
        <TabsTrigger value="fields">{t('catalog.fields')}</TabsTrigger>
        <TabsTrigger value="relations">{t('catalog.relations')}</TabsTrigger>
        <TabsTrigger value="functions">{t('catalog.functions')}</TabsTrigger>
      </TabsList>
      <TabsContent value="fields" className={listClass}>
        {fields.length === 0
          ? empty
          : fields.map((field) => (
              <button
                key={field.key}
                type="button"
                className={ITEM_CLASS}
                onClick={() => onInsert({ text: field.key })}
              >
                <span className="grid min-w-0 flex-1 gap-0.5">
                  <span className="truncate font-medium">{field.label}</span>
                  <code className="truncate text-xs text-muted-foreground">{field.key}</code>
                </span>
                <span className="flex shrink-0 flex-wrap justify-end gap-1">
                  <Badge variant="outline">{t(`kinds.${field.kind}`)}</Badge>
                  {field.is_formula && <Badge variant="secondary">{t('catalog.formula')}</Badge>}
                </span>
              </button>
            ))}
      </TabsContent>
      <TabsContent value="relations" className={listClass}>
        {catalog.relations.length === 0 ? (
          <p className="px-2 py-1.5 text-sm text-muted-foreground">{t('catalog.noRelations')}</p>
        ) : relations.length === 0 ? (
          empty
        ) : (
          relations.map((relation) => (
            <div key={relation.key} className="grid min-w-0 gap-1 rounded-md border px-2 py-2">
              <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
                <span className="grid min-w-0 gap-0.5">
                  <span className="truncate text-sm font-medium">{relation.label}</span>
                  <code className="truncate text-xs text-muted-foreground">{relation.key}</code>
                </span>
                <button
                  type="button"
                  className={CHIP_CLASS}
                  aria-label={t('catalog.insert', { item: `count(${relation.key})` })}
                  title={t('catalog.count')}
                  onClick={() => onInsert({ text: `count(${relation.key})` })}
                >
                  {`count(${relation.key})`}
                </button>
              </div>
              {relation.fields.length > 0 && (
                <ul className="grid gap-1" aria-label={t('catalog.relationFields', { relation: relation.label })}>
                  {relation.fields.map((field) => (
                    <li key={field.key} className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-sm">
                      <span className="min-w-0 flex-1 truncate">
                        {field.label} <code className="text-xs text-muted-foreground">{field.key}</code>
                      </span>
                      <span className="flex flex-wrap gap-1">
                        {RELATED_AGGREGATES.map((aggregate) => {
                          const text = `${aggregate}(${relation.key}.${field.key})`
                          return (
                            <button
                              key={aggregate}
                              type="button"
                              className={CHIP_CLASS}
                              aria-label={t('catalog.insert', { item: text })}
                              title={text}
                              onClick={() => onInsert({ text })}
                            >
                              {aggregate}
                            </button>
                          )
                        })}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))
        )}
      </TabsContent>
      <TabsContent value="functions" className={listClass}>
        {categories.length === 0
          ? empty
          : categories.map((category) => (
              <div key={category} className="grid gap-0.5">
                <h4 className="px-2 pt-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                  {isCategory(category) ? t(`catalog.categories.${category}`) : category}
                </h4>
                {(grouped.get(category) ?? []).map((fn) => {
                  const description = functionDescription(fn.name, t, (key) => i18n.exists(key))
                  return (
                    <button
                      key={fn.name}
                      type="button"
                      className={cn(ITEM_CLASS, 'flex-col gap-0.5')}
                      onClick={() => onInsert(functionSnippet(fn))}
                    >
                      <code className="max-w-full font-mono text-xs break-all">{fn.signature}</code>
                      {description && <span className="text-xs text-muted-foreground">{description}</span>}
                    </button>
                  )
                })}
              </div>
            ))}
      </TabsContent>
    </Tabs>
  )
}
