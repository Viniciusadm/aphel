import { useTranslation } from 'react-i18next'
import { SearchInput } from '@/components/ui/search-input'

interface SettingsSearchInputProps {
  value: string
  onChange: (value: string) => void
  className?: string
}

export function SettingsSearchInput({ value, onChange, className }: SettingsSearchInputProps) {
  const { t } = useTranslation('settings')
  return (
    <SearchInput
      value={value}
      onValueChange={onChange}
      placeholder={t('search.placeholder')}
      aria-label={t('search.label')}
      wrapperClassName={className}
    />
  )
}
