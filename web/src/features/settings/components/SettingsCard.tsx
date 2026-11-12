import type { ReactNode } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

interface SettingsCardProps {
  title: string
  description?: string
  disabled?: boolean
  children: ReactNode
}

export const WIDE_FIELD = 'md:col-span-2'

export function SettingsCard({ title, description, disabled, children }: SettingsCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      <CardContent>
        <fieldset disabled={disabled} className="grid min-w-0 gap-4 md:grid-cols-2">
          {children}
        </fieldset>
      </CardContent>
    </Card>
  )
}
