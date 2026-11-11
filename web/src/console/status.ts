export function isTransitional(status: string): boolean {
  return status === 'provisioning' || status === 'deleting'
}
