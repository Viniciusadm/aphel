import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'

export interface QuickFormProps {
  onDone: () => void
}

export function FormActions({ pending, disabled, label }: { pending: boolean; disabled?: boolean; label: string }) {
  const { t } = useTranslation('mobile')
  return (
    <div className="sticky bottom-0 -mx-4 mt-2 border-t bg-background px-4 pt-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
      <Button type="submit" size="lg" className="h-12 w-full text-base" disabled={pending || disabled}>
        {pending ? t('quickAdd.saving') : label}
      </Button>
    </div>
  )
}

export function FormBody({ children, onSubmit }: { children: ReactNode; onSubmit: () => void }) {
  return (
    <form
      className="grid gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        onSubmit()
      }}
    >
      {children}
    </form>
  )
}
