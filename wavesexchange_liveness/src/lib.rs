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

const LAST_BLOCK_TIMESTAMP_QUERY: &str = "SELECT time_stamp FROM blocks_microblocks WHERE time_stamp IS NOT NULL ORDER BY uid DESC LIMIT 1";

struct LastBlock {
    timestamp: i64,
    last_change: Instant,
}

pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub poolsize: u32,
}

#[derive(QueryableByName)]
struct LastBlockTimestamp {
    #[sql_type = "BigInt"]
    time_stamp: i64,
}

fn get_conn(pgconfig: &PostgresConfig) -> Result<PgConnection, diesel::result::ConnectionError> {
    let db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        pgconfig.user, pgconfig.password, pgconfig.host, pgconfig.port, pgconfig.database
    );
    PgConnection::establish(&db_url)
}

pub fn channel(
    pgconfig: PostgresConfig,
    poll_interval_secs: u64,
    max_block_age: Duration,
) -> UnboundedReceiver<Readiness> {
    let pgconfig: PostgresConfig = pgconfig.into();
    let (readiness_tx, readiness_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut last_block = LastBlock {
        timestamp: 0,
        last_change: Instant::now(),
    };

    tokio::spawn(async move {
        loop {
            let send = |status: Readiness| {
                if readiness_tx.send(status).is_err() {
                    error!("Failed to send {:?} status", status);
                }
            };

            tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)).await;

            match get_conn(&pgconfig) {
                Ok(conn) => {
                    let query_result = diesel::sql_query(LAST_BLOCK_TIMESTAMP_QUERY)
                        .load::<LastBlockTimestamp>(&conn)
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
