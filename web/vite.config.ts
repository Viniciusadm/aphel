import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig, type Plugin } from 'vite'
import { serviceWorkerPlugin } from './scripts/pwa/vite-plugin.ts'

const backendUrl = 'http://127.0.0.1:3001'

const fromHere = (path: string) => fileURLToPath(new URL(path, import.meta.url))

const externalModules: Record<string, string> = JSON.parse(
  readFileSync(fromHere('./modules.aliases.json'), 'utf8'),
)

const moduleAliases = Object.entries(externalModules).map(([key, directory]) => ({
  find: new RegExp(`^@modules/${key}(?=/|$)`),
  replacement: fromHere(directory),
}))

const sharedEntries: Record<string, { specifier: string; source: string }> = {
  react: { specifier: 'react', source: 'runtime/react.js' },
  'jsx-runtime': { specifier: 'react/jsx-runtime', source: 'runtime/jsx-runtime.js' },
  'react-dom': { specifier: 'react-dom', source: 'runtime/react-dom.js' },
  'react-router': { specifier: 'react-router', source: 'runtime/react-router.js' },
  'react-query': { specifier: '@tanstack/react-query', source: 'runtime/react-query.js' },
  'react-i18next': { specifier: 'react-i18next', source: 'runtime/react-i18next.js' },
  sdk: { specifier: '@aphel/web-sdk', source: 'src/sdk/index.ts' },
}

function pluginImportMap(): Plugin {
  let serving = false
  return {
    name: 'aphel-plugin-import-map',
    configResolved(config) {
      serving = config.command === 'serve'
    },
    transformIndexHtml() {
      const imports = Object.fromEntries(
        Object.entries(sharedEntries).map(([name, entry]) => [
          entry.specifier,
          serving ? `/${entry.source}` : `/shared/${name}.js`,
        ]),
      )
      return [
        {
          tag: 'script',
          attrs: { type: 'importmap' },
          children: JSON.stringify({ imports }),
          injectTo: 'head-prepend',
        },
      ]
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [pluginImportMap(), react(), tailwindcss(), serviceWorkerPlugin()],
  resolve: {
    alias: [
      ...moduleAliases,
      { find: /^@modules(?=\/)/, replacement: fromHere('./src/modules') },
      { find: /^@(?=\/)/, replacement: fromHere('./src') },
    ],
  },
  build: {
    rolldownOptions: {
      input: {
        index: fromHere('./index.html'),
        ...Object.fromEntries(
          Object.entries(sharedEntries).map(([name, entry]) => [`shared/${name}`, fromHere(`./${entry.source}`)]),
        ),
      },
      preserveEntrySignatures: 'exports-only',
      output: {
        entryFileNames: (chunk) => (chunk.name.startsWith('shared/') ? '[name].js' : 'assets/[name]-[hash].js'),
        codeSplitting: {
          groups: [
            { name: 'ui-utils', test: /node_modules[\\/](clsx|tailwind-merge|class-variance-authority)[\\/]/, priority: 1 },
            { name: 'react', test: /node_modules[\\/](react|react-dom|scheduler|react-router)[\\/]/ },
            { name: 'radix', test: /node_modules[\\/](radix-ui|@radix-ui)[\\/]/ },
            { name: 'i18n', test: /node_modules[\\/](i18next|react-i18next)[\\/]/ },
            { name: 'forms', test: /node_modules[\\/](zod|react-hook-form|@hookform)[\\/]/ },
            { name: 'query', test: /node_modules[\\/]@tanstack[\\/](query-core|react-query)[\\/]/ },
            { name: 'table', test: /node_modules[\\/]@tanstack[\\/](table-core|react-table|store|react-store)[\\/]/ },
            { name: 'dnd', test: /node_modules[\\/]@dnd-kit[\\/]/ },
            { name: 'icons', test: /node_modules[\\/]lucide-react[\\/]/ },
            { name: 'phone', test: /node_modules[\\/]libphonenumber-js[\\/]/ },
            { name: 'editor', test: /node_modules[\\/](@tiptap|prosemirror-[^\\/]+|orderedmap|rope-sequence|w3c-keyname|linkifyjs)[\\/]/ },
            { name: 'charts', test: /node_modules[\\/](recharts|d3-[^\\/]+|victory-vendor|@reduxjs|redux|redux-thunk|reselect|immer|decimal\.js-light|es-toolkit|eventemitter3|internmap|react-redux|tiny-invariant)[\\/]/ },
            {
              name: (id: string) => {
                const locale = /[\\/]i18n[\\/](pt-BR|en)(?:[\\/]|\.json$)/.exec(id)?.[1]
                return locale ? `locales-${locale}` : null
              },
              test: /[\\/]i18n[\\/].+\.json$/,
            },
          ],
        },
      },
    },
  },
  server: {
    fs: {
      allow: ['.', ...Object.values(externalModules).map(fromHere)],
    },
    proxy: {
      '/api': backendUrl,
      '/health': backendUrl,
    },
  },
})
