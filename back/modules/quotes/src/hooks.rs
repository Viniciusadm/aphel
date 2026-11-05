use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{Quote, QuoteAcceptance, QuoteDiscount, QuoteItemData, TotalsBreakdown};
use crate::service;
use aphel_kernel::modules::{Hook, PluginHookArgs};

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptingQuote {
    pub(crate) quote: Quote,
    pub(crate) acceptance: QuoteAcceptance,
}

pub(crate) const ACCEPTING: &str = "quote.accepting";

impl PluginHookArgs for AcceptingQuote {
    fn plugin_record_id(&self) -> String {
        self.quote_id().to_string()
    }

    fn plugin_payload(&self) -> Value {
        let mut quote = service::quote_snapshot(&self.quote);
        if let Value::Object(map) = &mut quote {
            map.insert("id".to_owned(), json!(self.quote_id()));
        }
        json!({
            "quote": quote,
            "deal_id": self.deal_id(),
            "acceptor": {
                "name": self.acceptor_name(),
                "email": self.acceptor_email(),
            },
            "manual": self.acceptance.recorded_by.is_some(),
        })
    }
}

impl AcceptingQuote {
    pub fn quote_id(&self) -> Uuid {
        self.quote.id
    }

    pub fn deal_id(&self) -> Uuid {
        self.quote.deal_id
    }

    pub fn acceptor_name(&self) -> &str {
        &self.acceptance.name
    }

    pub fn acceptor_email(&self) -> &str {
        &self.acceptance.email
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuoteAccepting;

impl Hook for QuoteAccepting {
    type Args = AcceptingQuote;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PricedQuote {
    pub(crate) quote_id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) currency: String,
    pub(crate) discount: QuoteDiscount,
    pub(crate) items: Vec<QuoteItemData>,
    pub(crate) totals: TotalsBreakdown,
}

impl PricedQuote {
    pub fn quote_id(&self) -> Uuid {
        self.quote_id
    }

    pub fn deal_id(&self) -> Uuid {
        self.deal_id
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuoteTotals;

impl Hook for QuoteTotals {
    type Args = PricedQuote;
}
