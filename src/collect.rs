use std::{error::Error, io::{self, BufRead, BufReader, Write}, time::Duration};
use std::fmt::Debug;

use crate::helpers::log_errors;

pub struct FlightConnector {
    port: BufReader<serialport::TTYPort>,
}

impl FlightConnector {
    pub fn new<'a>(port: impl Into<std::borrow::Cow<'a, str>>) -> io::Result<FlightConnector> {
        let mut port = serialport::new(port, 115_200).open_native()?;
        use crate::serialport::SerialPort;

        log_errors(port.set_timeout(Duration::new(5, 0)));

        Ok(FlightConnector {
            port: BufReader::new(port), // port: serialport::new(port, 115_200).open_native().expect("Fuck"),
        })
    }

    pub fn read_data(&mut self) -> Result<RawFlightData, FlightDataReadError> {
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

pub struct RawFlightData {
    pub ac_x: i16,
    pub ac_y: i16,
    pub ac_z: i16,
    pub gy_x: i16,
    pub gy_y: i16,
    pub gy_z: i16,
    pub dt: u64,
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

pub enum FlightDataReadError {
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

fn raw_to_rot_degrees(data: i16) -> f32 {
    (data as f32 / i16::MAX as f32) * 500.0
}

fn raw_to_g(data: i16) -> f32 {
    data as f32 / 2048.0
}

#[allow(unused)]
pub fn write_csv(stream: &mut impl Write, data: RawFlightData) -> io::Result<()> {
    writeln!(
        stream,
        "{},{},{},{},{},{},{}",
        data.dt,
        raw_to_g(data.ac_x),
        raw_to_g(data.ac_y),
        raw_to_g(data.ac_z),
        raw_to_rot_degrees(data.gy_x),
        raw_to_rot_degrees(data.gy_y),
        raw_to_rot_degrees(data.gy_z)
    )?;
    Ok(())
}