use std::env::args;


use std::fs::File;

use collect::write_csv;

extern crate serialport;

mod helpers;
mod collect;



fn main() -> std::io::Result<()> {
    let path = args().nth(1).expect("Please pass file to write to");
    let mut file = File::create(path).expect("Could not pen file for writing");
    helpers::run_model(move |data| { write_csv(&mut file, data); } );
    Ok(())
}
