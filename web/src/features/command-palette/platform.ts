export function isApplePlatform(): boolean {
  return typeof navigator !== 'undefined' && /mac|iphone|ipad/i.test(navigator.platform || navigator.userAgent)
}

export function modifierKeyLabel(): string {
  return isApplePlatform() ? '⌘' : 'Ctrl'
}
