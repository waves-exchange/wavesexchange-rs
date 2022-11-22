
use super::*;
use deadpool_diesel::{Manager, Pool};
use diesel::pg::PgConnection;
use diesel::result::Error as DslError;

pub struct DeadpoolPgBreaker(Pool<Manager<PgConnection>>);

impl FallibleDataSource for DeadpoolPgBreaker {
    const REINIT_ON_FAIL: bool = true;
    type Error = DslError;

    fn is_countable_err(err: &Self::Error) -> bool {
        err.to_string().contains("no connection to the server")
    }
}
