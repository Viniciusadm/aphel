import { execFileSync } from 'node:child_process'
import { copyFile, readdir, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const webDir = resolve(packageDir, '../../../web')
const distDir = join(packageDir, 'dist')
const compiler = join(webDir, 'node_modules/typescript/bin/tsc')
const aliasPattern = /(from\s+|import\()(['"])@\/([^'"]+)\2/g

async function declarationFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const nested = await Promise.all(
    entries.map((entry) => {
      const path = join(directory, entry.name)
      return entry.isDirectory() ? declarationFiles(path) : [path]
    }),
  )
  return nested.flat().filter((path) => path.endsWith('.d.ts'))
}

function relativeSpecifier(from, target) {
  const path = relative(dirname(from), join(distDir, target)).split('\\').join('/')
  return path.startsWith('.') ? path : `./${path}`
}

async function rewriteAliases(file) {
  const source = await readFile(file, 'utf8')
  const rewritten = source.replace(
    aliasPattern,
    (_match, prefix, quote, target) => `${prefix}${quote}${relativeSpecifier(file, target)}${quote}`,
  )
  if (rewritten !== source) await writeFile(file, rewritten)
}

async function main() {
  await rm(distDir, { recursive: true, force: true })
  execFileSync(process.execPath, [compiler, '-p', join(packageDir, 'tsconfig.generate.json')], {
    cwd: packageDir,
    stdio: 'inherit',
  })
  await Promise.all((await declarationFiles(distDir)).map(rewriteAliases))
  await copyFile(join(webDir, 'src/sdk/theme.css'), join(packageDir, 'theme.css'))
  await copyFile(join(webDir, 'src/sdk/tokens.css'), join(packageDir, 'tokens.css'))
  console.log('aphel ui types generated')
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
