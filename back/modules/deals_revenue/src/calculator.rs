use aphel_sdk::{
    deals::{DealRevenue, RevenueCalculator},
    line_items::LineItem,
};
use chrono::NaiveDate;
pub(crate) struct ItemsAmountCalculator;
impl RevenueCalculator for ItemsAmountCalculator {
    fn project(&self, revenue: &mut DealRevenue, items: &[LineItem], won_on: Option<NaiveDate>) {
        crate::recurring::apply_revenue(revenue, items, won_on);
    }
}
