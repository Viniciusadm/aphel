import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const webDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const manifest = readFileSync(resolve(webDir, '../back/aphel.modules.toml'), 'utf8')
const locales = ['pt-BR', 'en']

const keys = [...manifest.matchAll(/^(\w+)\s*=\s*\{[^}]*path\s*=\s*"modules\/[^"]+"[^}]*\}/gm)]
  .filter((match) => !/required\s*=\s*true/.test(match[0]))
  .map((match) => match[1])

function read(path) {
  return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : {}
}

const missing = []
for (const locale of locales) {
  const catalog = read(resolve(webDir, `src/i18n/${locale}/modules.json`)).catalog ?? {}
  for (const key of keys) {
    const own = read(resolve(webDir, `src/modules/${key}/i18n/${locale}.json`))
    for (const field of ['name', 'description']) {
      const value = own[field] ?? catalog[key]?.[field]
      if (typeof value !== 'string' || value.trim() === '') missing.push(`${locale}: ${key}.${field}`)
    }
  }
}

if (missing.length > 0) {
  console.error('Module texts missing (add name/description to src/modules/<key>/i18n/<locale>.json, or catalog.<key> in src/i18n/<locale>/modules.json):')
  for (const line of missing) console.error(`  ${line}`)
  process.exit(1)
}
console.log(`module texts: ${keys.length} modules x ${locales.length} locales`)
