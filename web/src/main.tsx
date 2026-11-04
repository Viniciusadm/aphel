import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import '@/i18n'
import { loadWebPlugins } from '@/kernel/modules/wasm_runtime'
import { APP_THEME, applyResolvedTheme } from '@/lib/theme'
import { registerPwa } from '@/pwa/register'

applyResolvedTheme(APP_THEME)
registerPwa()

await loadWebPlugins()
const { App } = await import('@/app/App')

const rootElement = document.getElementById('root')
if (!rootElement) throw new Error('root element not found')

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
