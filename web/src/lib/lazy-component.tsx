import { Suspense, lazy, type ComponentType, type ReactNode } from 'react'

export function lazyComponent<P extends object>(
  load: () => Promise<ComponentType<P>>,
  fallback: ReactNode = null,
): ComponentType<P> {
  const Loaded = lazy(async () => ({ default: await load() }))
  function LazyComponent(props: P) {
    return (
      <Suspense fallback={fallback}>
        <Loaded {...props} />
      </Suspense>
    )
  }
  return LazyComponent
}
