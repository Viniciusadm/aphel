type SignOutCleanup = () => void | Promise<void>

const cleanups = new Set<SignOutCleanup>()

export function registerSignOutCleanup(cleanup: SignOutCleanup): () => void {
  cleanups.add(cleanup)
  return () => {
    cleanups.delete(cleanup)
  }
}

export async function runSignOutCleanups(): Promise<void> {
  await Promise.allSettled([...cleanups].map((cleanup) => Promise.resolve().then(cleanup)))
}
