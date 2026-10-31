import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { VARIABLE_GROUPS, filterVariables, type TemplateVariable } from '../lib/variables'
import { SearchInput } from '@/components/ui/search-input'

export interface VariableListProps {
  variables: TemplateVariable[]
  onPick: (variable: TemplateVariable) => void
}

export function VariableList({ variables, onPick }: VariableListProps) {
  const { t } = useTranslation('automations')
  const [term, setTerm] = useState('')
  const visible = useMemo(() => filterVariables(variables, term), [term, variables])

  return (
    <div className="grid">
      <SearchInput wrapperClassName="border-b p-2"
          value={term}
          onValueChange={(value) => setTerm(value)}
          placeholder={t('variables.search')}
          aria-label={t('variables.search')}
          
          autoFocus
        />
      <div className="max-h-72 overflow-y-auto p-1">
        {visible.length === 0 ? (
          <p className="px-2 py-3 text-sm text-muted-foreground">{t('variables.empty')}</p>
        ) : (
          VARIABLE_GROUPS.map((group) => {
            const items = visible.filter((variable) => variable.group === group)
            if (items.length === 0) return null
            return (
              <div key={group} role="group" aria-label={t(`variables.groups.${group}`)} className="grid">
                <p className="px-2 pt-2 pb-1 text-xs font-medium text-muted-foreground uppercase">
                  {t(`variables.groups.${group}`)}
                </p>
                {items.map((variable) => (
                  <button
                    key={variable.path}
                    type="button"
                    onClick={() => onPick(variable)}
                    className="grid gap-0.5 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:outline-none"
                  >
                    <span className="truncate">{variable.label}</span>
                    <code className="truncate text-xs text-muted-foreground">{`{{${variable.path}}}`}</code>
                  </button>
                ))}
              </div>
            )
          })
        )}
      </div>
    </div>
  )
}
