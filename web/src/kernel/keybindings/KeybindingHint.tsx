import { useTranslation } from 'react-i18next'
import { KbdChord } from '@/components/ui/kbd'
import { formatChord } from './chord'
import { useActionKeys } from './hooks'

export interface KeybindingKeysProps {
  keys: string[]
  size?: 'sm' | 'md'
  className?: string
  limit?: number
}

export function KeybindingKeys({ keys, size, className, limit = keys.length }: KeybindingKeysProps) {
  const { t } = useTranslation('keybindings')
  if (keys.length === 0) return null
  return (
    <span className={className ? `inline-flex flex-wrap items-center gap-1.5 ${className}` : 'inline-flex flex-wrap items-center gap-1.5'}>
      {keys.slice(0, limit).map((chord, index) => (
        <span key={chord} className="inline-flex items-center gap-1.5">
          {index > 0 && <span className="text-xs text-muted-foreground">{t('or')}</span>}
          <KbdChord steps={formatChord(chord)} separator={t('then')} size={size} />
        </span>
      ))}
    </span>
  )
}

export interface KeybindingHintProps {
  action: string
  size?: 'sm' | 'md'
  className?: string
  limit?: number
}

export function KeybindingHint({ action, size, className, limit = 1 }: KeybindingHintProps) {
  const keys = useActionKeys(action)
  return <KeybindingKeys keys={keys} size={size} className={className} limit={limit} />
}
