#!/usr/bin/env node
import { createHash } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const DEFAULT_BASE_URL = 'packages'
const BUILTIN_PREFIX = 'aphel.'
const DEFAULT_LOCALE = 'pt-BR'
const HEADER_ORDER = ['format', 'format_version', 'min_aphel_version', 'id', 'version', 'name', 'description', 'author', 'locale']

const catalogDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryDir = resolve(catalogDir, '..')
const templatesDir = join(repositoryDir, 'back', 'kernel', 'config-packages', 'templates')

function parseArguments(argv) {
  const options = { baseUrl: DEFAULT_BASE_URL, index: join(catalogDir, 'index.json'), output: null, check: false }
  for (let position = 0; position < argv.length; position += 1) {
    const argument = argv[position]
    if (argument === '--check') options.check = true
    else if (argument === '--base-url') options.baseUrl = argv[(position += 1)]
    else if (argument === '--index') options.index = resolve(argv[(position += 1)])
    else if (argument === '--output') options.output = resolve(argv[(position += 1)])
    else throw new Error(`unknown argument ${argument}`)
  }
  options.baseUrl = options.baseUrl.replace(/\/+$/, '')
  options.output ??= options.index
  return options
}

function ordered(manifest) {
  const result = {}
  for (const key of HEADER_ORDER) if (manifest[key] !== undefined) result[key] = manifest[key]
  for (const [key, value] of Object.entries(manifest)) if (!(key in result)) result[key] = value
  return result
}

function builtinPackage(entry) {
  if (!entry.id.startsWith(BUILTIN_PREFIX)) return null
  const [key] = entry.id.slice(BUILTIN_PREFIX.length).split('.')
  const templatePath = join(templatesDir, entry.locale ?? DEFAULT_LOCALE, `${key}.json`)
  if (!existsSync(templatePath)) return null
  const template = JSON.parse(readFileSync(templatePath, 'utf8'))
  return `${JSON.stringify(ordered({ ...template, id: entry.id, version: entry.version }), null, 2)}\n`
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

function main() {
  const options = parseArguments(process.argv.slice(2))
  const index = JSON.parse(readFileSync(options.index, 'utf8'))
  const problems = []
  for (const entry of index.entries) {
    if (entry.kind !== 'template' && entry.kind !== 'theme') continue
    const fileName = `${entry.id}-${entry.version}.json`
    const packagePath = join(catalogDir, 'packages', fileName)
    const generated = builtinPackage(entry)
    if (generated !== null && !options.check) writeFileSync(packagePath, generated)
    if (!existsSync(packagePath)) {
      problems.push(`${entry.id}: missing catalog/packages/${fileName}`)
      continue
    }
    const bytes = readFileSync(packagePath)
    const manifest = JSON.parse(bytes.toString('utf8'))
    if (manifest.id !== entry.id || manifest.version !== entry.version) {
      problems.push(`${entry.id}: the package declares ${manifest.id} ${manifest.version}`)
    }
    const expected = { url: `${options.baseUrl}/${fileName}`, sha256: sha256(bytes), size: bytes.length }
    if (options.check) {
      if (entry.package?.url !== expected.url) problems.push(`${entry.id}: url differs from ${expected.url}`)
      if (entry.package?.sha256 !== expected.sha256) problems.push(`${entry.id}: sha256 differs from the file`)
      if (entry.package?.size !== undefined && entry.package.size !== expected.size) problems.push(`${entry.id}: size differs from the file`)
      if (generated !== null && generated !== bytes.toString('utf8')) problems.push(`${entry.id}: the package is outdated, run node catalog/scripts/build.mjs`)
    } else {
      entry.package = expected
    }
  }
  if (problems.length > 0) {
    for (const problem of problems) console.error(problem)
    process.exit(1)
  }
  if (!options.check) {
    writeFileSync(options.output, `${JSON.stringify(index, null, 2)}\n`)
    console.log(`wrote ${options.output}`)
  } else {
    console.log('catalog is consistent')
  }
}

main()
