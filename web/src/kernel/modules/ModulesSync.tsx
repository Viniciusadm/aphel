import { useEffect } from 'react'
import { useModules } from './api'
import { preloadModuleTranslations } from './i18n'
import { serverModuleState, setServerModules } from './registry'

export function ModulesSync() {
  const { data, isError } = useModules()
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
  return null
}
