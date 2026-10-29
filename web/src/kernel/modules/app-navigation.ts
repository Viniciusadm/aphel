export type AppNavigator = (to: string) => void

let navigator: AppNavigator | null = null

export function setAppNavigator(next: AppNavigator | null) {
  navigator = next
}

export function navigateApp(to: string) {
  if (navigator) navigator(to)
  else window.location.assign(to)
}
