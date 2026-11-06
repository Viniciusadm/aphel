import { useEffect } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { mobileHomePath } from '@/features/mobile/lib/home'
import { useNavigationPreferences } from '@/features/settings/navigation/api'
import { useInstalledBlueprint } from '@/kernel/modules/blueprint'
import { useUiPlugin } from '@/kernel/modules/ui-plugin-context'
import { pluginRoleHome } from '@/kernel/modules/wasm_runtime'

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
  const { ready } = useUiPlugin()
  const navigation = useNavigationPreferences()
  const { me } = useSession()
  const blueprint = useInstalledBlueprint()
  const blueprintHome = blueprint.data?.home && blueprint.data.home !== '/' ? blueprint.data.home : null
  const home = navigation.home ?? pluginRoleHome(me.role_key) ?? blueprintHome ?? mobileHomePath()

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
    if (!ready || blueprint.isPending) return
    markRedirected()
    if (home && home !== '/') void navigate(home, { replace: true })
  }, [home, location.pathname, navigate, ready, blueprint.isPending])

  return null
}
