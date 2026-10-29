import { Tags } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { PRICE_LISTS_PATH } from './api'

export function PriceListsLink() {
  const { t } = useTranslation('price_lists')
  return (
    <Button asChild size="sm" variant="outline">
      <Link to={PRICE_LISTS_PATH}>
        <Tags aria-hidden="true" />
        {t('priceLists')}
      </Link>
    </Button>
  )
}
