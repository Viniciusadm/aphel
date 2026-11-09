import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { FieldError } from '@/components/ui/field-error'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import type { PackPluginStatus, PluginPackResult } from './types'

const STATUS_TONE: Record<PackPluginStatus, 'success' | 'neutral' | 'destructive' | 'warning'> = {
  installed: 'success',
  upgraded: 'success',
  unchanged: 'neutral',
  failed: 'destructive',
  skipped: 'warning',
}

export function PluginPackResultDialog({ result, onClose }: { result: PluginPackResult; onClose: () => void }) {
  const { t } = useTranslation('modules')
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('plugins.packResultTitle', { name: result.name })}</DialogTitle>
          <DialogDescription>{t('plugins.packResultDescription')}</DialogDescription>
        </DialogHeader>
        <ol className="grid gap-2 text-sm">
          {result.plugins.map((plugin) => (
            <li key={plugin.key} className="grid gap-1 rounded-md border px-3 py-2">
              <div className="flex items-center gap-2">
                <span className="flex-1 font-medium">{plugin.key}</span>
                <span className="text-xs text-muted-foreground">{t('version', { version: plugin.version })}</span>
                <Badge tone={STATUS_TONE[plugin.status]}>{t(`plugins.packStatus.${plugin.status}`)}</Badge>
              </div>
              {plugin.error && <FieldError>{plugin.error.message}</FieldError>}
            </li>
          ))}
        </ol>
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('plugins.packResultClose')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
