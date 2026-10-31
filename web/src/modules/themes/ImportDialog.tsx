import { TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { contrastWarnings } from './lib/contrast-report'
import type { ThemePackage, ThemeValidation } from './lib/types'
import { ThemeSwatches } from './ThemeSwatches'

interface ImportDialogProps {
  themePackage: ThemePackage | null
  validation: ThemeValidation | null
  installing: boolean
  onCancel: () => void
  onInstall: () => void
}

export function ImportDialog({ themePackage, validation, installing, onCancel, onInstall }: ImportDialogProps) {
  const { t } = useTranslation('themes')
  const open = themePackage !== null && validation !== null
  const blocked = (validation?.issues.length ?? 0) > 0
  const contrast = themePackage && !blocked ? contrastWarnings(themePackage.tokens, 'light') : []

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !installing && onCancel()}>
      <DialogContent>
        {themePackage && validation && (
          <>
            <DialogHeader>
              <DialogTitle>{t('import.title', { name: themePackage.name || themePackage.id })}</DialogTitle>
              <DialogDescription>
                {t('import.summary', {
                  version: themePackage.version,
                  author: themePackage.author,
                  license: themePackage.license,
                })}
              </DialogDescription>
            </DialogHeader>
            <ThemeSwatches tokens={themePackage.tokens} />
            <p className="text-sm text-muted-foreground">
              {t('import.counts', { tokens: validation.token_count, fonts: validation.fonts.length })}
            </p>
            {validation.installed_version && (
              <p className="flex items-start gap-2 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
                <TriangleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                {t('import.replaces', { version: validation.installed_version })}
              </p>
            )}
            {validation.issues.length > 0 && (
              <div role="alert" className="grid gap-1 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm">
                <p className="font-medium">{t('import.issues')}</p>
                <ul className="grid gap-1">
                  {validation.issues.map((issue) => (
                    <li key={`${issue.path}:${issue.message}`} className="break-words">
                      <code>{issue.path}</code> {issue.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {validation.warnings.length > 0 && (
              <details className="rounded-md border p-3 text-sm">
                <summary className="cursor-pointer">{t('import.warnings', { count: validation.warnings.length })}</summary>
                <ul className="mt-2 grid gap-1">
                  {validation.warnings.map((warning) => (
                    <li key={`${warning.path}:${warning.message}`} className="break-words">
                      <code>{warning.path}</code> {warning.message}
                    </li>
                  ))}
                </ul>
              </details>
            )}
            {contrast.length > 0 && (
              <div className="grid gap-1 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
                <p className="font-medium">{t('contrast.title')}</p>
                <ul className="grid gap-1">
                  {contrast.map((warning) => (
                    <li key={`${warning.foreground}:${warning.background}`}>
                      {t('contrast.item', {
                        foreground: warning.foreground,
                        background: warning.background,
                        ratio: warning.ratio.toFixed(1),
                      })}
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {themePackage.fonts.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {validation.fonts.map((font) => (
                  <Badge key={`${font.family}:${font.weight}:${font.style}`} variant="outline">
                    {font.family} {font.weight} {font.style}
                  </Badge>
                ))}
              </div>
            )}
            <DialogFooter>
              <Button variant="outline" onClick={onCancel} disabled={installing}>
                {t('actions.cancel')}
              </Button>
              <Button onClick={onInstall} disabled={blocked || installing}>
                {installing ? t('import.installing') : validation.installed_version ? t('import.replace') : t('import.install')}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

