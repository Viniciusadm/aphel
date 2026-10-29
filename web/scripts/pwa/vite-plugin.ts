import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import type { Plugin } from 'vite'

const TEMPLATE_PATH = fileURLToPath(new URL('./service-worker.js', import.meta.url))
const VERSION_PLACEHOLDER = "'__APHEL_SW_VERSION__'"
const PRECACHE_PLACEHOLDER = "['__APHEL_SW_PRECACHE__']"
const SHELL_URL = '/'

interface BundleEntry {
  type: 'chunk' | 'asset'
  fileName: string
  isEntry?: boolean
  imports?: string[]
}

function precacheUrls(entries: BundleEntry[]): string[] {
  const byName = new Map(entries.map((entry) => [entry.fileName, entry]))
  const urls = new Set<string>([SHELL_URL])
  const visit = (fileName: string) => {
    const url = `/${fileName}`
    if (urls.has(url)) return
    urls.add(url)
    for (const imported of byName.get(fileName)?.imports ?? []) visit(imported)
  }
  for (const entry of entries) {
    if (entry.type === 'chunk' && entry.isEntry) visit(entry.fileName)
    if (entry.type === 'asset' && entry.fileName.startsWith('assets/') && entry.fileName.endsWith('.css')) {
      urls.add(`/${entry.fileName}`)
    }
  }
  return [...urls]
}

export function serviceWorkerPlugin(): Plugin {
  return {
    name: 'aphel-service-worker',
    apply: 'build',
    enforce: 'post',
    generateBundle(_options, bundle) {
      const entries = Object.values(bundle) as BundleEntry[]
      const version = createHash('sha256')
        .update(
          entries
            .map((entry) => entry.fileName)
            .sort()
            .join('\n'),
        )
        .digest('hex')
        .slice(0, 16)
      const source = readFileSync(TEMPLATE_PATH, 'utf8')
        .replace(VERSION_PLACEHOLDER, JSON.stringify(version))
        .replace(PRECACHE_PLACEHOLDER, JSON.stringify(precacheUrls(entries)))
      this.emitFile({ type: 'asset', fileName: 'sw.js', source })
    },
  }
}
