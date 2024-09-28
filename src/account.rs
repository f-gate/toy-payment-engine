use crate::types::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Account {
    /// The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts.
    pub available: Balance,
    /// The total funds that are held for dispute. This should be equal to total - available amounts.
    pub held: Balance,
    /// The total funds that are available or held. This should be equal to available + held.
    pub total: Balance,
    /// Whether the account is locked. An account is locked if a charge back occurs
    pub locked: Option<Locked>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Locked {
    pub reason_for_lock: LockReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LockReason {
    Chargeback,
}

impl Account {
    pub fn deposit(&mut self, amount: f64) {
        self.available += amount;
    }

    pub fn withdraw(&mut self, amount: f64) {
        self.available -= amount;
    }

    pub fn freeze_funds(&mut self, amount: f64) {
        self.available -= amount;
        self.held += amount;
    }

    pub fn thaw_funds(&mut self, amount: f64) {
        self.available += amount;
        self.held -= amount;
    }

    pub fn chargeback(&mut self, amount: f64) {
        self.held -= amount;
        self.locked = Some(Locked {
            reason_for_lock: LockReason::Chargeback,
        });
    }

    pub fn total(&self) -> f64 {
        self.available + self.held
    }
}
