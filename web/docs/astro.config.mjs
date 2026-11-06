import starlight from '@astrojs/starlight'
import { defineConfig } from 'astro/config'
import starlightLinksValidator from 'starlight-links-validator'

export default defineConfig({
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
      sidebar: [
        { label: 'Getting started', items: [{ autogenerate: { directory: 'start' } }] },
        { label: 'Using Aphel', items: [{ autogenerate: { directory: 'using' } }] },
        { label: 'Administration', items: [{ autogenerate: { directory: 'admin' } }] },
        { label: 'Building plugins', items: [{ autogenerate: { directory: 'plugins' } }] },
        { label: 'Reference', items: [{ autogenerate: { directory: 'reference' } }] },
      ],
      plugins: [starlightLinksValidator({ errorOnLocalLinks: false })],
    }),
  ],
})
