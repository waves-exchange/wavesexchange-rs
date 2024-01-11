use diesel::sql_types::BigInt;
use diesel::Connection;
use diesel::PgConnection;
use diesel::QueryableByName;
use diesel::RunQueryDsl;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use wavesexchange_log::{debug, error};
use wavesexchange_warp::endpoints::Readiness;

const LAST_BLOCK_TIMESTAMP_QUERY: &str = "SELECT time_stamp FROM blocks_microblocks WHERE time_stamp IS NOT NULL AND time_stamp != 0 ORDER BY uid DESC LIMIT 1";

struct LastBlock {
    timestamp: i64,
    last_change: Instant,
}

#[derive(QueryableByName)]
struct LastBlockTimestamp {
    #[diesel(sql_type = BigInt)]
    time_stamp: i64,
}

pub fn channel(
    db_url: String,
    poll_interval_secs: u64,
    max_block_age: Duration,
    custom_query: Option<String>,
) -> UnboundedReceiver<Readiness> {
    let (readiness_tx, readiness_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut last_block = LastBlock {
        timestamp: 0,
        last_change: Instant::now(),
    };
    let query = custom_query.unwrap_or(LAST_BLOCK_TIMESTAMP_QUERY.to_string());

    tokio::spawn(async move {
        loop {
            let send = |status: Readiness| {
                if readiness_tx.send(status).is_err() {
                    error!("Failed to send {:?} status", status);
                }
            };

            tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)).await;

            match PgConnection::establish(&db_url) {
                Ok(mut conn) => {
                    let query_result = diesel::sql_query(&query)
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
