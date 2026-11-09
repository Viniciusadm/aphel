import { useEffect } from 'react'
import { useModules, usePublicModules } from './api'
import { preloadModuleTranslations } from './i18n'
import { serverModuleState, setServerModules, type ServerModule } from './registry'

function useServerModulesSync(data: ServerModule[] | undefined, isError: boolean) {
  useEffect(() => {
    if (!data && !isError) return
    const list = data ?? null
    let current = true
    void preloadModuleTranslations(serverModuleState(list))
      .catch(() => undefined)
      .then(() => {
        if (current) setServerModules(list)
      })
    return () => {
      current = false
    }
  }, [data, isError])
}

export function ModulesSync() {
  const { data, isError } = useModules()
  useServerModulesSync(data, isError)
  return null
}

export function PublicModulesSync() {
  const { data, isError } = usePublicModules()
  useServerModulesSync(data, isError)
  return null
}
