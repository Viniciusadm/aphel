import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { basename, dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const LOCALE = 'pt-BR'
const DEFAULT_NAMESPACE = 'common'
const webRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const root = join(webRoot, 'src')
const aliasesFile = join(webRoot, 'modules.aliases.json')
const output = join(root, 'i18n', 'i18next.d.ts')

function legacyNamespaces() {
  const directory = join(root, 'i18n', LOCALE)
  return readdirSync(directory)
    .filter((file) => file.endsWith('.json'))
    .map((file) => ({ namespace: basename(file, '.json'), path: join(directory, file) }))
}

function moduleNamespaces() {
  const directory = join(root, 'modules')
  if (!existsSync(directory)) return []
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => ({ namespace: entry.name, path: join(directory, entry.name, 'i18n', `${LOCALE}.json`) }))
    .filter((entry) => existsSync(entry.path))
}

function externalModuleNamespaces() {
  if (!existsSync(aliasesFile)) return []
  const aliases = JSON.parse(readFileSync(aliasesFile, 'utf8'))
  return Object.entries(aliases)
    .map(([namespace, directory]) => ({ namespace, path: join(webRoot, directory, 'i18n', `${LOCALE}.json`) }))
    .filter((entry) => existsSync(entry.path))
}

function identifier(namespace) {
  const camel = namespace.replace(/[-_]+([a-z0-9])/g, (_, letter) => letter.toUpperCase())
  return `${camel}Namespace`
}

function quote(namespace) {
  return /^[a-zA-Z_$][\w$]*$/.test(namespace) ? namespace : `'${namespace}'`
}

const namespaces = [...legacyNamespaces(), ...moduleNamespaces(), ...externalModuleNamespaces()].sort((left, right) =>
  left.namespace.localeCompare(right.namespace),
)
const duplicates = namespaces.filter((entry, index) => namespaces.findIndex((other) => other.namespace === entry.namespace) !== index)
if (duplicates.length > 0) {
  throw new Error(`duplicated i18n namespaces: ${duplicates.map((entry) => entry.namespace).join(', ')}`)
}

const imports = namespaces.map(({ namespace, path }) => {
  const specifier = relative(dirname(output), path).split('\\').join('/')
  return `import type ${identifier(namespace)} from '${specifier.startsWith('.') ? specifier : `./${specifier}`}'`
})
const resources = namespaces.map(({ namespace }) => `      ${quote(namespace)}: typeof ${identifier(namespace)}`)

writeFileSync(
  output,
  `import 'i18next'
${imports.join('\n')}

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: '${DEFAULT_NAMESPACE}'
    resources: {
${resources.join('\n')}
    }
  }
}
`,
)
console.log(`i18next.d.ts: ${namespaces.length} namespaces`)
