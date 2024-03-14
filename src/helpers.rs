use std::fmt::Debug;
use std::time::UNIX_EPOCH;

pub(crate) fn log_errors<S, E: Debug>(result: Result<S, E>) {
    if let Err(e) = result {
        println!("{:?}", &e);
    }
}
use crate::collect::FlightDataReadError;

use crate::collect::write_csv;

use std::time::SystemTime;

use std::time::Instant;

use crate::collect::FlightConnector;

use std::io::Write;

pub(crate) fn write_data(stream: &mut impl Write) {
    let mut cont;
    loop {
        match FlightConnector::new("/dev/ttyUSB0") {
            Ok(connector) => {
                cont = connector;
                break;
            }
            _ => {}
        }
    }
    println!("Connected");

    let mut buffer = Vec::new();
    buffer.resize(1024, 0);
    let mut last_update = Instant::now();

    let mut data_point_buffer = Vec::new();
    let mut n = 0;
    loop {
        match cont.read_data() {
            Ok(data) => {
                if last_update.elapsed().as_millis() >= 1000 / 200 {
                    last_update = Instant::now();
                    let time = SystemTime::now();
                    if n % 100 == 0 {
                        println!(
                            "{}, {:?}",
                            time.duration_since(UNIX_EPOCH).unwrap().as_micros(),
                            &data
                        );
                    }

                    write_csv(stream, data).unwrap();
                    data_point_buffer.clear();
                } else {
                    data_point_buffer.push(data);
                }
                n += 1;
            }
            Err(FlightDataReadError::NonFatal) => {
                println!("non fatal")
            }
            Err(FlightDataReadError::Fatal(e)) => {
                println!("Fatal error: {:?}", e);
                return;
            }
        }
    }
}