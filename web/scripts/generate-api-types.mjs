import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { readFile, writeFile } from 'node:fs/promises'
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

async function main() {
  ensureToolInstalled()
  const { label, spec } = await loadSpec()
  const entry = resolve(toolDir, 'node_modules/openapi-typescript/dist/index.mjs')
  const { default: openapiTS, astToString } = await import(pathToFileURL(entry).href)
  const ast = await openapiTS(spec)
  await writeFile(outputFile, astToString(ast))
  console.log(`api types generated from ${label} -> ${outputFile}`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
