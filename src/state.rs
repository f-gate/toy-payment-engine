use serde::Deserialize;

pub type ClientId = u16;
pub type TxId = u32;
pub type Balance = f32;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionCommand {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AnyTransaction {
    #[serde(rename = "type")]
    pub command_type: CommandType,
    #[serde(rename = "tx")]
    pub tx_id: TxId,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    pub amount: Option<Balance>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Deposit {
    pub client_id: ClientId,
    pub tx_id: TxId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Withdrawal {
    pub client_id: ClientId,
    pub tx_id: TxId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dispute {
    pub client_id: ClientId,
    pub tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolve {
    pub client_id: ClientId,
    pub tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chargeback {
    pub client_id: ClientId,
    pub tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Locked {
    pub locked_at: u32,
    pub reason_for_lock: LockReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LockReason {
    Chargeback,
}
