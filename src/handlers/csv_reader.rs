use crate::transaction::*;

use csv::Reader;
use eyre::*;
use std::result::Result::Ok;
use std::{fs::File, sync::mpsc::Sender, thread};

/// Used for reading line by line and deserializing.
pub struct CsvReader {
    tx: Sender<AnyTransaction>,
}

impl CsvReader {
    pub fn new(tx: Sender<AnyTransaction>) -> Self {
        Self { tx }
    }

    // Log and ignore erroneous lines.
    pub fn start(self, file_name: String, _thread_count: u8) -> Result<thread::JoinHandle<()>> {
        let handle = thread::spawn(move || {
            let file = match File::open(&file_name) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to open file {}: {:?}", file_name, e);
                    return;
                }
            };
            let mut rdr = Reader::from_reader(file);

            for result in rdr.deserialize() {
                match result {
                    Ok(tx) => {
                        let tx: AnyTransaction = tx;
                        if self.tx.send(tx).is_err() {
                            break; // Receiver has been dropped
                        }
                    }
                    Err(e) => eprintln!("Failed to deserialize transaction: {:?}", e),
                }
            }

            drop(self.tx);
        });

        Ok(handle)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::mpsc::channel;

    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "type,client,tx,amount").unwrap();
        writeln!(temp_file, "deposit,1,1,1.0").unwrap();
        writeln!(temp_file, "withdrawal,2,2,2.0").unwrap();
        writeln!(temp_file, "deposit,1,3,2.0").unwrap();

        let (tx, rx) = channel();

        let csv_reader = CsvReader::new(tx);
        let handle = csv_reader
            .start(temp_file.path().to_str().unwrap().to_string(), 1)
            .unwrap();

        handle.join().unwrap();

        let transactions: Vec<AnyTransaction> = rx.iter().collect();

        assert_eq!(transactions.len(), 3);

        assert_eq!(transactions[0].command_type, CommandType::Deposit);
        assert_eq!(transactions[0].client_id, 1);
        assert_eq!(transactions[0].tx_id, 1);
        assert_eq!(transactions[0].amount, Some(1.0));

        assert_eq!(transactions[1].command_type, CommandType::Withdrawal);
        assert_eq!(transactions[1].client_id, 2);
        assert_eq!(transactions[1].tx_id, 2);
        assert_eq!(transactions[1].amount, Some(2.0));

        assert_eq!(transactions[2].command_type, CommandType::Deposit);
        assert_eq!(transactions[2].client_id, 1);
        assert_eq!(transactions[2].tx_id, 3);
        assert_eq!(transactions[2].amount, Some(2.0));
    }
}
