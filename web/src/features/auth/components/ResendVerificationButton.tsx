import { useMutation } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Button, type ButtonProps } from '@/components/ui/button'
import { toast } from '@/components/ui/toast-store'
import { resendVerification } from '../api'
import { RATE_LIMITED_CODE, generalErrorMessage } from '../lib/api-errors'

export function ResendVerificationButton({ variant = 'outline', size = 'sm', ...props }: Omit<ButtonProps, 'onClick'>) {
  const { t } = useTranslation('auth')
  const mutation = useMutation({
    mutationFn: resendVerification,
    onSuccess: () => toast({ title: t('emailVerification.sent'), variant: 'success' }),
    onError: (error) => {
      const rateLimited = isApiError(error) && error.code === RATE_LIMITED_CODE
      toast({
        title: rateLimited ? t('emailVerification.rateLimited') : t(generalErrorMessage(error)),
        variant: 'destructive',
      })
    },
  })

  return (
    <Button
      variant={variant}
      size={size}
      disabled={mutation.isPending}
      aria-busy={mutation.isPending || undefined}
      onClick={() => mutation.mutate()}
      {...props}
    >
      {mutation.isPending && <LoaderCircle className="animate-spin" aria-hidden />}
      {mutation.isPending ? t('emailVerification.resending') : t('emailVerification.resend')}
    </Button>
  )
}
