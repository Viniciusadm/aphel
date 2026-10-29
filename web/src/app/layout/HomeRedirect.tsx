import { useEffect } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { mobileHomePath } from '@/features/mobile/lib/home'
import { usePersonalPreferences } from '@/features/settings/navigation/api'

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
  const home = preferences.data?.navigation.home ?? mobileHomePath()

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
