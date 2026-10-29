#!/usr/bin/env node
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'
import { componentize } from '@bytecodealliance/componentize-js'

const sdkDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const witPath = join(sdkDir, 'wit', 'plugin.wit')
const usage = 'usage: aphel-plugin-js <entry.js|entry.ts> [-o backend.wasm]'

const args = process.argv.slice(2)
const entry = args.find((arg, index) => !arg.startsWith('-') && args[index - 1] !== '-o')
const outputIndex = args.indexOf('-o')
const output = resolve(outputIndex >= 0 ? args[outputIndex + 1] : 'backend.wasm')

if (!entry) {
  console.error(usage)
  process.exit(1)
}

const workDir = await mkdtemp(join(tmpdir(), 'aphel-plugin-'))
try {
  const bundle = join(workDir, 'plugin.js')
  const exportsList = ['onHook', 'onEvent', 'onRoute', 'onSchedule', 'onAction', 'onWidget']
  await build({
    stdin: {
      contents: [
        `import * as entry from ${JSON.stringify(resolve(entry))}`,
        'const plugin = entry.default ?? entry',
        ...exportsList.map((name) => `export const ${name} = plugin.${name}`),
      ].join('\n'),
      resolveDir: dirname(resolve(entry)),
      sourcefile: 'aphel-plugin-entry.js',
      loader: 'js',
    },
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    target: 'es2022',
    outfile: bundle,
    external: ['aphel:*', 'wasi:*'],
    alias: { '@aphel/plugin-sdk': join(sdkDir, 'src', 'index.js') },
    logLevel: 'warning',
  })
  const { component } = await componentize({
    sourcePath: bundle,
    witWorld: await readFile(witPath, 'utf8'),
    worldName: 'plugin',
    disableFeatures: ['http', 'fetch-event'],
  })
  await writeFile(output, component)
  console.log(`built ${output} (${(component.byteLength / 1024 / 1024).toFixed(1)} MB)`)
} finally {
  await rm(workDir, { recursive: true, force: true })
}
