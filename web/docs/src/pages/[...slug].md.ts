import type { APIRoute, GetStaticPaths } from 'astro'
import type { CollectionEntry } from 'astro:content'
import { docsInOrder, markdownOf, textResponse } from '../llms'

export const getStaticPaths = (async () =>
  (await docsInOrder()).map((entry) => ({ params: { slug: entry.id }, props: { entry } }))) satisfies GetStaticPaths

export const GET: APIRoute<{ entry: CollectionEntry<'docs'> }> = ({ props }) =>
  textResponse(markdownOf(props.entry), 'text/markdown')
