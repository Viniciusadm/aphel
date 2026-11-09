import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { GLOBAL_SCOPE, KeybindingKeys, chordText, useKeybindings } from '@/kernel/keybindings'

export interface ShortcutsHelpDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export const KEYBOARD_SETTINGS_PATH = '/account/keyboard'

export function ShortcutsHelpDialog({ open, onOpenChange }: ShortcutsHelpDialogProps) {
  const { t } = useTranslation('keybindings')
  const { actions, scopes, activeScopes, enabled } = useKeybindings()
  const paletteKeys = actions.find((action) => action.id === 'core.palette')?.keys ?? []

  const sections = useMemo(() => {
    const rank = (scope: string) => {
      if (activeScopes.includes(scope)) return 0
      if (scope === GLOBAL_SCOPE) return 1
      return 2
    }
    return scopes
      .map((scope) => ({ scope, rows: actions.filter((action) => action.scope === scope.id && action.keys.length > 0) }))
      .filter((section) => section.rows.length > 0)
      .sort((left, right) => rank(left.scope.id) - rank(right.scope.id))
  }, [actions, scopes, activeScopes])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('help.title')}</DialogTitle>
          <DialogDescription>{t('help.description')}</DialogDescription>
        </DialogHeader>
        {!enabled && (
          <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">
            {t('help.disabled', { keys: paletteKeys.map((chord) => chordText(chord)).join(' / ') })}
          </p>
        )}
        <div className="grid max-h-[60vh] gap-5 overflow-y-auto pr-1">
          {sections.map(({ scope, rows }) => (
            <section key={scope.id} className="grid gap-2">
              <h3 className="text-sm font-medium">{scope.label}</h3>
              <dl className="grid gap-1.5">
                {rows.map((action) => (
                  <div key={action.id} className="flex items-center justify-between gap-4 text-sm">
                    <dt className="min-w-0 text-muted-foreground">{action.label}</dt>
                    <dd>
                      <KeybindingKeys keys={action.keys} size="md" />
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
        <p className="text-sm">
          <Link to={KEYBOARD_SETTINGS_PATH} className="text-primary underline-offset-4 hover:underline" onClick={() => onOpenChange(false)}>
            {t('help.customize')}
          </Link>
        </p>
      </DialogContent>
    </Dialog>
  )
}
