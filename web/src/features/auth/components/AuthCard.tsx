import { PageTitle } from '@/components/ui/page-title'
import type { ReactNode } from 'react'
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from '@/components/ui/card'

interface AuthCardProps {
  title: string
  description?: ReactNode
  footer?: ReactNode
  children?: ReactNode
}

export function AuthCard({ title, description, footer, children }: AuthCardProps) {
  return (
    <Card>
      <CardHeader>
        <PageTitle >{title}</PageTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      {children && <CardContent>{children}</CardContent>}
      {footer && <CardFooter className="justify-center text-sm text-muted-foreground">{footer}</CardFooter>}
    </Card>
  )
}
