import starlight from '@astrojs/starlight'
import { defineConfig } from 'astro/config'
import starlightLinksValidator from 'starlight-links-validator'
import { sections } from './src/sections.ts'

export default defineConfig({
  site: process.env.DOCS_SITE_URL,
  base: '/docs',
  outDir: '../dist/docs',
  integrations: [
    starlight({
      title: 'Aphel Docs',
      description: 'Documentation for Aphel, the open source platform for composing business systems from plugins.',
      logo: { src: './src/assets/logo.svg', alt: 'Aphel' },
      favicon: '/favicon.svg',
      customCss: ['./src/styles/custom.css'],
      lastUpdated: false,
      sidebar: sections.map(({ label, directory }) => ({ label, items: [{ autogenerate: { directory } }] })),
      plugins: [starlightLinksValidator({ errorOnLocalLinks: false })],
    }),
  ],
})
