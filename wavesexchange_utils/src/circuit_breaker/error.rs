use std::time::Duration;

#[derive(Debug)]
pub enum CBError<E> {
    CircuitBroke { err_count: u16, elapsed: Duration },
    Inner(E),
}
