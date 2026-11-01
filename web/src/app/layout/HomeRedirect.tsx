import { useEffect } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { mobileHomePath } from '@/features/mobile/lib/home'
import { usePersonalPreferences } from '@/features/settings/navigation/api'
import { pluginRoleHome } from '@/kernel/modules/plugins'

const HOME_REDIRECT_KEY = 'aphel.home.redirected'

function alreadyRedirected(): boolean {
  try {
    return window.sessionStorage.getItem(HOME_REDIRECT_KEY) === '1'
  } catch {
    return true
  }
}

function markRedirected(): void {
  try {
    window.sessionStorage.setItem(HOME_REDIRECT_KEY, '1')
  } catch {
    return
  }
}

export function HomeRedirect() {
  const location = useLocation()
  const navigate = useNavigate()
  const preferences = usePersonalPreferences()
  const { me } = useSession()
  const home = preferences.data?.navigation.home ?? pluginRoleHome(me.role_key) ?? mobileHomePath()

  useEffect(
    () => () => {
      try {
        window.sessionStorage.removeItem(HOME_REDIRECT_KEY)
      } catch {
        return
      }
    },
    [],
  )

  useEffect(() => {
    if (alreadyRedirected()) return
    if (location.pathname !== '/') {
      markRedirected()
      return
    }
    if (preferences.isPending) return
    markRedirected()
    if (home && home !== '/') void navigate(home, { replace: true })
  }, [home, location.pathname, navigate, preferences.isPending])

  return null
}
