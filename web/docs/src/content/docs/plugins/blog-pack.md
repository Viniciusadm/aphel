---
title: Blog and site packs
description: What the Blog, Institutional site, Pages, Comments and Blog plugins do, what each pack installs and how to operate them.
---

## Plugins and packs

A pack lists plugins and seeds a few records; it contains no code of its own. Three plugins make the blog and the site:

| Plugin | What it is |
| --- | --- |
| **Blog** `1.5.1` | The product for people who write: ideas, posts, series, authors, categories, tags, feeds, import and export. |
| **Pages** `1.0.0` | Site pages built from blocks: Home, About, Services, Contact. |
| **Comments** `0.3.0` | Visitor comments with moderation and staff replies, for any record that has a public page. |

| Pack | Installs | Creates |
| --- | --- | --- |
| **Blog** `2.5.1` | Blog, Pages, Comments | A blog, an author for the person who imports, an About page in draft, menu items, the home page pointing at the blog, categories and tags |
| **Institutional site** `1.1.0` | Pages | Home, About, Services and Contact in draft with example text, their menu items and the home page |

Both need Aphel `>=0.1.1` built with `objects`, `site` and `publication`; the Blog pack also needs `tagging`. The pack checks the host version and compiled modules before installing a plugin or creating resources. Disabled required modules are shown in consent and enabled by the same import.

## Import and publish

1. Sign in as an administrator and open the plugin import screen. Select `blog-2.5.1.aphelpack` (or `institutional-1.1.0.aphelpack`) and verify it with the adjacent SHA-256 file.
2. Review the consent dialog and import. The API equivalent is `POST /api/v1/wasm-runtime/packs` with the multipart field `package`, first with `dry_run=true`, then with `acknowledge_web_version=<pack version>`.
3. Open Site → *Overview*: it lists what is left (name and logo, home page, menu, first published content, search engines).
4. The **Blog** group of the menu has *Write*, *Posts*, *Series*, *Comments*, *Authors*, *Categories and tags* and *Blog settings*.

If `/blog` or `/sobre` is already taken, the pack takes a suffix (`/blog-2`). Importing again keeps what people edited and does not duplicate anything. Administrators and managers publish (`blog.publish`, `pages.publish`).

## Writing a post

- **Write** opens with one button, *New post*, which creates a draft and opens the editor; the arrow next to it starts from a template (tutorial, news, list, review, interview, announcement). An ideas box keeps one-line titles until you decide to write them.
- **The editor** is one page of text: the title on top, a toolbar, `/` to insert headings, lists, quotes, code, dividers, images, galleries, video, callouts, embeds, FAQ and buttons. Pasting from Google Docs or Word keeps headings, lists, links and bold. An image pasted or dropped is uploaded, attached to the post and asks for its description. Focus mode hides everything but the text; the preview shows the post in the real theme as you type.
- **Saving** is automatic. The title becomes the address while the post is a draft.
- **Publish** opens a checklist (title, text, summary, cover and descriptions, categories, headings) and how the post looks on Google. Required items block; suggestions do not. Publish now or schedule; once live the panel offers the link and share buttons.
- **Posts** shows a board by stage (ideas, drafts, in review, scheduled, published), a list with bulk actions and a calendar.
- **Series** group posts that are read in sequence; each post lists its series and `/blog/serie/<address>` shows it.

## Comments

The Comments plugin adds the moderation inbox to the Blog group. New comments wait for approval; a notification points to the inbox. Approved comments show under the post, oldest first; staff replies appear nested with a team badge. *Blog settings → Accept comments* shows or hides the comment area on posts.

## Review and scheduling

Enable Approvals to require review, then create a policy for `blog.post` at `publication.activate`. Choose approvers other than the requester: people cannot approve their own requests, and an empty eligible approver set blocks publication explicitly. Approval identifies the captured revision; editing the draft while it is under review does not change what approval publishes. Scheduling stores a captured revision too. The worker must run (`aphel serve --with-worker`, or a separate worker) for scheduled activation and approval events.

A configured active approval policy remains a requirement when Approvals is disabled. Publication stops with reason `publication_approvals_unavailable`; enable the provider and submit again. See [Publication](/_aphel/docs/plugins/publication/).

## Categories, tags and appearance

Categories (hierarchical) and tags are chosen in the post details. Their selection is captured with the revision. People with `blog.terms.manage` (managers) create, rename and delete terms in *Categories and tags*. Site → *Appearance* manages global identity and the presentation of the blog; see [Site pages](/_aphel/docs/plugins/site-pages/).

## Import and export

*Blog settings → Import and export* reads the XML exported by WordPress or Markdown files with front matter (`title`, `slug`, `description`, `tags`, `categories`, `author`, `draft`), plus the images of the old blog chosen alongside, which are linked to posts by file name. Posts arrive as drafts (or published, if you choose); the original date is not kept. Export gives a zip with one Markdown file per post and the images. Everything runs in the browser.

## For plugin authors

Pages and Blog describe a *document kind* and let the host edit it: see [Document editor](/_aphel/docs/plugins/document-editor/).

## Build

From the workspace containing `app/` and `plugins/`:

```bash
CARGO_NET_OFFLINE=true cargo run --manifest-path app/sdk/cli/Cargo.toml --offline -- bundle plugins/blog/packs/blog -o plugins/packs/blog/releases/blog-2.5.1.aphelpack
CARGO_NET_OFFLINE=true cargo run --manifest-path app/sdk/cli/Cargo.toml --offline -- bundle plugins/pages/packs/institutional -o plugins/packs/institutional/releases/institutional-1.1.0.aphelpack
```

A pack recipe lists plugins by path; each plugin is packed from its own directory, so nothing is copied between them.
