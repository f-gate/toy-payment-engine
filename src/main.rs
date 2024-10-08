mod account;
mod handlers;
mod transaction;
mod types;
mod validated_transaction;

use handlers::*;
use transaction::*;

use csv::Writer;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

use eyre::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <transactions.csv>");
        return Ok(());
    }
    let input_filename = args[1].clone();

    process_transactions(input_filename, None)
}

/// Pass None into output_filename to write to std-out
pub fn process_transactions(input_filename: String, output_filename: Option<String>) -> Result<()> {
    let (tx_any_tx, rx_any_tx): (Sender<AnyTransaction>, Receiver<AnyTransaction>) = channel();
    let (tx_tx_command, rx_tx_command): (Sender<TransactionCommand>, Receiver<TransactionCommand>) =
        channel();

    let csv_reader = CsvReader::new(tx_any_tx.clone());
    let csv_reader_handle = csv_reader.start(input_filename.clone(), 1)?;

    let command_converter = CommandConverter::new(rx_any_tx, tx_tx_command.clone());
    let command_converter_handle = command_converter.start();

    let account_manager = AccountManager::new(rx_tx_command);
    let account_manager_handle = account_manager.start();

    drop(tx_any_tx);
    drop(tx_tx_command);

    // TODO: Need to at least eprintln the errors
    csv_reader_handle.join().unwrap();
    command_converter_handle.join().unwrap();
    let accounts = account_manager_handle.join().unwrap();

    let output_file: Box<dyn Write> = match output_filename {
        Some(ref s) => Box::new(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(s)?,
        ),
        None => Box::new(std::io::stdout()),
    };

    let mut wtr = Writer::from_writer(output_file);
    wtr.write_record(["client", "available", "held", "total", "locked"])?;

    for (client_id, account) in accounts {
        wtr.write_record(&[
            client_id.to_string(),
            // TODO: Better decimal handling using decimal crate.
            format!("{:.4}", account.available),
            format!("{:.4}", account.held),
            format!("{:.4}", account.total()),
            if account.locked.is_some() {
                "true".to_string()
            } else {
                "false".to_string()
            },
        ])?;
    }

    wtr.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Result;
    use std::fs::read_to_string;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_process_transactions() -> Result<()> {
        let mut temp_input = NamedTempFile::new().unwrap();
        writeln!(temp_input, "type,client,tx,amount")?;
        writeln!(temp_input, "deposit,1,1,10.0")?;
        writeln!(temp_input, "deposit,2,2,5.0")?;
        writeln!(temp_input, "deposit,1,3,5.0")?;
        writeln!(temp_input, "withdrawal,1,4,3.0")?;
        writeln!(temp_input, "dispute,1,1,")?;
        writeln!(temp_input, "resolve,1,1,")?;
        writeln!(temp_input, "dispute,1,3,")?;
        writeln!(temp_input, "chargeback,1,3,")?;

        let temp_output = NamedTempFile::new().unwrap();

        process_transactions(
            temp_input.path().to_str().unwrap().to_string(),
            Some(temp_output.path().to_str().unwrap().to_string()),
        )?;

        let output_content = read_to_string(temp_output.path())?;

        let expected_output = "\
client,available,held,total,locked\n\
1,7.0000,0.0000,7.0000,true\n\
2,5.0000,0.0000,5.0000,false\n";

        assert_eq!(output_content, expected_output);

        Ok(())
    }
}
