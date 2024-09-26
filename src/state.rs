
type ClientId = u16;
type TxId = u32;
type Balance = f32;

#[derive(Debug, Clone, PartialEq)]
enum TransactionCommand {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    ChargeBack(ChargeBack),
}

#[derive(Debug, Clone, PartialEq)]
struct Account {
    /// The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts.
    available: Balance,
    /// The total funds that are held for dispute. This should be equal to total - available amounts.
    held: Balance,
    /// The total funds that are available or held. This should be equal to available + held.
    total: Balance,
    /// Whether the account is locked. An account is locked if a charge back occurs
    locked: Option<Locked>
}

#[derive(Debug, Clone, PartialEq)]
struct AnyTransaction {
    #[serde(rename = "type")]
    command_type: CommandType,
    #[serde(rename = "tx")]
    tx_id: TxId,
    #[serde(rename = "client")]
    client_id: ClientId,
    amount: Option<Balance>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]  
enum CommandType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
    #[default]
    Unknown, 
}

#[derive(Debug, Clone, PartialEq)]
struct Deposit {
    client_id: ClientId,
    tx_id: TxId,
    amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
struct Withdrawal {
    client_id: ClientId,
    tx_id: TxId,
    amount: Balance,
}

#[derive(Debug, Clone, PartialEq)]
struct Dispute {
    client_id: ClientId,
    tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
struct Resolve {
    client_id: ClientId,
    tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
struct Chargeback {
    client_id: ClientId,
    tx_id: TxId,
}

#[derive(Debug, Clone, PartialEq)]
struct Locked {
    locked_at: u32,
    reason_for_lock: LockReason,
}

#[derive(Debug, Clone, PartialEq)]
enum LockReason {
    ChargeBack
}

