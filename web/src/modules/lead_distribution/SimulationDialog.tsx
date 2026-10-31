import { useEffect, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useSimulation, type DistributionRule } from './api'

const DEFAULT_COUNT = 10
const MAX_COUNT = 50

interface SimulationDialogProps {
  rule: DistributionRule
  onClose: () => void
}

export function SimulationDialog({ rule, onClose }: SimulationDialogProps) {
  const { t } = useTranslation('lead_distribution')
  const simulation = useSimulation()
  const [count, setCount] = useState(DEFAULT_COUNT)
  const [value, setValue] = useState('')
  const routingField = rule.routing?.field
  const { mutate } = simulation

  useEffect(() => {
    mutate(
      { ruleId: rule.id, request: { count: DEFAULT_COUNT, values: {} } },
      { onError: (error) => notifyError(error) },
    )
  }, [mutate, rule.id])

  const run = (event?: FormEvent) => {
    event?.preventDefault()
    const values = routingField && value.trim() ? { [routingField]: value.trim() } : {}
    mutate(
      { ruleId: rule.id, request: { count: Math.min(Math.max(count || 1, 1), MAX_COUNT), values } },
      { onError: (error) => notifyError(error) },
    )
  }

  const result = simulation.data

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{t('simulation.title', { name: rule.name })}</DialogTitle>
          <DialogDescription>{t('simulation.description')}</DialogDescription>
        </DialogHeader>
        {!rule.is_active && <p className="text-sm text-muted-foreground">{t('simulation.inactiveRule')}</p>}
        <form className="flex flex-wrap items-end gap-3" onSubmit={run}>
          <div className="grid w-28 gap-2">
            <Label htmlFor="simulation-count">{t('simulation.count')}</Label>
            <Input placeholder={t('placeholders.count')}
              id="simulation-count"
              type="number"
              min={1}
              max={MAX_COUNT}
              value={Number.isFinite(count) ? count : ''}
              onChange={(event) => setCount(event.target.valueAsNumber)}
            />
          </div>
          {routingField && (
            <div className="grid min-w-40 flex-1 gap-2">
              <Label htmlFor="simulation-value">{t('simulation.value', { field: routingField })}</Label>
              <Input
                id="simulation-value"
                placeholder={t('simulation.valuePlaceholder')}
                value={value}
                onChange={(event) => setValue(event.target.value)}
              />
            </div>
          )}
          <Button type="submit" disabled={simulation.isPending}>
            {simulation.isPending ? t('simulation.running') : t('simulation.run')}
          </Button>
        </form>
        {result && (
          <div className="grid gap-4" aria-live="polite">
            <p className="text-sm text-muted-foreground">
              {result.selection === 'route'
                ? t('simulation.selection.route', { index: (result.route_index ?? 0) + 1 })
                : t(`simulation.selection.${result.selection}`)}
            </p>
            {result.selection === 'unassigned' ? (
              <p className="text-sm">{t('simulation.unassigned')}</p>
            ) : result.recipients.length === 0 ? (
              <p className="text-sm">{t('simulation.nobody')}</p>
            ) : (
              <div className="grid gap-2">
                <h3 className="text-sm font-medium">{t('simulation.next', { count: result.recipients.length })}</h3>
                <ol className="grid gap-1 rounded-lg border p-2">
                  {result.recipients.map((recipient) => (
                    <li key={recipient.position} className="flex items-center gap-3 rounded-md px-2 py-1 text-sm">
                      <span className="w-6 text-right text-muted-foreground tabular-nums">{recipient.position}.</span>
                      <span>{recipient.name}</span>
                    </li>
                  ))}
                </ol>
              </div>
            )}
            {result.skipped.length > 0 && (
              <div className="grid gap-2">
                <h3 className="text-sm font-medium">{t('simulation.skipped')}</h3>
                <ul className="flex flex-wrap gap-2">
                  {result.skipped.map((skipped) => (
                    <li key={skipped.user_id}>
                      <Badge variant="outline">
                        {skipped.name ?? skipped.user_id} · {t(`simulation.reasons.${skipped.reason}`)}
                      </Badge>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t('actions.close')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
