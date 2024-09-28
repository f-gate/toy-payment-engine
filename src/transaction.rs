use crate::types::*;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionCommand {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DepositState {
    pub client_id: ClientId,
    pub amount: Balance,
    pub is_under_dispute: bool,
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

