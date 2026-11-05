import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { readFile, readdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const webDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const toolDir = resolve(webDir, 'scripts/api-types')
const outputFile = resolve(webDir, 'src/api/schema.d.ts')
const remoteUrl = process.env.APHEL_OPENAPI_URL ?? 'http://127.0.0.1:3001/api/openapi.json'
const fallbackFile = resolve(webDir, '../back/openapi.json')
const explicitSource = process.argv[2]

function ensureToolInstalled() {
  if (existsSync(resolve(toolDir, 'node_modules/openapi-typescript'))) return
  const command = existsSync(resolve(toolDir, 'package-lock.json')) ? 'ci' : 'install'
  const result = spawnSync('npm', [command, '--no-audit', '--no-fund'], {
    cwd: toolDir,
    stdio: 'inherit',
  })
  if (result.status !== 0) throw new Error('failed to install openapi-typescript tooling')
}

async function fetchRemoteSpec() {
  try {
    const response = await fetch(remoteUrl, { signal: AbortSignal.timeout(3000) })
    if (!response.ok) return null
    return { label: remoteUrl, spec: await response.json() }
  } catch {
    return null
  }
}

async function readSpecFile(path) {
  return { label: path, spec: JSON.parse(await readFile(path, 'utf8')) }
}

async function loadSpec() {
  if (explicitSource) return readSpecFile(resolve(process.cwd(), explicitSource))
  const remote = await fetchRemoteSpec()
  if (remote) return remote
  if (existsSync(fallbackFile)) return readSpecFile(fallbackFile)
  throw new Error(`OpenAPI spec unavailable: ${remoteUrl} did not respond and ${fallbackFile} does not exist`)
}

const modulesDir = resolve(webDir, 'src/modules')

function collectRefs(node, found) {
  if (Array.isArray(node)) {
    for (const item of node) collectRefs(item, found)
    return
  }
  if (node && typeof node === 'object') {
    for (const [key, value] of Object.entries(node)) {
      if (key === '$ref' && typeof value === 'string') found.add(value)
      else collectRefs(value, found)
    }
  }
}

function pruneSchemas(spec, paths) {
  const schemas = spec.components?.schemas ?? {}
  const found = new Set()
  collectRefs(paths, found)
  const kept = {}
  const pending = [...found]
  while (pending.length > 0) {
    const ref = pending.pop()
    const match = /^#\/components\/schemas\/(.+)$/.exec(ref)
    if (!match || kept[match[1]] || !schemas[match[1]]) continue
    kept[match[1]] = schemas[match[1]]
    const nested = new Set()
    collectRefs(schemas[match[1]], nested)
    pending.push(...nested)
  }
  return kept
}

async function moduleTags(key) {
  try {
    const declared = JSON.parse(await readFile(resolve(modulesDir, key, 'api-tags.json'), 'utf8'))
    return [key, key.replaceAll('_', '-'), ...declared]
  } catch {
    return null
  }
}

async function writeModuleTypes(spec, openapiTS, astToString) {
  const keys = (await readdir(modulesDir, { withFileTypes: true })).filter((entry) => entry.isDirectory()).map((entry) => entry.name)
  const written = []
  for (const key of keys) {
    const declared = await moduleTags(key)
    if (!declared) continue
    const tags = new Set(declared)
    const paths = {}
    for (const [path, item] of Object.entries(spec.paths ?? {})) {
      const operations = Object.fromEntries(
        Object.entries(item).filter(([, operation]) => operation && typeof operation === 'object' && operation.tags?.some((tag) => tags.has(tag))),
      )
      if (Object.keys(operations).length > 0) paths[path] = operations
    }
    if (Object.keys(paths).length === 0) continue
    const subset = { openapi: spec.openapi, info: spec.info, paths, components: { schemas: pruneSchemas(spec, paths) } }
    await writeFile(resolve(modulesDir, key, 'api-types.d.ts'), astToString(await openapiTS(subset)))
    written.push(key)
  }
  return written
}

async function main() {
  ensureToolInstalled()
  const { label, spec } = await loadSpec()
  const entry = resolve(toolDir, 'node_modules/openapi-typescript/dist/index.mjs')
  const { default: openapiTS, astToString } = await import(pathToFileURL(entry).href)
  const ast = await openapiTS(spec)
  await writeFile(outputFile, astToString(ast))
  console.log(`api types generated from ${label} -> ${outputFile}`)
  const written = await writeModuleTypes(spec, openapiTS, astToString)
  console.log(`module api types: ${written.length} (${written.join(', ')})`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
