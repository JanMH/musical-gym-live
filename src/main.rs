use std::env::args;
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};
extern crate serialport;

struct FlightConnector {
    port: BufReader<serialport::TTYPort>,
}

impl FlightConnector {
    fn new<'a>(port: impl Into<std::borrow::Cow<'a, str>>) -> io::Result<FlightConnector> {
        let mut port = serialport::new(port, 115_200).open_native()?;
        use crate::serialport::SerialPort;

        log_errors(port.set_timeout(Duration::new(5, 0)));

        Ok(FlightConnector {
            port: BufReader::new(port), // port: serialport::new(port, 115_200).open_native().expect("Fuck"),
        })
    }

    fn read_data(&mut self) -> Result<RawFlightData, FlightDataReadError> {
        let mut line = String::new();
        self.port.read_line(&mut line)?;
        if line.starts_with("DBG") {
            println!("{}", &line[3..]);
            return Err(FlightDataReadError::NonFatal);
        }
        let split_line: Vec<_> = line.split(", ").collect();
        let values: Vec<Result<i16, _>> = split_line
            .iter()
            .take(6)
            .map(|value| value.trim().parse())
            .collect();
        if values.iter().any(|r| r.is_err()) {
            return Err(FlightDataReadError::NonFatal);
        } else if split_line.len() != 7 {
            return Err(FlightDataReadError::NonFatal);
        } else {
            let mut values = values.into_iter().map(|v| v.unwrap());

            return Ok(RawFlightData {
                ac_x: values.next().unwrap(),
                ac_y: values.next().unwrap(),
                ac_z: values.next().unwrap(),
                gy_x: values.next().unwrap(),
                gy_y: values.next().unwrap(),
                gy_z: values.next().unwrap(),
                dt: split_line[6].trim().parse().unwrap(),
            });
        }
    }
}

struct RawFlightData {
    ac_x: i16,
    ac_y: i16,
    ac_z: i16,
    gy_x: i16,
    gy_y: i16,
    gy_z: i16,
    dt: u64,
}
impl Debug for RawFlightData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("acc_x {:>+3.2} ", raw_to_g(self.ac_x)))?;
        f.write_fmt(format_args!("acc_y {:>+3.2} ", raw_to_g(self.ac_y)))?;
        f.write_fmt(format_args!("acc_z {:>+3.2} ", raw_to_g(self.ac_z)))?;
        f.write_fmt(format_args!("gy_x {:3.1} ", raw_to_rot_degrees(self.gy_x)))?;
        f.write_fmt(format_args!("gy_y {:3.1} ", raw_to_rot_degrees(self.gy_y)))?;
        f.write_fmt(format_args!("gy_z {:3.1} ", raw_to_rot_degrees(self.gy_z)))
    }
}
impl RawFlightData {
    fn as_buffer(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.reserve(13);
        result.push(0xff);
        result.push((self.ac_x & 0xff) as u8);
        result.push((self.ac_x >> 8) as u8);
        result.push((self.ac_y & 0xff) as u8);
        result.push((self.ac_y >> 8) as u8);
        result.push((self.ac_z & 0xff) as u8);
        result.push((self.ac_z >> 8) as u8);
        result.push((self.gy_x & 0xff) as u8);
        result.push((self.gy_x >> 8) as u8);
        result.push((self.gy_y & 0xff) as u8);
        result.push((self.gy_y >> 8) as u8);
        result.push((self.gy_z & 0xff) as u8);
        result.push((self.gy_z >> 8) as u8);
        let mut dt_pushed = self.dt;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        dt_pushed >>= 8;
        result.push((dt_pushed & 0xff) as u8);
        result
    }

    fn average(data: &[RawFlightData]) -> RawFlightData {
        let mut ac_x: i64 = 0;
        let mut ac_y: i64 = 0;
        let mut ac_z: i64 = 0;
        let mut gy_x: i64 = 0;
        let mut gy_y: i64 = 0;
        let mut gy_z: i64 = 0;
        let mut dt: u64 = 0;
        for point in data {
            ac_x += point.ac_x as i64;
            ac_y += point.ac_y as i64;
            ac_z += point.ac_z as i64;
            gy_x += point.gy_x as i64;
            gy_y += point.gy_y as i64;
            gy_z += point.gy_z as i64;
            dt += point.dt;
        }

        RawFlightData {
            ac_x: (ac_x / data.len() as i64) as i16,
            ac_y: (ac_y / data.len() as i64) as i16,
            ac_z: (ac_z / data.len() as i64) as i16,
            gy_x: (gy_x / data.len() as i64) as i16,
            gy_y: (gy_y / data.len() as i64) as i16,
            gy_z: (gy_z / data.len() as i64) as i16,
            dt,
        }
    }
}

enum FlightDataReadError {
    Fatal(Box<dyn Error>),
    NonFatal,
}

impl From<std::io::Error> for FlightDataReadError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::TimedOut => FlightDataReadError::NonFatal,
            _ => FlightDataReadError::Fatal(Box::new(e)),
        }
    }
}

fn log_errors<S, E: Debug>(result: Result<S, E>) {
    if let Err(e) = result {
        println!("{:?}", &e);
    }
}

fn raw_to_rot_degrees(data: i16) -> f32 {
    (data as f32 / i16::MAX as f32) * 500.0
}

fn raw_to_g(data: i16) -> f32 {
    data as f32 / 2048.0
}

fn write_csv(stream: &mut impl Write, time: SystemTime, data: RawFlightData) -> io::Result<()> {
    write!(
        stream,
        "{},",
        time.duration_since(UNIX_EPOCH).unwrap().as_micros()
    )?;
    writeln!(
        stream,
        "{},{},{},{},{},{}",
        raw_to_g(data.ac_x),
        raw_to_g(data.ac_y),
        raw_to_g(data.ac_z),
        raw_to_rot_degrees(data.gy_x),
        raw_to_rot_degrees(data.gy_y),
        raw_to_rot_degrees(data.gy_z)
    )?;
    Ok(())
}

fn write_data(stream: &mut impl Write) {
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

                    write_csv(stream, time, data).unwrap();
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

fn main() -> std::io::Result<()> {
    let path = args().nth(1).expect("Please pass file to write to");
    let mut file = File::create(path).expect("Could not pen file for writing");
    write_data(&mut file);
    Ok(())
}
