use crate::types::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedTransactionCommand {
    Deposit(ValidDeposit),
    Withdrawal(ValidWithdrawal),
    Dispute(ValidDispute),
    Resolve(ValidResolve),
    Chargeback(ValidChargeback),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidDeposit {
    pub tx_id: TxId,
    pub client_id: ClientId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidWithdrawal {
    pub tx_id: TxId,
    pub client_id: ClientId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidDispute {
    pub tx_id: TxId,
    pub raising_client_id: ClientId,
    pub contended_client_id: ClientId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidResolve {
    pub tx_id: TxId,
    pub raising_client_id: ClientId,
    pub contended_client_id: ClientId,
    pub amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidChargeback {
    pub tx_id: TxId,
    pub raising_client_id: ClientId,
    pub contended_client_id: ClientId,
    pub amount: Balance,
}
