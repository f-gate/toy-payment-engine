use crate::types::*;
use crate::transaction::*;
use crate::account::*;
use crate::validated_transaction::*;
use csv::Reader;
use eyre::*;
use std::{fs::File, sync::{mpsc::{Receiver, Sender}}, thread, collections::HashMap};

/// Used for reading line by line and deserialising.
struct CsvReader {
    tx: Sender<AnyTransaction>,
}
impl CsvReader {
    pub fn new(tx: Sender<AnyTransaction>) -> Self {
        Self { tx }
    }

    // Todo log and ignore erroneous lines.
    pub fn start(self, file_name: String, _thread_count: u8) -> Result<()> {
        thread::spawn(move || {
            let file = File::open(file_name).unwrap();
            let mut rdr = Reader::from_reader(file);

            for result in rdr.deserialize() {
                let tx: AnyTransaction = result.unwrap();
                self.tx.send(tx).unwrap();
            }
        });

        Ok(())
    }
}

/// Try and convert the AnyTransaction into a transacton command.
/// Valdation over required fields is done here.
struct CommandConverter {
    tx: Sender<TransactionCommand>,
    rx: Receiver<AnyTransaction>,
}

impl CommandConverter {
    pub fn new(rx: Receiver<AnyTransaction>, tx: Sender<TransactionCommand>) -> Self {
        Self { tx, rx }
    }

    pub fn start(self) {
        thread::spawn(move || loop {
            match self.rx.recv() {
                Result::Ok(tx) => {
                    let maybe_tx_command = match tx.command_type {
                        CommandType::Deposit => {
                            if let Some(amount) = tx.amount {
                                Ok(TransactionCommand::Deposit(Deposit {
                                    client_id: tx.client_id,
                                    tx_id: tx.tx_id,
                                    amount,
                                }))
                            } else {
                                Err(eyre!("found erronious deposit tx, ignoring: {:?}", tx))
                            }
                        }
                        CommandType::Withdrawal => {
                            if let Some(amount) = tx.amount {
                                Ok(TransactionCommand::Withdrawal(Withdrawal {
                                    client_id: tx.client_id,
                                    tx_id: tx.tx_id,
                                    amount,
                                }))
                            } else {
                                Err(eyre!("found erronious withdrawal tx, ignoring: {:?}", tx))
                            }
                        }
                        CommandType::Dispute => Ok(TransactionCommand::Dispute(Dispute {
                            client_id: tx.client_id,
                            tx_id: tx.tx_id,
                        })),
                        CommandType::Resolve => Ok(TransactionCommand::Resolve(Resolve {
                            client_id: tx.client_id,
                            tx_id: tx.tx_id,
                        })),
                        CommandType::Chargeback => Ok(TransactionCommand::Chargeback(Chargeback {
                            client_id: tx.client_id,
                            tx_id: tx.tx_id,
                        })),
                        CommandType::Unknown => Err(eyre!("found unknown tx, ignoring: {:?}", tx)),
                    };

                    match maybe_tx_command {
                        Result::Ok(tx_command) => self.tx.send(tx_command).unwrap(),
                        Err(e) => tracing::error!(
                            "Failed to convert AnyTransaction into TransactionCommand:\n{:?}",
                            e
                        ),
                    }
                }
                Err(_e) => break,
            }
        });
    }
}

struct AccountManager {
    accounts: HashMap<ClientId, Account>,
    tx_id_to_deposit: HashMap<TxId, DepositState>,
    rx: Receiver<TransactionCommand>,
}

impl AccountManager {
    pub fn new(rx: Receiver<TransactionCommand>) -> Self {
        Self {
            accounts: HashMap::new(),
            tx_id_to_deposit: HashMap::new(),
            rx,
        }
    }

