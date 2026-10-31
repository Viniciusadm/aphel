import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import {
  TWO_FACTOR_NOT_ENABLED,
  TWO_FACTOR_POLICIES,
  TWO_FACTOR_SETUP_REQUIRED,
  hasErrorCode,
  twoFactorApi,
  twoFactorKeys,
  useTwoFactorPolicy,
  type TwoFactorPolicy,
  type TwoFactorPolicyState,
} from './api'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function OrganizationPolicy() {
  const { t } = useTranslation('two-factor')
  const policy = useTwoFactorPolicy(true)

  return (
    <Page>
      <PageHeader title={t('policy.title')} description={t('policy.description')} />
      {policy.isPending ? (
        <Skeleton className="h-40 w-full" />
      ) : policy.isError ? (
        hasErrorCode(policy.error, TWO_FACTOR_SETUP_REQUIRED) ? (
          <p className="rounded-md border p-3 text-sm text-muted-foreground">{t('policy.setupFirst')}</p>
        ) : (
          <ErrorState onRetry={() => void policy.refetch()} />
        )
      ) : (
        <PolicyForm key={policy.data.policy} state={policy.data} />
      )}
    </Page>
  )
}

function PolicyForm({ state }: { state: TwoFactorPolicyState }) {
  const { t } = useTranslation('two-factor')
  const queryClient = useQueryClient()
  const name = useId()
  const [selected, setSelected] = useState<TwoFactorPolicy>(state.policy)
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setPending(true)
    setError(null)
    try {
      const saved = await twoFactorApi.updatePolicy(selected)
      queryClient.setQueryData(twoFactorKeys.policy(), saved)
      notifySuccess(t('policy.saved'))
    } catch (cause) {
      if (hasErrorCode(cause, TWO_FACTOR_NOT_ENABLED)) setError(t('policy.enableYourselfFirst'))
      else notifyError(cause)
    } finally {
      setPending(false)
    }
  }

  return (
    <Card>
      <CardContent className="pt-6">
        <form noValidate onSubmit={(event) => void submit(event)} className="grid gap-4">
          <fieldset className="grid gap-2">
            <legend className="mb-2 text-sm font-medium">{t('policy.legend')}</legend>
            {TWO_FACTOR_POLICIES.map((option) => {
              const id = `${name}-${option}`
              return (
                <label
                  key={option}
                  htmlFor={id}
                  className="flex cursor-pointer items-start gap-3 rounded-md border p-3 has-[:checked]:border-primary has-[:checked]:bg-primary/5"
                >
                  <input
                    id={id}
                    type="radio"
                    name={name}
                    value={option}
                    checked={selected === option}
                    onChange={() => setSelected(option)}
                    className="mt-1 size-4 accent-primary"
                  />
                  <span className="grid gap-0.5">
                    <span className="text-sm font-medium">{t(`policy.options.${option}.label`)}</span>
                    <span className="text-xs text-muted-foreground">{t(`policy.options.${option}.description`)}</span>
                  </span>
                </label>
              )
            })}
          </fieldset>
          {state.policy !== 'none' && (
            <p className="text-sm text-muted-foreground" aria-live="polite">
              {t('policy.membersWithout', { count: state.members_without_two_factor })}
            </p>
          )}
          {error && (
            <SharedFieldError>
              {error}
            </SharedFieldError>
          )}
          <div className="flex justify-end">
            <Button type="submit" disabled={pending || selected === state.policy} aria-busy={pending || undefined}>
              {pending ? t('policy.saving') : t('policy.save')}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
