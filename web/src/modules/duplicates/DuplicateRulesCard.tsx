import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { useDuplicateRules, useUpdateDuplicateRules, type DuplicateRecordType, type DuplicateRules } from './api'
import { ruleLabelKey } from './lib/labels'

export function DuplicateRulesCard({ recordType, label }: { recordType: DuplicateRecordType; label: string }) {
  const { t } = useTranslation('duplicates')
  const rules = useDuplicateRules(recordType)

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          {t('settings.title')} · {label}
        </CardTitle>
        <CardDescription>{t('settings.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {rules.data ? (
          <RulesForm key={JSON.stringify(rules.data.rules)} recordType={recordType} current={rules.data} />
        ) : (
          <Skeleton className="h-20 w-full" />
        )}
      </CardContent>
    </Card>
  )
}

function RulesForm({ recordType, current }: { recordType: DuplicateRecordType; current: DuplicateRules }) {
  const { t } = useTranslation('duplicates')
  const save = useUpdateDuplicateRules(recordType)
  const [values, setValues] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(current.rules.map((rule) => [rule.rule, rule.enabled])),
  )
  const changed = current.rules.some((rule) => values[rule.rule] !== rule.enabled)

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        save.mutate(values, {
          onSuccess: () => notifySuccess(t('settings.saved')),
          onError: (error) => notifyError(error),
        })
      }}
    >
      <ul className="grid gap-3">
        {current.rules.map((rule) => {
          const id = `duplicate-rule-${recordType}-${rule.rule}`
          const labelKey = ruleLabelKey(rule.rule)
          return (
            <li key={rule.rule} className="flex items-center gap-2">
              <Checkbox
                id={id}
                checked={values[rule.rule] ?? false}
                onCheckedChange={(checked) => setValues((previous) => ({ ...previous, [rule.rule]: checked === true }))}
              />
              <Label htmlFor={id}>{labelKey ? t(labelKey) : rule.rule}</Label>
            </li>
          )
        })}
      </ul>
      <div>
        <Button type="submit" size="sm" disabled={!changed || save.isPending}>
          {t('settings.save')}
        </Button>
      </div>
    </form>
  )
}
