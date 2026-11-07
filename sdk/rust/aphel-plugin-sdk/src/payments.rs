//! Pure settlement rules. The integration must authenticate events, query authoritative
//! charge state and persist the ledger with optimistic concurrency in one transaction.
use crate::{PluginError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_SAFE: u64 = 9_007_199_254_740_991;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeStatus {
    Pending,
    Captured,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeSnapshot {
    pub sale_id: String,
    pub provider: String,
    pub account: String,
    pub id: String,
    /// Monotonic authoritative revision, not webhook arrival order.
    pub revision: u64,
    pub currency: String,
    /// Integer minor units (centavos for BRL).
    pub amount: u64,
    pub refunded: u64,
    pub status: ChargeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementStatus {
    Unpaid,
    Partial,
    Pending,
    Paid,
    Overpaid,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Settlement {
    pub status: SettlementStatus,
    pub captured: u64,
    pub refunded: u64,
    pub net: u64,
    pub balance: u64,
    pub excess: u64,
    pub pending: usize,
    pub can_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentLedger {
    sale_id: String,
    currency: String,
    total: u64,
    #[serde(serialize_with = "serialize_charges")]
    charges: BTreeMap<(String, String, String), ChargeSnapshot>,
}
fn serialize_charges<S: serde::Serializer>(
    charges: &BTreeMap<(String, String, String), ChargeSnapshot>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    charges.values().collect::<Vec<_>>().serialize(serializer)
}
impl<'de> Deserialize<'de> for PaymentLedger {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Stored {
            sale_id: String,
            currency: String,
            total: u64,
            charges: Vec<ChargeSnapshot>,
        }
        let stored = Stored::deserialize(deserializer)?;
        let mut ledger = PaymentLedger::new(stored.sale_id, stored.currency, stored.total)
            .map_err(|error| serde::de::Error::custom(error.message))?;
        for charge in stored.charges {
            ledger
                .apply(charge)
                .map_err(|error| serde::de::Error::custom(error.message))?;
        }
        Ok(ledger)
    }
}

fn invalid(message: &str) -> PluginError {
    PluginError::new("payment_snapshot_invalid", message).with_status(422)
}
fn conflict(message: &str) -> PluginError {
    PluginError::new("payment_snapshot_conflict", message).with_status(409)
}
fn key(value: &str) -> bool {
    !value.is_empty() && value.len() <= 191
}

impl PaymentLedger {
    pub fn new(
        sale_id: impl Into<String>,
        currency: impl Into<String>,
        total: u64,
    ) -> Result<Self> {
        let sale_id = sale_id.into();
        let currency = currency.into();
        if !key(&sale_id)
            || currency.len() != 3
            || !currency.bytes().all(|b| b.is_ascii_uppercase())
            || total > MAX_SAFE
        {
            return Err(invalid("invalid sale, currency or total"));
        }
        Ok(Self {
            sale_id,
            currency,
            total,
            charges: BTreeMap::new(),
        })
    }
    pub fn snapshots(&self) -> Vec<ChargeSnapshot> {
        self.charges.values().cloned().collect()
    }
    pub fn apply(&mut self, snapshot: ChargeSnapshot) -> Result<bool> {
        if snapshot.sale_id != self.sale_id
            || snapshot.currency != self.currency
            || !key(&snapshot.id)
            || !key(&snapshot.provider)
            || !key(&snapshot.account)
            || snapshot.revision == 0
            || snapshot.revision > MAX_SAFE
            || snapshot.amount == 0
            || snapshot.amount > MAX_SAFE
            || snapshot.refunded > snapshot.amount
            || (snapshot.status != ChargeStatus::Captured && snapshot.refunded != 0)
        {
            return Err(invalid("invalid charge snapshot or sale/currency mismatch"));
        }
        let key = (
            snapshot.provider.clone(),
            snapshot.account.clone(),
            snapshot.id.clone(),
        );
        if let Some(previous) = self.charges.get(&key) {
            if snapshot.revision < previous.revision {
                return Ok(false);
            }
            if snapshot.revision == previous.revision {
                return if snapshot == *previous {
                    Ok(false)
                } else {
                    Err(conflict("same revision has different charge data"))
                };
            }
            if previous.status == ChargeStatus::Captured
                && (snapshot.status != ChargeStatus::Captured
                    || snapshot.amount != previous.amount
                    || snapshot.refunded < previous.refunded)
            {
                return Err(conflict(
                    "captured funds or refunds cannot be erased by a later snapshot",
                ));
            }
        } else if self.charges.len() >= 1000 {
            return Err(invalid("too many charges in one sale"));
        }
        let mut next = self.clone();
        next.charges.insert(key, snapshot);
        next.summary()?; // Reject overflow before mutating the ledger.
        *self = next;
        Ok(true)
    }
    pub fn summary(&self) -> Result<Settlement> {
        let mut captured = 0u64;
        let mut refunded = 0u64;
        let mut pending = 0usize;
        for charge in self.charges.values() {
            if charge.status == ChargeStatus::Pending {
                pending += 1;
            }
            if charge.status == ChargeStatus::Captured {
                captured = captured
                    .checked_add(charge.amount)
                    .filter(|n| *n <= MAX_SAFE)
                    .ok_or_else(|| invalid("payment total overflow"))?;
                refunded = refunded
                    .checked_add(charge.refunded)
                    .filter(|n| *n <= MAX_SAFE)
                    .ok_or_else(|| invalid("refund total overflow"))?;
            }
        }
        let net = captured - refunded;
        let balance = self.total.saturating_sub(net);
        let excess = net.saturating_sub(self.total);
        let can_complete = net == self.total && pending == 0;
        let status = if excess > 0 {
            SettlementStatus::Overpaid
        } else if pending > 0 {
            SettlementStatus::Pending
        } else if can_complete {
            SettlementStatus::Paid
        } else if captured > 0 && net == 0 {
            SettlementStatus::Refunded
        } else if net > 0 {
            SettlementStatus::Partial
        } else {
            SettlementStatus::Unpaid
        };
        Ok(Settlement {
            status,
            captured,
            refunded,
            net,
            balance,
            excess,
            pending,
            can_complete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn charge(id: &str, amount: u64) -> ChargeSnapshot {
        ChargeSnapshot {
            sale_id: "sale".into(),
            provider: "provider".into(),
            account: "account".into(),
            id: id.into(),
            revision: 1,
            currency: "BRL".into(),
            amount,
            refunded: 0,
            status: ChargeStatus::Captured,
        }
    }
    #[test]
    fn split_duplicate_excess_and_refund() {
        let mut ledger = PaymentLedger::new("sale", "BRL", 1000).unwrap();
        assert!(ledger.apply(charge("a", 400)).unwrap());
        assert!(!ledger.apply(charge("a", 400)).unwrap());
        assert_eq!(ledger.summary().unwrap().balance, 600);
        assert!(!ledger.summary().unwrap().can_complete);
        ledger.apply(charge("b", 600)).unwrap();
        assert!(ledger.summary().unwrap().can_complete);
        ledger.apply(charge("c", 100)).unwrap();
        assert_eq!(ledger.summary().unwrap().status, SettlementStatus::Overpaid);
        let mut refund = charge("c", 100);
        refund.revision = 2;
        refund.refunded = 100;
        ledger.apply(refund).unwrap();
        assert!(ledger.summary().unwrap().can_complete);
        assert!(!ledger.apply(charge("c", 100)).unwrap());
        let saved = serde_json::to_value(&ledger).unwrap();
        let restored: PaymentLedger = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.summary().unwrap(), ledger.summary().unwrap());
    }
    #[test]
    fn pending_wrong_sale_currency_and_conflicting_events() {
        let mut ledger = PaymentLedger::new("sale", "BRL", 1000).unwrap();
        ledger.apply(charge("a", 1000)).unwrap();
        let mut pending = charge("b", 500);
        pending.status = ChargeStatus::Pending;
        ledger.apply(pending.clone()).unwrap();
        assert!(!ledger.summary().unwrap().can_complete);
        pending.revision = 2;
        pending.status = ChargeStatus::Canceled;
        ledger.apply(pending).unwrap();
        assert!(ledger.summary().unwrap().can_complete);
        let mut wrong = charge("a", 900);
        assert!(ledger.apply(wrong.clone()).is_err());
        wrong.id = "c".into();
        wrong.sale_id = "other".into();
        assert!(ledger.apply(wrong).is_err());
        let mut wrong = charge("c", 100);
        wrong.currency = "USD".into();
        assert!(ledger.apply(wrong).is_err());
        let mut wrong = charge("a", 1000);
        wrong.revision = 3;
        wrong.status = ChargeStatus::Canceled;
        assert!(ledger.apply(wrong).is_err());
        assert_eq!(ledger.summary().unwrap().net, 1000);
    }
}
