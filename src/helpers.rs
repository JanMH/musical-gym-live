use std::fmt::Debug;

pub(crate) fn log_errors<S, E: Debug>(result: Result<S, E>) {
    if let Err(e) = result {
        println!("{:?}", &e);
    }
}
