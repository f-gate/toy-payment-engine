
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::time::Duration;
use eyre::Result;
use state::*;

use crate::command::AnyTransaction;

struct Reader {
    tx: Sender<AnyTransaction>,
}

impl Reader {
    pub fn new(tx: Sender<AnyTransaction>) {
        Self {tx}
    }

    pub fn start(self, file_name: &str, thread_count: u8) ->  Result<()> {
        thread::spawn(move || {
            let file = File::open(file_name).map_err(eyre!(e))?;
            let mut rdr = Reader::from_reader(file);
    
            for result in rdr.deserialize() {
                let tx: AnyTransaction = result.map_err(eyre!(e));
                self.tx.send(tx)?;
            }

            Ok(())
        })
    }
}
