import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { type DraftGroup, type FieldDefinition, type FieldSchema, FilterBuilder, type LayoutContext, type LayoutField, serializeFilter, toDraft } from '@/sdk'
import type { FieldRules } from '@/modules/record_layouts/ui/lib/draft'

export interface FieldRulesDialogProps {
  field: FieldDefinition | null
  rule: LayoutField | null
  schema: FieldSchema
  placedKeys: string[]
  context: LayoutContext
  onApply: (rules: FieldRules) => void
  onClose: () => void
}

export function FieldRulesDialog({ field, rule, schema, placedKeys, context, onApply, onClose }: FieldRulesDialogProps) {
  return (
    <Dialog open={Boolean(field && rule)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        {field && rule && (
          <RulesBody
            key={field.key}
            field={field}
            rule={rule}
            schema={schema}
            placedKeys={placedKeys}
            context={context}
            onApply={onApply}
            onClose={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function RulesBody({
  field,
  rule,
  schema,
  placedKeys,
  context,
  onApply,
  onClose,
}: Omit<FieldRulesDialogProps, 'field' | 'rule'> & { field: FieldDefinition; rule: LayoutField }) {
  const { t } = useTranslation('layouts')
  const conditionSchema = useMemo<FieldSchema>(
    () => ({ ...schema, fields: schema.fields.filter((item) => placedKeys.includes(item.key)) }),
    [schema, placedKeys],
  )
  const [visible, setVisible] = useState<DraftGroup>(() => toDraft(rule.visible_when))
  const [required, setRequired] = useState<DraftGroup>(() => toDraft(rule.required_when))
  const canHide = !field.is_required || context === 'view'
  const canRequire = context !== 'view' && !field.is_required

  const apply = () => {
    onApply({
      visible_when: canHide ? serializeFilter(visible, conditionSchema) : null,
      required_when: canRequire ? serializeFilter(required, conditionSchema) : null,
    })
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t('rules.title', { field: field.label })}</DialogTitle>
        <DialogDescription>{t('rules.description')}</DialogDescription>
      </DialogHeader>
      <div className="grid min-w-0 gap-5">
        <section className="grid min-w-0 gap-2" aria-labelledby="rules-visible">
          <h3 id="rules-visible" className="text-sm font-semibold">
            {t('rules.visibleWhen')}
          </h3>
          {canHide ? (
            <FilterBuilder schema={conditionSchema} value={visible} onChange={setVisible} />
          ) : (
            <p className="text-sm text-muted-foreground">{t('rules.alwaysVisible')}</p>
          )}
        </section>
        <section className="grid min-w-0 gap-2" aria-labelledby="rules-required">
          <h3 id="rules-required" className="text-sm font-semibold">
            {t('rules.requiredWhen')}
          </h3>
          {canRequire ? (
            <>
              <FilterBuilder schema={conditionSchema} value={required} onChange={setRequired} />
              <p className="text-xs text-muted-foreground">{t('rules.hiddenNotRequired')}</p>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              {context === 'view' ? t('rules.noRequiredInView') : t('rules.alreadyRequired')}
            </p>
          )}
        </section>
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose}>
          {t('rules.cancel')}
        </Button>
        <Button type="button" onClick={apply}>
          {t('rules.apply')}
        </Button>
      </DialogFooter>
    </>
  )
}
