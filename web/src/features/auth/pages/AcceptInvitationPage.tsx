import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { storeSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { acceptInvitation, fetchInvitation, isTwoFactorRequired, type InvitationResponse, type LoginResponse } from '../api'
import { TWO_FACTOR_CHALLENGE_PATH } from '../two-factor/paths'
import { AuthCard } from '../components/AuthCard'
import { FormAlert } from '../components/FormAlert'
import { FormField } from '../components/FormField'
import { InlineLoader } from '../components/InlineLoader'
import { InvalidTokenNotice } from '../components/InvalidTokenNotice'
import { SubmitButton } from '../components/SubmitButton'
import { applyApiError, generalErrorMessage, isInvalidTokenError } from '../lib/api-errors'
import {
  existingAccountInvitationSchema,
  newAccountInvitationSchema,
  type ExistingAccountInvitationValues,
  type NewAccountInvitationValues,
} from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'

interface InvitationFormProps {
  token: string
  invitation: InvitationResponse
  onAccepted: (response: LoginResponse) => void
  onInvalidToken: () => void
}

function NewAccountForm({ token, onAccepted, onInvalidToken }: InvitationFormProps) {
  const { t } = useTranslation('auth')
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<NewAccountInvitationValues, unknown, NewAccountInvitationValues>({
    resolver: zodResolver(newAccountInvitationSchema),
    defaultValues: { displayName: '', password: '' },
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      onAccepted(await acceptInvitation({ token, display_name: values.displayName, password: values.password }))
    } catch (error) {
      if (isInvalidTokenError(error)) {
        onInvalidToken()
        return
      }
      applyApiError(error, setError, { fields: { display_name: 'displayName', password: 'password' } })
    }
  })

  return (
    <form noValidate onSubmit={onSubmit} className="grid gap-4">
      <FormAlert message={errors.root?.server?.message} />
      <FormField placeholder={t('placeholders.displayName')}
        id="invitation-display-name"
        autoComplete="name"
        label={t('fields.displayName')}
        error={errors.displayName?.message}
        {...register('displayName')}
      />
      <FormField placeholder={t('placeholders.newPassword')}
        id="invitation-password"
        type="password"
        autoComplete="new-password"
        label={t('fields.password')}
        hint={t('hints.password')}
        error={errors.password?.message}
        {...register('password')}
      />
      <SubmitButton pending={isSubmitting} pendingLabel={t('acceptInvitation.submitting')}>
        {t('acceptInvitation.submit')}
      </SubmitButton>
    </form>
  )
}

function ExistingAccountForm({ token, onAccepted, onInvalidToken }: InvitationFormProps) {
  const { t } = useTranslation('auth')
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ExistingAccountInvitationValues, unknown, ExistingAccountInvitationValues>({
    resolver: zodResolver(existingAccountInvitationSchema),
    defaultValues: { password: '' },
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      onAccepted(await acceptInvitation({ token, password: values.password }))
    } catch (error) {
      if (isInvalidTokenError(error)) {
        onInvalidToken()
        return
      }
      applyApiError(error, setError, {
        fields: { password: 'password' },
        codes: { unauthorized: { field: 'password', message: 'errors.wrongPassword' } },
      })
    }
  })

  return (
    <form noValidate onSubmit={onSubmit} className="grid gap-4">
      <FormAlert message={errors.root?.server?.message} />
      <FormField placeholder={t('placeholders.currentPassword')}
        id="invitation-current-password"
        type="password"
        autoComplete="current-password"
        label={t('fields.currentPassword')}
        error={errors.password?.message}
        {...register('password')}
      />
      <SubmitButton pending={isSubmitting} pendingLabel={t('acceptInvitation.submitting')}>
        {t('acceptInvitation.submit')}
      </SubmitButton>
    </form>
  )
}

export function AcceptInvitationPage() {
  const { t } = useTranslation('auth')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const token = searchParams.get('token')
  const [tokenRejected, setTokenRejected] = useState(false)
  const invitation = useQuery({
    queryKey: ['auth', 'invitation', token],
    queryFn: () => fetchInvitation(token ?? ''),
    enabled: Boolean(token),
    retry: false,
    staleTime: Infinity,
  })

  const goHome = (
    <Button asChild variant="outline" className="w-full">
      <Link to="/">{t('invalidToken.goHome')}</Link>
    </Button>
  )

  if (!token) return <InvalidTokenNotice description={t('invalidToken.missing')} action={goHome} />

  if (tokenRejected || invitation.isError) {
    if (tokenRejected || isInvalidTokenError(invitation.error)) {
      return <InvalidTokenNotice description={t('invalidToken.invitationDescription')} action={goHome} />
    }
    return <ErrorState description={t(generalErrorMessage(invitation.error))} onRetry={() => void invitation.refetch()} />
  }

  if (!invitation.data) {
    return (
      <AuthCard title={t('acceptInvitation.loading')}>
        <InlineLoader label={t('acceptInvitation.loading')} />
      </AuthCard>
    )
  }

  const details = invitation.data
  const formProps: InvitationFormProps = {
    token,
    invitation: details,
    onAccepted: (response) => {
      if (isTwoFactorRequired(response)) {
        void navigate(TWO_FACTOR_CHALLENGE_PATH, { replace: true })
        return
      }
      storeSession(queryClient, response)
      void navigate('/', { replace: true })
    },
    onInvalidToken: () => setTokenRejected(true),
  }
  const interpolation = { organization: details.organization_name, email: details.email }

  return (
    <AuthCard
      title={t('acceptInvitation.title', interpolation)}
      description={
        details.requires_account
          ? t('acceptInvitation.descriptionNewAccount', interpolation)
          : t('acceptInvitation.descriptionExistingAccount', interpolation)
      }
    >
      {details.requires_account ? <NewAccountForm {...formProps} /> : <ExistingAccountForm {...formProps} />}
    </AuthCard>
  )
}
