import { getCollection, type CollectionEntry } from 'astro:content'
import { sections } from './sections'

type Entry = CollectionEntry<'docs'>

const site = (import.meta.env.SITE ?? '').replace(/\/$/, '')
const base = import.meta.env.BASE_URL.replace(/\/$/, '')

const summary =
  'Aphel is an open source platform for composing business systems from plugins. The core knows no business domain: a blueprint picks the plugins, configuration and navigation. It ships as a single Rust binary with the React app embedded, one installation per company, backed by MySQL. Runtime plugins (.aphelplugin) add entities, hooks, routes, screens and interfaces without rebuilding Aphel.'

export function markdownPath(slug: string) {
  return `${base}/${slug}.md`
}

export function markdownUrl(entry: Entry) {
  return `${site}${markdownPath(entry.id)}`
}

export function isPage(entry: Entry) {
  return entry.id !== 'index'
}

function rank(entry: Entry) {
  const section = sections.findIndex((item) => entry.id.startsWith(`${item.directory}/`))
  const order = entry.data.sidebar.order ?? 1000
  return (section === -1 ? sections.length : section) * 10000 + order
}

export async function docsInOrder() {
  const entries = (await getCollection('docs')).filter(isPage)
  return entries.sort((a, b) => rank(a) - rank(b) || a.id.localeCompare(b.id))
}

export function markdownOf(entry: Entry) {
  const description = entry.data.description ? `> ${entry.data.description}\n\n` : ''
  return `# ${entry.data.title}\n\n${description}${(entry.body ?? '').trim()}\n`
}

export async function llmsIndex() {
  const entries = await docsInOrder()
  const lines = ['# Aphel', '', `> ${summary}`, '']
  for (const section of sections) {
    lines.push(`## ${section.label}`, '')
    for (const entry of entries.filter((item) => item.id.startsWith(`${section.directory}/`))) {
      lines.push(`- [${entry.data.title}](${markdownUrl(entry)}): ${entry.data.description ?? ''}`.trimEnd())
    }
    lines.push('')
  }
  lines.push('## Optional', '', `- [Full documentation in one file](${site}${base}/llms-full.txt)`, '')
  return lines.join('\n')
}

export async function llmsFull() {
  const entries = await docsInOrder()
  return [`# Aphel documentation\n\n> ${summary}\n`, ...entries.map(markdownOf)].join('\n---\n\n')
}

export function textResponse(body: string, type = 'text/plain') {
  return new Response(body, { headers: { 'Content-Type': `${type}; charset=utf-8` } })
}
