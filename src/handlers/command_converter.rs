use crate::transaction::*;

use eyre::*;
use std::result::Result::Ok;
use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
    thread::JoinHandle,
};

/// Try and convert the AnyTransaction into a transacton command.
/// Valdation over required fields is done here.
pub struct CommandConverter {
    tx: Sender<TransactionCommand>,
    rx: Receiver<AnyTransaction>,
}

impl CommandConverter {
    pub fn new(rx: Receiver<AnyTransaction>, tx: Sender<TransactionCommand>) -> Self {
        Self { tx, rx }
    }

    pub fn start(self) -> JoinHandle<()> {
        thread::spawn(move || {
            loop {
                match self.rx.recv() {
                    Ok(tx) => {
                        let maybe_tx_command = match tx.command_type {
                            CommandType::Deposit => {
                                if let Some(amount) = tx.amount {
                                    Ok(TransactionCommand::Deposit(Deposit {
                                        client_id: tx.client_id,
                                        tx_id: tx.tx_id,
                                        amount,
                                    }))
                                } else {
                                    Err(eyre!(
                                        "Found erroneous deposit transaction, ignoring: {:?}",
                                        tx
                                    ))
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
                                    Err(eyre!(
                                        "Found erroneous withdrawal transaction, ignoring: {:?}",
                                        tx
                                    ))
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
                            CommandType::Chargeback => {
                                Ok(TransactionCommand::Chargeback(Chargeback {
                                    client_id: tx.client_id,
                                    tx_id: tx.tx_id,
                                }))
                            }
                            CommandType::Unknown => {
                                Err(eyre!("Found unknown transaction, ignoring: {:?}", tx))
                            }
                        };

                        match maybe_tx_command {
                            Ok(tx_command) => {
                                if self.tx.send(tx_command).is_err() {
                                    break; // Receiver has been dropped
                                }
                            }
                            Err(e) => eprintln!(
                                "Failed to convert AnyTransaction into TransactionCommand:\n{:?}",
                                e
                            ),
                        }
                    }
                    Err(_) => break, // Receiver has been dropped
                }
            }
            drop(self.tx);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_command_converter() {
        let (tx_any_tx, rx_any_tx) = channel();
        let (tx_tx_command, rx_tx_command) = channel();

        let command_converter = CommandConverter::new(rx_any_tx, tx_tx_command);
        let handle = command_converter.start();

        tx_any_tx
            .send(AnyTransaction {
                command_type: CommandType::Deposit,
                client_id: 1,
                tx_id: 1,
                amount: Some(1.0),
            })
            .unwrap();

        tx_any_tx
            .send(AnyTransaction {
                command_type: CommandType::Withdrawal,
                client_id: 2,
                tx_id: 2,
                amount: Some(2.0),
            })
            .unwrap();

        drop(tx_any_tx);

        handle.join().unwrap();

        let commands: Vec<TransactionCommand> = rx_tx_command.iter().collect();

        assert_eq!(commands.len(), 2);

        match &commands[0] {
            TransactionCommand::Deposit(deposit) => {
                assert_eq!(deposit.client_id, 1);
                assert_eq!(deposit.tx_id, 1);
                assert_eq!(deposit.amount, 1.0);
            }
            _ => panic!("Expected Deposit command"),
        }

        match &commands[1] {
            TransactionCommand::Withdrawal(withdrawal) => {
                assert_eq!(withdrawal.client_id, 2);
                assert_eq!(withdrawal.tx_id, 2);
                assert_eq!(withdrawal.amount, 2.0);
            }
            _ => panic!("Expected Withdrawal command"),
        }
    }
}
