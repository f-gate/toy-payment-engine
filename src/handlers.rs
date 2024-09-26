use crate::state::*;
use csv::Reader;
use eyre::Result;
use eyre::*;
use std::fs::File;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

struct CsvReader {
    tx: Sender<AnyTransaction>,
}

impl CsvReader {
    pub fn new(tx: Sender<AnyTransaction>) -> Self {
        Self { tx }
    }

    // Todo log and ignore erroneous lines.
    pub fn start(self, file_name: String, thread_count: u8) -> Result<()> {
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
                Err(e) => todo!("Handle error"),
            }
        });
    }
}
