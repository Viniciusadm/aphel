import { PageTitle } from '@/components/ui/page-title'
import { ArrowLeft, LoaderCircle } from 'lucide-react'
import { useEffect, useRef, type FormEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from '@/components/ui/card'
import { useSetupFrame } from './context'
import { Alert } from '@/components/ui/alert'

export interface SetupAction {
  label: string
  onClick?: () => void
  disabled?: boolean
  busy?: boolean
  busyLabel?: string
}

interface SetupStepFrameProps {
  title: string
  description?: ReactNode
  error?: string | null
  notice?: string | null
  primary?: SetupAction
  secondary?: SetupAction
  aside?: ReactNode
  children?: ReactNode
}

export function SetupStepFrame({ title, description, error, notice, primary, secondary, aside, children }: SetupStepFrameProps) {
  const { t } = useTranslation('setup')
  const { onBack } = useSetupFrame()
  const headingRef = useRef<HTMLHeadingElement>(null)
  const busy = primary?.busy === true || secondary?.busy === true

  useEffect(() => {
    headingRef.current?.focus({ preventScroll: true })
  }, [])

  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (event.target !== event.currentTarget || busy || primary?.disabled) return
    primary?.onClick?.()
  }

  return (
    <Card>
      <form className="grid gap-6" onSubmit={onSubmit} noValidate>
        <CardHeader className="gap-2">
          <PageTitle ref={headingRef} tabIndex={-1} className="outline-none">
            {title}
          </PageTitle>
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
        <CardContent className="grid gap-5">
          {notice && (
            <p role="status" className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
              {notice}
            </p>
          )}
          {error && (
            <Alert className="text-destructive">
              {error}
            </Alert>
          )}
          {children}
        </CardContent>
        {(primary || secondary || onBack || aside) && (
          <CardFooter className="flex flex-col-reverse items-stretch gap-2 border-t pt-6 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:items-center">
              {onBack && (
                <Button type="button" variant="ghost" onClick={onBack} disabled={busy}>
                  <ArrowLeft aria-hidden />
                  {t('actions.back')}
                </Button>
              )}
              {aside}
            </div>
            <div className="flex flex-col-reverse gap-2 sm:flex-row">
              {secondary && (
                <Button type="button" variant="outline" onClick={secondary.onClick} disabled={busy || secondary.disabled}>
                  {secondary.busy && <LoaderCircle className="animate-spin" aria-hidden />}
                  {secondary.busy && secondary.busyLabel ? secondary.busyLabel : secondary.label}
                </Button>
              )}
              {primary && (
                <Button type="submit" disabled={busy || primary.disabled}>
                  {primary.busy && <LoaderCircle className="animate-spin" aria-hidden />}
                  {primary.busy && primary.busyLabel ? primary.busyLabel : primary.label}
                </Button>
              )}
            </div>
          </CardFooter>
        )}
      </form>
    </Card>
  )
}
