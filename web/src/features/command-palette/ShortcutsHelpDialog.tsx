import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { GO_SHORTCUTS } from '@/app/shortcuts'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useNavigationEntries } from '@/kernel/modules/navigation'
import { modifierKeyLabel } from './platform'

export interface ShortcutsHelpDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  enabled: boolean
}

interface ShortcutRow {
  id: string
  keys: string[][]
  label: string
}

function Keys({ sequence, separator }: { sequence: string[][]; separator: string }) {
  return (
    <span className="flex shrink-0 flex-wrap items-center gap-1">
      {sequence.map((combo, index) => (
        <span key={combo.join('+')} className="flex items-center gap-1">
          {index > 0 && <span className="text-xs text-muted-foreground">{separator}</span>}
          {combo.map((key) => (
            <kbd
              key={key}
              className="inline-flex h-6 min-w-6 items-center justify-center rounded border bg-muted px-1.5 font-mono text-xs"
            >
              {key}
            </kbd>
          ))}
        </span>
      ))}
    </span>
  )
}

export function ShortcutsHelpDialog({ open, onOpenChange, enabled }: ShortcutsHelpDialogProps) {
  const { t } = useTranslation('command-palette')
  const entries = useNavigationEntries()
  const modifier = modifierKeyLabel()

  const general: ShortcutRow[] = [
    { id: 'palette', keys: [[modifier, 'K']], label: t('help.items.palette') },
    { id: 'create', keys: [['C']], label: t('help.items.create') },
    { id: 'help', keys: [['?']], label: t('help.items.help') },
  ]
  const navigation: ShortcutRow[] = GO_SHORTCUTS.flatMap((shortcut) => {
    const item = entries.find((candidate) => candidate.to === shortcut.to)
    if (!item) return []
    return [
      {
        id: shortcut.to,
        keys: [['G'], [shortcut.key.toUpperCase()]],
        label: t('help.items.go', { page: item.label }),
      },
    ]
  })

  const sections = [
    { id: 'general', title: t('help.sections.general'), rows: general },
    { id: 'navigation', title: t('help.sections.navigation'), rows: navigation },
  ]

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('help.title')}</DialogTitle>
          <DialogDescription>{t('help.description')}</DialogDescription>
        </DialogHeader>
        {!enabled && (
          <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">
            {t('help.disabled')}{' '}
            <Link to="/account/preferences" className="text-primary underline-offset-4 hover:underline" onClick={() => onOpenChange(false)}>
              {t('help.manage')}
            </Link>
          </p>
        )}
        {sections.map((section) => (
          <section key={section.id} className="grid gap-2">
            <h3 className="text-sm font-medium">{section.title}</h3>
            <dl className="grid gap-1.5">
              {section.rows.map((row) => (
                <div key={row.id} className="flex items-center justify-between gap-4 text-sm">
                  <dt className="min-w-0 text-muted-foreground">{row.label}</dt>
                  <dd>
                    <Keys sequence={row.keys} separator={t('help.then')} />
                  </dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </DialogContent>
    </Dialog>
  )
}
