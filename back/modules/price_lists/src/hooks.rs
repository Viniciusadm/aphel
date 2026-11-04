use std::cmp::Ordering;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::{
    BundleComponent, BundleExpansion, BundlePricing, CatalogProduct, PriceList, PriceListAppliesTo,
    PriceListCandidate, PriceListSelection, PriceListSummary, PriceRequest, ResolvedPrice,
    select_tier,
};
use crate::store::{BundleStore, DiscountLimitStore, PriceListStore, SegmentMembership};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::line_items::round_money;
use aphel_sdk::money::organization_today;

pub use aphel_sdk::pricing::{
    BundleExpander, DiscountLimits, PriceListResolver, bundle_expander, discount_limits,
    price_list_resolver,
};

pub(crate) fn summary(list: &PriceList) -> PriceListSummary {
    PriceListSummary {
        id: list.id,
        name: list.name.clone(),
        currency: list.currency.clone(),
    }
}

fn most_specific(left: &&PriceListCandidate, right: &&PriceListCandidate) -> Ordering {
    left.list
        .valid_from
        .cmp(&right.list.valid_from)
        .then_with(|| left.list.created_at.cmp(&right.list.created_at))
}

pub struct CatalogPriceListResolver;

impl CatalogPriceListResolver {
    async fn segment_match(
        ctx: &Ctx<'_>,
        candidates: &[PriceListCandidate],
        company_id: Uuid,
    ) -> Result<Option<PriceListSummary>, AppError> {
        let mut segmented: Vec<&PriceListCandidate> = candidates
            .iter()
            .filter(|candidate| candidate.list.applies_to == PriceListAppliesTo::Segment)
            .collect();
        segmented.sort_by(|left, right| most_specific(right, left));
        let membership = SegmentMembership::of(ctx);
        for candidate in segmented {
            let Some(segment_id) = candidate.list.segment_id else {
                continue;
            };
            if membership
                .contains(ctx.scope, segment_id, company_id)
                .await?
            {
                return Ok(Some(summary(&candidate.list)));
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl PriceListResolver for CatalogPriceListResolver {
    async fn find(
        &self,
        ctx: &Ctx<'_>,
        price_list_id: Uuid,
    ) -> Result<Option<PriceListSummary>, AppError> {
        Ok(PriceListStore::of(ctx)
            .find(price_list_id)
            .await?
            .as_ref()
            .map(summary))
    }

    async fn default_for(
        &self,
        ctx: &Ctx<'_>,
        selection: &PriceListSelection,
    ) -> Result<Option<PriceListSummary>, AppError> {
        let candidates = PriceListStore::of(ctx)
            .candidates(&selection.currency, selection.on, selection.company_id)
            .await?;
        if candidates.is_empty() {
            return Ok(None);
        }
        if let Some(company_id) = selection.company_id {
            let specific = candidates
                .iter()
                .filter(|candidate| {
                    candidate.list.applies_to == PriceListAppliesTo::Organizations
                        && candidate.matches_company
                })
                .max_by(most_specific);
            if let Some(candidate) = specific {
                return Ok(Some(summary(&candidate.list)));
            }
            if let Some(found) = Self::segment_match(ctx, &candidates, company_id).await? {
                return Ok(Some(found));
            }
        }
        Ok(candidates
            .iter()
            .find(|candidate| candidate.list.is_default)
            .map(|candidate| summary(&candidate.list)))
    }

    async fn price(
        &self,
        ctx: &Ctx<'_>,
        request: &PriceRequest,
    ) -> Result<Option<ResolvedPrice>, AppError> {
        let store = PriceListStore::of(ctx);
        let Some(list) = store.find(request.price_list_id).await? else {
            return Ok(None);
        };
        if !list.is_valid_on(request.on) {
            return Ok(None);
        }
        let items = store.product_items(list.id, request.product_id).await?;
        Ok(
            select_tier(&items, request.product_id, request.quantity).map(|item| ResolvedPrice {
                price_list_id: list.id,
                product_id: request.product_id,
                unit_price: item.unit_price,
                currency: list.currency.clone(),
            }),
        )
    }
}

pub struct RoleDiscountLimits;

#[async_trait]
impl DiscountLimits for RoleDiscountLimits {
    async fn max_discount_percent(
        &self,
        ctx: &Ctx<'_>,
        user_id: &UserId,
    ) -> Result<Option<Decimal>, AppError> {
        DiscountLimitStore::of(ctx).for_user(user_id).await
    }
}

pub struct CatalogBundleExpander;

impl CatalogBundleExpander {
    async fn unit_price(
        ctx: &Ctx<'_>,
        list: Option<&PriceListSummary>,
        product: &CatalogProduct,
        quantity: Decimal,
        on: NaiveDate,
    ) -> Result<Decimal, AppError> {
        let Some(list) = list else {
            return Ok(product.price);
        };
        let resolved = CatalogPriceListResolver
            .price(
                ctx,
                &PriceRequest {
                    price_list_id: list.id,
                    product_id: product.id,
                    quantity,
                    on,
                },
            )
            .await?;
        Ok(resolved.map_or(product.price, |price| price.unit_price))
    }
}

#[async_trait]
impl BundleExpander for CatalogBundleExpander {
    async fn expand(
        &self,
        ctx: &Ctx<'_>,
        product_id: Uuid,
        price_list_id: Option<Uuid>,
    ) -> Result<Option<BundleExpansion>, AppError> {
        let bundles = BundleStore::of(ctx);
        let Some(bundle) = bundles.find(product_id).await? else {
            return Ok(None);
        };
        let mut ids = vec![product_id];
        ids.extend(bundle.components.iter().map(|item| item.product_id));
        let products = bundles.products(ctx, &ids).await?;
        let Some(kit) = products.iter().find(|product| product.id == product_id) else {
            return Ok(None);
        };
        let list = match price_list_id {
            Some(id) => CatalogPriceListResolver.find(ctx, id).await?,
            None => None,
        };
        let timezone = bundles.timezone().await?;
        let on = organization_today(&timezone, ctx.now());
        let mut components = Vec::new();
        for item in &bundle.components {
            let Some(product) = products
                .iter()
                .find(|product| product.id == item.product_id)
            else {
                continue;
            };
            let unit_price =
                Self::unit_price(ctx, list.as_ref(), product, item.quantity, on).await?;
            components.push(BundleComponent {
                product_id: product.id,
                name: product.name.clone(),
                code: product.code.clone(),
                quantity: item.quantity,
                unit_price,
            });
        }
        let unit_price = match bundle.pricing {
            BundlePricing::Fixed => {
                Self::unit_price(ctx, list.as_ref(), kit, Decimal::ONE, on).await?
            }
            BundlePricing::Sum => round_money(
                components
                    .iter()
                    .map(|component| component.unit_price * component.quantity)
                    .sum(),
            ),
        };
        Ok(Some(BundleExpansion {
            product_id,
            pricing: bundle.pricing,
            unit_price,
            currency: list.map_or_else(|| kit.currency.clone(), |list| list.currency),
            components,
        }))
    }
}
