import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import type { Plugin } from 'vite'

const TEMPLATE_PATH = fileURLToPath(new URL('./service-worker.js', import.meta.url))
const VERSION_PLACEHOLDER = "'__APHEL_SW_VERSION__'"
const PRECACHE_PLACEHOLDER = "['__APHEL_SW_PRECACHE__']"

interface BundleEntry {
  type: 'chunk' | 'asset'
  fileName: string
  isEntry?: boolean
  imports?: string[]
  code?: string
  source?: string | Uint8Array
}

function precacheUrls(entries: BundleEntry[], base: string): string[] {
  const byName = new Map(entries.map((entry) => [entry.fileName, entry]))
  const urls = new Set<string>()
  const visit = (fileName: string) => {
    const url = `${base}${fileName}`
    if (urls.has(url)) return
    urls.add(url)
    for (const imported of byName.get(fileName)?.imports ?? []) visit(imported)
  }
  for (const entry of entries) {
    if (entry.type === 'chunk' && entry.isEntry) visit(entry.fileName)
    if (entry.type === 'asset' && entry.fileName.startsWith('assets/') && entry.fileName.endsWith('.css')) {
      urls.add(`${base}${entry.fileName}`)
    }
  }
  return [...urls]
}

export function serviceWorkerPlugin(): Plugin {
  let base = '/'
  return {
    name: 'aphel-service-worker',
    apply: 'build',
    enforce: 'post',
    configResolved(config) {
      base = config.base
    },
    generateBundle(_options, bundle) {
      const entries = Object.values(bundle) as BundleEntry[]
      const hash = createHash('sha256')
      for (const entry of [...entries].sort((left, right) => left.fileName.localeCompare(right.fileName))) {
        hash.update(entry.fileName)
        hash.update('\0')
        hash.update(entry.type === 'chunk' ? (entry.code ?? '') : (entry.source ?? ''))
        hash.update('\0')
      }
      const version = hash.digest('hex').slice(0, 16)
      const source = readFileSync(TEMPLATE_PATH, 'utf8')
        .replace(VERSION_PLACEHOLDER, JSON.stringify(version))
        .replace(PRECACHE_PLACEHOLDER, JSON.stringify(precacheUrls(entries, base)))
      this.emitFile({ type: 'asset', fileName: 'sw.js', source })
    },
  }
}
