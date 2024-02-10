//! Liveness probe based on periodic Postgres query check

#[cfg(all(feature = "diesel1", not(feature = "diesel2")))]
extern crate diesel1 as diesel; // Diesel 1.x

#[cfg(all(feature = "diesel2", not(feature = "diesel1")))]
extern crate diesel2 as diesel; // Diesel 2.x

#[rustfmt::skip]
#[cfg(any(all(feature = "diesel1", feature = "diesel2"), not(any(feature = "diesel1", feature = "diesel2"))))]
compile_error!("Either feature \"diesel1\" or \"diesel2\" must be enabled for this crate, but not both.");

use diesel::{
    sql_query, sql_types::BigInt, Connection, PgConnection, QueryableByName, RunQueryDsl,
};
use std::time::{Duration, Instant};
use tokio::{sync::mpsc, task, time};
use wavesexchange_log::{debug, error};
use wavesexchange_warp::endpoints::Readiness;

const LAST_BLOCK_TIMESTAMP_QUERY: &str = "SELECT time_stamp FROM blocks_microblocks WHERE time_stamp IS NOT NULL AND time_stamp != 0 ORDER BY uid DESC LIMIT 1";

struct LastBlock {
    timestamp: i64,
    last_change: Instant,
}

#[derive(QueryableByName)]
struct LastBlockTimestamp {
    #[cfg_attr(feature = "diesel1", sql_type = "BigInt")] // for Diesel 1.x
    #[cfg_attr(feature = "diesel2", diesel(sql_type = BigInt))] // for Diesel 2.x
    time_stamp: i64,
}

pub fn channel(
    db_url: String,
    poll_interval_secs: u64,
    max_block_age: Duration,
    custom_query: Option<String>,
) -> mpsc::UnboundedReceiver<Readiness> {
    let (readiness_tx, readiness_rx) = mpsc::unbounded_channel();

    let mut last_block = LastBlock {
        timestamp: 0,
        last_change: Instant::now(),
    };
    let query = custom_query.unwrap_or(LAST_BLOCK_TIMESTAMP_QUERY.to_string());

    task::spawn(async move {
        loop {
            let send = |status: Readiness| {
                if readiness_tx.send(status).is_err() {
                    error!("Failed to send {:?} status", status);
                }
            };

            time::sleep(Duration::from_secs(poll_interval_secs)).await;

            match PgConnection::establish(&db_url) {
                Ok(mut conn) => {
                    let query_result = sql_query(&query)
                        .load::<LastBlockTimestamp>(&mut conn)
                        .map(|results| results.into_iter().next().map(|result| result.time_stamp));

                    match query_result {
                        Ok(last_block_timestamp) => {
                            if let Some(timestamp) = last_block_timestamp {
                                debug!("Current timestamp: {}", timestamp);
                                let now = Instant::now();
                                if timestamp > last_block.timestamp {
                                    last_block.timestamp = timestamp;
                                    last_block.last_change = now;
                                    debug!("Sending status: Ready");
                                    send(Readiness::Ready);
                                } else {
                                    if now.duration_since(last_block.last_change) > max_block_age {
                                        debug!("Sending status: Dead");
                                        send(Readiness::Dead);
                                    } else {
                                        debug!("Sending status: Ready");
                                        send(Readiness::Ready);
                                    }
                                }
                            } else {
                                error!("Could not get last block timestamp");
                                debug!("Sending status: Ready");
                                send(Readiness::Ready);
                            }
                        }
                        Err(err) => {
                            error!("Error while fetching last block timestamp: {}", err);
                            debug!("Sending status: Dead");
                            send(Readiness::Dead);
                        }
                    }
                }
                Err(err) => {
                    error!("Error establishing database connection: {}", err);
                }
            }
        }
    });

    readiness_rx
}
