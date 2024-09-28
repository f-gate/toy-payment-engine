use crate::types::*;
use crate::transaction::*;
use crate::account::*;
use crate::validated_transaction::*;

use eyre::*;
use std::{sync::{mpsc::{Receiver}}, thread, thread::JoinHandle, collections::HashMap};
use std::result::Result::Ok;

pub struct AccountManager {
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

    pub fn start(mut self) -> JoinHandle<HashMap<ClientId, Account>> {
        thread::spawn(move || {
            loop {
                match self.rx.recv() {
                    Ok(tx_command) => {
                        // Insert deposits into tx_id_to_deposit
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
                            Ok(validated_tx) => {
                                if let Some(actioning_account) = self.find_actioning_account(&validated_tx) {
                                    if actioning_account.locked.is_none() {
                                        // Execute the command
                                        AccountManager::execute_command(actioning_account, &validated_tx);
                                    } else {
                                        eprintln!("Cannot action on a locked account: {:?}", actioning_account);
                                    }
                                } else {
                                    eprintln!("Cannot find actioning account for transaction: {:?}", validated_tx);
                                    continue;
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to validate transaction: {:?}", e);
                                continue;
                            }
                        }
                    }
                    Err(_) => break, // Receiver has been dropped
                }
            }
            self.accounts
        })
    }
    
   fn validate_transaction(
    &mut self,
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
                let associated_tx = self.tx_id_to_deposit.get_mut(&dispute.tx_id).ok_or_else(|| {
                    eyre!("Unable to find associated transaction for dispute")
                })?;
    
                if associated_tx.is_under_dispute {
                    return Err(eyre!("Transaction is already under dispute: {:?}", dispute));
                }
    
                associated_tx.is_under_dispute = true;
    
                Ok(ValidatedTransactionCommand::Dispute(ValidDispute {
                    tx_id: dispute.tx_id,
                    raising_client_id: dispute.client_id,
                    contended_client_id: associated_tx.client_id,
                    amount: associated_tx.amount,
                }))
            }
            TransactionCommand::Resolve(resolve) => {
                let associated_tx = self.tx_id_to_deposit.get_mut(&resolve.tx_id).ok_or_else(|| {
                    eyre!("Unable to find associated transaction for resolve")
                })?;
    
                if !associated_tx.is_under_dispute {
                    return Err(eyre!("Transaction is not under dispute: {:?}", resolve));
                }
    
                associated_tx.is_under_dispute = false;
    
                Ok(ValidatedTransactionCommand::Resolve(ValidResolve {
                    tx_id: resolve.tx_id,
                    raising_client_id: resolve.client_id,
                    contended_client_id: associated_tx.client_id,
                    amount: associated_tx.amount,
                }))
            }
            TransactionCommand::Chargeback(chargeback) => {
                let associated_tx = self.tx_id_to_deposit.get_mut(&chargeback.tx_id).ok_or_else(|| {
                    eyre!("Unable to find associated transaction for chargeback")
                })?;
    
                if !associated_tx.is_under_dispute {
                    return Err(eyre!("Transaction is not under dispute: {:?}", chargeback));
                }
    
                associated_tx.is_under_dispute = false;
    
                Ok(ValidatedTransactionCommand::Chargeback(ValidChargeback {
                    tx_id: chargeback.tx_id,
                    raising_client_id: chargeback.client_id,
                    contended_client_id: associated_tx.client_id,
                    amount: associated_tx.amount,
                }))
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

    fn execute_command(account: &mut Account, tx_command: &ValidatedTransactionCommand) {
        match tx_command {
            ValidatedTransactionCommand::Deposit(deposit) => {
                account.deposit(deposit.amount);
            },
            ValidatedTransactionCommand::Withdrawal(withdrawal) => {
                account.withdraw(withdrawal.amount);
            },
            ValidatedTransactionCommand::Dispute(dispute) => {
                account.freeze_funds(dispute.amount);
            },
            ValidatedTransactionCommand::Resolve(resolve) => {
                account.thaw_funds(resolve.amount);
            },
            ValidatedTransactionCommand::Chargeback(chargeback) => {
                account.chargeback(chargeback.amount);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_account_manager() {
        let (tx_tx_command, rx_tx_command) = channel();

        let account_manager = AccountManager::new(rx_tx_command);
        let handle = account_manager.start();

        tx_tx_command
            .send(TransactionCommand::Deposit(Deposit {
                client_id: 1,
                tx_id: 1,
                amount: 1.0,
            }))
            .unwrap();

        tx_tx_command
            .send(TransactionCommand::Withdrawal(Withdrawal {
                client_id: 1,
                tx_id: 2,
                amount: 0.5,
            }))
            .unwrap();

        drop(tx_tx_command);

        let accounts = handle.join().unwrap();

        let account = accounts.get(&1).expect("Account not found");
        assert_eq!(account.available, 0.5);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total(), 0.5);
        assert!(account.locked.is_none());
    }
}
