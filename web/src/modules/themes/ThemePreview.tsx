import { useEffect, type CSSProperties } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { base64ToBuffer } from './lib/package'
import { previewStyle } from './lib/tokens'
import type { FontFile, TokenSets } from './lib/types'

interface ThemePreviewProps {
  tokens: TokenSets
  fonts: FontFile[]
  mode: 'light' | 'dark'
}

function useDraftFonts(fonts: FontFile[]) {
  useEffect(() => {
    const faces = fonts.map(
      (font) =>
        new FontFace(font.family, base64ToBuffer(font.data), { weight: String(font.weight), style: font.style }),
    )
    for (const face of faces) {
      document.fonts.add(face)
      void face.load().catch(() => undefined)
    }
    return () => {
      for (const face of faces) document.fonts.delete(face)
    }
  }, [fonts])
}

export function ThemePreview({ tokens, fonts, mode }: ThemePreviewProps) {
  const { t } = useTranslation('themes')
  useDraftFonts(fonts)
  const style: CSSProperties = {
    ...previewStyle(tokens, mode),
    fontFamily: 'var(--font-family-sans)',
  }

  return (
    <div
      data-theme={mode === 'dark' ? 'dark' : undefined}
      style={style}
      className="grid gap-4 rounded-lg border bg-background p-4 text-foreground"
      aria-label={t('preview.label')}
      role="group"
    >
      <div className="flex flex-wrap gap-2">
        <Button>{t('preview.primary')}</Button>
        <Button variant="secondary">{t('preview.secondary')}</Button>
        <Button variant="outline">{t('preview.outline')}</Button>
        <Button variant="ghost">{t('preview.ghost')}</Button>
        <Button variant="destructive">{t('preview.destructive')}</Button>
      </div>
      <div className="flex flex-wrap gap-2">
        <Badge>{t('preview.badge')}</Badge>
        <Badge variant="secondary">{t('preview.badge')}</Badge>
        <Badge variant="success">{t('preview.badge')}</Badge>
        <Badge variant="warning">{t('preview.badge')}</Badge>
        <Badge variant="destructive">{t('preview.badge')}</Badge>
        <Badge variant="outline">{t('preview.badge')}</Badge>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>{t('preview.cardTitle')}</CardTitle>
          <CardDescription>{t('preview.cardDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="grid gap-2.5">
            <Label htmlFor="preview-input">{t('preview.field')}</Label>
            <Input id="preview-input" placeholder={t('preview.placeholder')} />
          </div>
          <Select>
            <SelectTrigger aria-label={t('preview.select')}>
              <SelectValue placeholder={t('preview.select')} />
            </SelectTrigger>
          </Select>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 text-sm">
              <Checkbox defaultChecked /> {t('preview.checkbox')}
            </label>
            <label className="flex items-center gap-2 text-sm">
              <Switch defaultChecked /> {t('preview.switch')}
            </label>
          </div>
        </CardContent>
      </Card>
      <Tabs defaultValue="one">
        <TabsList>
          <TabsTrigger value="one">{t('preview.tabOne')}</TabsTrigger>
          <TabsTrigger value="two">{t('preview.tabTwo')}</TabsTrigger>
        </TabsList>
      </Tabs>
      <Table>
        <TableHeader>
        <TableRow>
          <TableHead>{t('preview.columnName')}</TableHead>
          <TableHead className="text-right">{t('preview.columnValue')}</TableHead>
        </TableRow>
        </TableHeader>
        <TableBody>
        <TableRow>
          <TableCell>{t('preview.rowOne')}</TableCell>
          <TableCell className="text-right">1.240</TableCell>
        </TableRow>
        <TableRow>
          <TableCell>{t('preview.rowTwo')}</TableCell>
          <TableCell className="text-right">860</TableCell>
        </TableRow>
        </TableBody>
      </Table>
    </div>
  )
}
