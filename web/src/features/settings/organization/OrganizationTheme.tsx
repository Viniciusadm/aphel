import { useEffect } from 'react'
import { useSession } from '@/app/session/session-context'
import { applyOrganizationColor } from './lib/apply-color'

export function OrganizationTheme() {
  const { me } = useSession()
  const color = me.organization?.primary_color ?? null

  useEffect(() => {
    applyOrganizationColor(color)
    return () => applyOrganizationColor(null)
  }, [color])

  return null
}
