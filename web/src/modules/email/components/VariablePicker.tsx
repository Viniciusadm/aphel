import { Braces } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useTemplateVariables } from '../api'

interface VariablePickerProps {
  recordType?: string | null
  onInsert: (token: string) => void
  disabled?: boolean
}

export function VariablePicker({ recordType, onInsert, disabled }: VariablePickerProps) {
  const { t } = useTranslation('email')
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const variables = useTemplateVariables(recordType)
  const term = search.trim().toLowerCase()
  const options = (variables.data ?? []).filter(
    (variable) => !term || variable.label.toLowerCase().includes(term) || variable.key.toLowerCase().includes(term),
  )

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" disabled={disabled}>
          <Braces aria-hidden="true" />
          {t('templates.insertVariable')}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="grid w-72 gap-2 p-2">
        <Input
          value={search}
          placeholder={t('templates.searchVariables')}
          aria-label={t('templates.searchVariables')}
          onChange={(event) => setSearch(event.target.value)}
        />
        <div className="grid max-h-64 overflow-y-auto">
          {variables.isPending && <p className="p-2 text-sm text-muted-foreground">{t('common.loading')}</p>}
          {variables.isError && <p className="p-2 text-sm text-destructive">{t('templates.variablesError')}</p>}
          {variables.isSuccess && options.length === 0 && (
            <p className="p-2 text-sm text-muted-foreground">{t('templates.noVariables')}</p>
          )}
          {options.map((variable) => (
            <button
              key={variable.key}
              type="button"
              className="grid rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
              onClick={() => {
                onInsert(`{{${variable.key}}}`)
                setOpen(false)
              }}
            >
              <span>{variable.label}</span>
              <code className="text-xs text-muted-foreground">{`{{${variable.key}}}`}</code>
            </button>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  )
}
