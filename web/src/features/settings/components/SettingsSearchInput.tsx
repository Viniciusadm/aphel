import { Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

interface SettingsSearchInputProps {
  value: string
  onChange: (value: string) => void
  className?: string
}

export function SettingsSearchInput({ value, onChange, className }: SettingsSearchInputProps) {
  const { t } = useTranslation('settings')
  return (
    <div className={cn('relative', className)}>
      <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t('search.placeholder')}
        aria-label={t('search.label')}
        className="pl-9"
      />
    </div>
  )
}
