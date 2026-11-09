import { Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { KeybindingHint, ariaKeyShortcut, useActionKeys } from '@/kernel/keybindings'
import { cn } from '@/lib/utils'
import { useCommandPalette } from './context'

export function PaletteTrigger({ className }: { className?: string }) {
  const { t } = useTranslation('command-palette')
  const { openPalette } = useCommandPalette()
  const keys = useActionKeys('core.palette')

  return (
    <button
      type="button"
      onClick={() => openPalette()}
      aria-label={t('trigger.label')}
      aria-keyshortcuts={ariaKeyShortcut(keys)}
      aria-haspopup="dialog"
      className={cn(
        'flex h-9 w-full min-w-0 items-center gap-2 rounded-md border border-input bg-background px-3 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
        className,
      )}
    >
      <Search className="size-4 shrink-0" aria-hidden />
      <span className="min-w-0 flex-1 truncate text-left">{t('trigger.placeholder')}</span>
      <span className="hidden sm:inline-flex" aria-hidden>
        <KeybindingHint action="core.palette" />
      </span>
    </button>
  )
}
