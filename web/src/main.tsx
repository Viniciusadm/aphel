import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import '@/i18n'
import { App } from '@/app/App'
import { APP_THEME, applyResolvedTheme } from '@/lib/theme'
import { registerPwa } from '@/pwa/register'

applyResolvedTheme(APP_THEME)
registerPwa()

const rootElement = document.getElementById('root')
if (!rootElement) throw new Error('root element not found')

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
