import { readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import openapiTS, { astToString } from 'openapi-typescript'

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const outputFile = resolve(packageDir, 'src/schema.ts')
const defaultFile = resolve(packageDir, '../../back/openapi.json')
const source = process.argv[2] ?? process.env.APHEL_OPENAPI_URL ?? defaultFile

async function loadSpec(location) {
  if (/^https?:\/\//.test(location)) {
    const response = await fetch(location, { signal: AbortSignal.timeout(10_000) })
    if (!response.ok) throw new Error(`could not download ${location}: HTTP ${response.status}`)
    return response.json()
  }
  return JSON.parse(await readFile(resolve(process.cwd(), location), 'utf8'))
}

async function main() {
  const spec = await loadSpec(source)
  const ast = await openapiTS(spec)
  await writeFile(outputFile, astToString(ast))
  console.log(`aphel sdk types generated from ${source} -> ${outputFile}`)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
