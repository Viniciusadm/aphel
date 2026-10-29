import { KeyRound } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import type { SsoOption } from '../api'

export interface SsoOptionsProps {
  options: SsoOption[]
  redirect: string | null
}

function startUrl(option: SsoOption, redirect: string | null): string {
  if (!redirect) return option.url
  const separator = option.url.includes('?') ? '&' : '?'
  return `${option.url}${separator}redirect=${encodeURIComponent(redirect)}`
}

export function SsoOptions({ options, redirect }: SsoOptionsProps) {
  const { t } = useTranslation('auth')
  if (options.length === 0) return null
  return (
    <div className="grid gap-2">
      {options.map((option) => (
        <Button key={`${option.provider}:${option.url}`} asChild variant="outline" className="w-full">
          <a href={startUrl(option, redirect)}>
            <KeyRound className="size-4" aria-hidden />
            {t('login.ssoContinue', { label: option.label })}
          </a>
        </Button>
      ))}
    </div>
  )
}