    pub fn start(mut self) {
        thread::spawn(move || loop {
            match self.rx.recv() {
                Result::Ok(tx_command) => {
                    if let TransactionCommand::Deposit(deposit) = &tx_command {
                        self.tx_id_to_deposit.insert(
                            deposit.tx_id,
                            DepositState {
                                client_id: deposit.client_id,
                                amount: deposit.amount,
                                is_under_dispute: false,
                            },
                        );
                    }

                    match self.validate_transaction(&tx_command) {
                        Result::Ok(validated_tx) => {
                            if let Some(actioning_account) =
                                self.find_actioning_account(&validated_tx)
                            {
                                Self::execute_command(actioning_account, &validated_tx);
                            } else {
                                tracing::error!(
                                    "Cannot find actioning account for transaction: {:?}",
                                    validated_tx
                                );
                                continue;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to validate transaction: {:?}", e);
                            continue;
                        }
                    }
                }
                Err(_) => break,
            }
        });
    }

    fn validate_transaction(
        &self,
        tx_command: &TransactionCommand,
    ) -> Result<ValidatedTransactionCommand> {
        match tx_command {
            TransactionCommand::Deposit(deposit) => Ok(ValidatedTransactionCommand::Deposit(
                ValidDeposit {
                    client_id: deposit.client_id,
                    tx_id: deposit.tx_id,
                    amount: deposit.amount,
                },
            )),
            TransactionCommand::Withdrawal(withdrawal) => {
                // Access the account immutably for validation
                if let Some(account) = self.accounts.get(&withdrawal.client_id) {
                    if account.available >= withdrawal.amount {
                        Ok(ValidatedTransactionCommand::Withdrawal(ValidWithdrawal {
                            client_id: withdrawal.client_id,
                            tx_id: withdrawal.tx_id,
                            amount: withdrawal.amount,
                        }))
                    } else {
                        Err(eyre!(
                            "Cannot withdraw, not enough funds. {:?}",
                            withdrawal
                        ))
                    }
                } else {
                    Err(eyre!(
                        "Account not found for withdrawal: {:?}",
                        withdrawal
                    ))
                }
            }
            TransactionCommand::Dispute(dispute) => {
                let associated_tx = self
                    .tx_id_to_deposit
                    .get(&dispute.tx_id)
                    .ok_or_else(|| eyre!("Unable to find associated transaction for dispute"))?;
                Ok(ValidatedTransactionCommand::Dispute(ValidDispute {
                    tx_id: dispute.tx_id,
                    raising_client_id: dispute.client_id,
                    contended_client_id: associated_tx.client_id,
                    amount: associated_tx.amount,
                }))
            }
            TransactionCommand::Resolve(resolve) => {
                let associated_tx = self
                    .tx_id_to_deposit
                    .get(&resolve.tx_id)
                    .ok_or_else(|| eyre!("Unable to find associated transaction for resolve"))?;
                if associated_tx.is_under_dispute {
                    Ok(ValidatedTransactionCommand::Resolve(ValidResolve {
                        tx_id: resolve.tx_id,
                        raising_client_id: resolve.client_id,
                        contended_client_id: associated_tx.client_id,
                        amount: associated_tx.amount,
                    }))
                } else {
                    Err(eyre!("Transaction is not under dispute: {:?}", resolve))
                }
            }
            TransactionCommand::Chargeback(chargeback) => {
                let associated_tx = self
                    .tx_id_to_deposit
                    .get(&chargeback.tx_id)
                    .ok_or_else(|| eyre!("Unable to find associated transaction for chargeback"))?;
                if associated_tx.is_under_dispute {
                    Ok(ValidatedTransactionCommand::Chargeback(ValidChargeback {
                        tx_id: chargeback.tx_id,
                        raising_client_id: chargeback.client_id,
                        contended_client_id: associated_tx.client_id,
                        amount: associated_tx.amount,
                    }))
                } else {
                    Err(eyre!(
                        "Transaction is not under dispute: {:?}",
                        chargeback
                    ))
                }
            }
        }
    }

    fn find_actioning_account(
        &mut self,
        tx_command: &ValidatedTransactionCommand,
    ) -> Option<&mut Account> {
        match tx_command {
            ValidatedTransactionCommand::Deposit(deposit) => {
                Some(self.accounts.entry(deposit.client_id).or_insert(Default::default()))
            }
            ValidatedTransactionCommand::Withdrawal(withdrawal) => {
                self.accounts.get_mut(&withdrawal.client_id)
            }
            ValidatedTransactionCommand::Dispute(dispute) => {
                self.accounts.get_mut(&dispute.contended_client_id)
            }
            ValidatedTransactionCommand::Resolve(resolve) => {
                self.accounts.get_mut(&resolve.contended_client_id)
            }
            ValidatedTransactionCommand::Chargeback(chargeback) => {
                self.accounts.get_mut(&chargeback.contended_client_id)
            }
        }
    }

    fn execute_command(
        account: &mut Account,
        tx_command: &ValidatedTransactionCommand,
    ) {
        match tx_command {
            ValidatedTransactionCommand::Deposit(deposit) => {
                account.deposit(deposit.amount);
            }
            ValidatedTransactionCommand::Withdrawal(withdrawal) => {
                account.withdraw(withdrawal.amount);
            }
            ValidatedTransactionCommand::Dispute(dispute) => {
                account.freeze_funds(dispute.amount);
            }
            ValidatedTransactionCommand::Resolve(resolve) => {
                account.thaw_funds(resolve.amount);
            }
            ValidatedTransactionCommand::Chargeback(chargeback) => {
                account.withdraw(chargeback.amount);
                account.lock_account(LockReason::Chargeback);
            }
        }
    }
}