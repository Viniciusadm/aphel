---
title: UI library
description: The @aphel/ui components and design tokens that plugin screens should use, and how plugin CSS interacts with them.
sidebar:
  order: 53
---

`@aphel/ui` is the component library of Aphel. The host provides it to plugins through the import map, and `@aphel/web-sdk` re-exports everything in it, so both imports give the same components:

```tsx
import { Button, Card, CardContent, Page, PageHeader } from '@aphel/ui'
```

The components do not depend on i18n, the session or the API. Their colors, radii, heights and shadows come from design tokens (CSS custom properties), so a plugin screen built with them follows the chosen [interface plugin](/docs/plugins/ui-plugins/) and the organization brand color automatically.

## Components

### Layout and structure

| Export | Notes |
| --- | --- |
| `Page`, `PageSection` | Page container. `Page` takes `width`: `'full'` (default), `'form'` or `'narrow'`, plus section props. |
| `PageHeader` | Props: `title`, `description?`, `actions?`, `back?: { to, label? }`, `badges?`, `icon?`, `level?: 'page' \| 'section'`, `className?`. |
| `PageTitle`, `SectionTitle` | Heading elements with the standard styles. |
| `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, `CardFooter` | Card parts. |
| `Separator`, `Skeleton`, `Spinner` | |
| `SectionWorkspace`, `SectionIndex`, `SectionLink`, `SectionContent` | A list of links next to the content of the active one. `SectionWorkspace` takes `value`, `defaultValue`, `onValueChange`. |
| `UrlTabs` | Tabs stored in the URL. Props: `tabs: { key, label, content }[]`, `param` (default `'tab'`). |
| `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`, `TabsNav`, `TabsNavItem` | Radix tabs and a router based variant. |
| `MasterDetail` | Props: `items: { id, label, actions? }[]`, `selected`, `onSelect`, `header?`, `detail`. |
| `ItemList`, `ItemRow` | A simple bordered list. |
| `CollectionView` | Searchable and filterable list. Props: `items`, `emptyTitle`, `searchLabel?`, `getSearchText?`, `filterLabel?`, `allLabel?`, `filters?`, `getFilterValue?`, `children(items)`. |
| `AreaNavigation`, `SidebarLink`, `WorkspaceOptions` | Navigation helpers used by settings screens. |

### Forms and inputs

| Export | Notes |
| --- | --- |
| `Button` | `variant`: `default`, `destructive`, `outline`, `secondary`, `ghost`, `link`. `size`: `default`, `sm`, `lg`, `icon`. `asChild` renders the child element (for links). |
| `Input`, `Textarea`, `Label`, `Checkbox`, `Switch` | Native element wrappers. |
| `Field` | Label, description and error around a control. Props: `label`, `id?`, `description?`, `error?`, `required?`, `hideLabel?`, `labelAction?`, `inline?`, `className?`, `children` (nodes or a function receiving `{ id, 'aria-describedby', 'aria-invalid' }`). |
| `FieldError` | Error text. |
| `SelectField` | Native select. Props: `value`, `onValueChange`, `options: { value, label }[]`, `emptyLabel?`, `allowEmpty?`. |
| `Select`, `SelectTrigger`, `SelectValue`, `SelectContent`, `SelectItem`, `SelectGroup`, `SelectLabel`, `SelectSeparator` | Radix select. |
| `SearchInput` | Props: `value`, `onValueChange`, `wrapperClassName?`, `clearLabel?`. |
| `ToggleField` | Checkbox or switch with label. Props: `label`, `checked`, `onCheckedChange`, `type?: 'checkbox' \| 'switch'`, `description?`, `error?`, `disabled?`. |
| `OptionGroup` | Single (`value`, `onValueChange`, `clearable?`) or multiple (`multiple`, `value: string[]`) choice. `options: { value, label, icon?, disabled? }[]`. |
| `SegmentedControl` | Props: `value`, `onValueChange`, `options: { value, label, icon?, disabled?, ariaLabel? }[]`, `size?`. |
| `OptionCard` | A selectable card. |
| `MoneyInput` | Props: `value` (string), `onValueChange`, `separators: { decimal, group }`, `decimals?`, `allowNegative?`. Get separators with `moneySeparatorsFromSetting`. |
| `NumberMaskInput` | Props: `kind`, `value`, `onValueChange`, `decimalSeparator?`. |
| `DialogForm` | Form with title, submit and cancel. Props: `title`, `submitLabel`, `cancelLabel`, `onCancel`, `onSubmit` (form), `description?`, `error?`, `pending?`, `submitDisabled?`, `submitVariant?`, `presentation?: 'dialog' \| 'page'`, `back?`. |

### Overlays and feedback

| Export | Notes |
| --- | --- |
| `Dialog`, `DialogTrigger`, `DialogContent`, `DialogHeader`, `DialogTitle`, `DialogDescription`, `DialogFooter`, `DialogClose`, `DialogPortal`, `DialogOverlay` | `DialogContent` takes `size` (`sm` to `3xl`), `side` (`center`, `left`, `right`, `bottom`) and `showCloseButton`. |
| `ConfirmDialog` | Props: `open`, `onOpenChange`, `title`, `onConfirm`, `description?`, `children?`, `confirmLabel?`, `cancelLabel?`, `variant?: 'default' \| 'destructive'`, `confirmationText?` (the user must type it), `confirmDisabled?`, `pending?`, `onError?`. |
| `Popover`, `PopoverTrigger`, `PopoverContent`, `PopoverAnchor`, `PopoverClose` | |
| `DropdownMenu` and its parts (`Item`, `CheckboxItem`, `RadioGroup`, `RadioItem`, `Label`, `Separator`, `Group`, `Content`, `Trigger`, `Portal`) | |
| `Tooltip`, `TooltipTrigger`, `TooltipContent`, `TooltipProvider` | The host already provides a `TooltipProvider`. |
| `toast(input)` | `{ title, description?, variant?: 'default' \| 'success' \| 'destructive', duration?, action?: { label, onClick, altText? } }`. The host mounts the `Toaster`, so do not mount another one. |
| `Alert` | An inline error box (`role="alert"`). |
| `EmptyState` | Props: `title?`, `description?`, `icon?`, `action?`. |
| `ErrorState` | Props: `title?`, `description?`, `onRetry?`. |
| `PageLoader` | Props: `label?`, `fullScreen?`. |
| `QueryState` | Renders loading, error and empty states for a query. Props: `query`, `children(data)`, `empty?`, `isEmpty?`, `skeletonClassName?`. |

### Data display

| Export | Notes |
| --- | --- |
| `Badge` | `variant`: `default`, `secondary`, `destructive`, `success`, `warning`, `outline`; or `tone`: `neutral`, `primary`, `success`, `warning`, `destructive`. Also `size` (`sm`, `md`), `count`, `icon`. |
| `StatusBadge` | A badge that requires `tone`; adds `struck`. |
| `Avatar` | Props: `name`, `size?: 'sm' \| 'md' \| 'lg' \| 'xl'`. |
| `Table`, `TableHeader`, `TableBody`, `TableFooter`, `TableRow`, `TableHead`, `TableCell`, `TableCaption` | |
| `Pagination`, `PaginationSummary`, `PageSizeSelect` | `Pagination` props: `page`, `perPage`, `total`, `onPageChange`. |

### Utilities and labels

| Export | Notes |
| --- | --- |
| `cn(...)` | Class name merge (clsx and tailwind-merge). |
| `UiLabelsProvider`, `useUiLabels`, `defaultUiLabels`, `UiLabels` | The strings used inside components (close, retry, confirm, pagination, loading and so on). Inside the application the host already provides them in the user's language. Outside of it, wrap your tree in `UiLabelsProvider labels={{ ... }}`. The defaults are in Portuguese. |
| `parseDecimalSeparator`, `DECIMAL_SEPARATOR_CHARS`, `moneySeparatorsFromSetting` | Number parsing helpers. |

## Tokens

The tokens are CSS custom properties defined on `:root`. Use them with `var(--name)` in CSS, or through Tailwind utilities when you import `@aphel/ui/theme.css` (for example `bg-card`, `text-muted-foreground`, `border-border`, `rounded-lg`). The complete list with default values for the light and dark themes is in [Design tokens](/docs/reference/design-tokens/). The groups:

| Group | Tokens |
| --- | --- |
| Colors | `background`, `foreground`, `card`, `card-foreground`, `popover`, `popover-foreground`, `primary`, `primary-solid`, `primary-foreground`, `secondary`, `secondary-foreground`, `muted`, `muted-foreground`, `accent`, `accent-foreground`, `destructive`, `destructive-solid`, `destructive-foreground`, `success`, `success-solid`, `success-foreground`, `warning`, `warning-solid`, `warning-foreground`, `border`, `input`, `ring`, `sidebar`, `sidebar-foreground`, `sidebar-accent`, `sidebar-accent-foreground`, `sidebar-border` |
| Shape | `radius`, `button-radius`, `input-radius`, `checkbox-radius`, `badge-radius`, `card-radius`, `card-border-width`, `dialog-radius`, `popover-radius` |
| Size | `control-height-sm`, `control-height-md`, `control-height-lg` |
| Depth | `elevation-xs`, `elevation-sm`, `elevation-md`, `elevation-lg`, `overlay`, `card-shadow`, `popover-shadow` |
| Type | `font-family-sans`, `font-family-heading`, `font-family-mono` |
| Motion | `duration-fast` |

Two tokens of each pair matter for contrast: `primary` is a text and outline color that works on the page background, while `primary-solid` is a fill color that works behind `primary-foreground`. The same applies to `destructive`, `success` and `warning`.

## How plugin CSS interacts with the tokens

- Read tokens, do not redefine them. Only an [interface plugin](/docs/plugins/ui-plugins/) should change token values, and it does so through `[ui.tokens]`, not through CSS in a bundle.
- Reference tokens as `var(--card)`, never as copies of their values.
- The page layer order is `theme, base, components, plugin-ui, utilities`. The `@aphel/ui` utilities used by a public app are placed in `plugin-ui`, below `utilities`. Follow the rules in [Web bundle](/docs/plugins/web-bundle/) for prefixes and preflight.

### Organization brand color

An administrator can set a brand color for the organization. The application then sets these properties on the root element, which override the defaults of `primary`, `primary-solid` and `primary-foreground`:

| Property | Used for |
| --- | --- |
| `--org-primary` | `primary` in the light theme. |
| `--org-primary-dark` | `primary` in the dark theme. |
| `--org-primary-solid` | `primary-solid`. |
| `--org-primary-foreground` | `primary-foreground`. |

The defaults and the values generated by interface plugins are written as `var(--org-primary, <value>)`, so the brand color wins when it is set and your value applies when it is not. Do not set the `--org-*` properties yourself and do not hardcode brand colors in a plugin: use `primary` and `primary-solid`.

## Dark mode

The application currently runs only in the light theme: the root element always gets `data-theme="light"` and there is no user switch. The dark values of the tokens and the `[data-theme='dark']` rules exist, and `@aphel/ui/theme.css` defines the `dark:` variant (`dark:bg-card`) for when a dark theme is enabled. Build with tokens, and your screens will follow when that happens. Do not rely on dark mode being visible today.

## Using the library outside the application

Static [public web apps](/docs/plugins/public-web-apps/) run outside the application shell. The host serves a prebuilt stylesheet for the library at `/shared/ui.css`, which contains the base reset, the tokens and the utilities of the components, and the module at `/shared/ui.js`. A page that imports them needs its own bundle and `UiLabelsProvider`, and must also load the interface stylesheet to follow the interface plugin.

## Related pages

- [Design tokens](/docs/reference/design-tokens/)
- [UI plugins](/docs/plugins/ui-plugins/)
- [Web bundle](/docs/plugins/web-bundle/)
