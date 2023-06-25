use std::sync::{Arc, Mutex};

pub fn init_throttle(
) -> rusqlite::Result<Arc<Mutex<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>, rusqlite::Error>
{
    log::info!("Open a throttle DB connection");
    let throttle_db_path = String::from("throttle.db");
    let conn_manager: r2d2_sqlite::SqliteConnectionManager =
        r2d2_sqlite::SqliteConnectionManager::file(throttle_db_path);
    let conn_pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager> =
        r2d2::Pool::new(conn_manager).expect("Failed to create a DB connection pool");
    let conn_pool_am: Arc<Mutex<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>> =
        Arc::new(Mutex::new(conn_pool));
    let conn: r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager> =
        conn_pool_am.lock().unwrap().get().unwrap();

    log::info!("Exceute DDL, DML queries for throttling");
    conn.execute_batch(
        "DROP TABLE if exists visitor_limits;
        DROP TABLE if exists visitors;
        DROP TABLE if exists limits;",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS visitors (
            id INTEGER PRIMARY KEY ASC
            , name TEXT NOT NULL
            , created_at INTEGER CHECK ( created_at > 1680373723 )
            , updated_at INTEGER CHECK ( updated_at > 1680373723 )
            ) STRICT;
        CREATE TABLE IF NOT EXISTS limits (
            id INTEGER PRIMARY KEY ASC
            , name TEXT UNIQUE NOT NULL
            , maximum INTEGER NOT NULL CHECK ( maximum > 0 )
            , period_minutes INTEGER NOT NULL CHECK ( period_minutes >= 1 )
            , UNIQUE (name, maximum)
            ) STRICT;
        CREATE TABLE IF NOT EXISTS visitor_limits (
            id INTEGER PRIMARY KEY ASC
            , visitor_id INTEGER NOT NULL REFERENCES visitors(id)
            , limit_id INTEGER NOT NULL REFERENCES limits(id)
            , denied INTEGER NOT NULL DEFAULT 0 CHECK ( denied IN (0, 1) )
            -- set by trigger
            , count INTEGER CHECK ( count >= 0 )
            , resets_at INTEGER CHECK ( resets_at > 1680373723 )
            , UNIQUE (visitor_id, limit_id)
            ) STRICT;",
    )?;
    conn.execute(
        "INSERT INTO limits (name, maximum, period_minutes)
        VALUES ('send_message', 5, 2);",
        (),
    )?; // allow 5 messages per 2 minutes
    conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS init_visitor_limit_counter
            AFTER INSERT ON visitor_limits
                WHEN (NEW.count IS NULL OR NEW.resets_at IS NULL)
            BEGIN
                UPDATE visitor_limits
                SET count = 1
                    , resets_at = (unixepoch() + (60 * (SELECT period_minutes FROM limits l WHERE l.id = NEW.limit_id)))
                    , denied = 0
                WHERE id = NEW.id;
            END;
        CREATE TRIGGER IF NOT EXISTS update_visitor_limit_counter_breaker
            BEFORE INSERT ON visitor_limits
            BEGIN
                -- trip breaker for non-expired
                UPDATE visitor_limits
                SET denied = visitor_limits.count >= (SELECT maximum FROM limits l WHERE l.id = NEW.limit_id)
                WHERE visitor_limits.visitor_id = NEW.visitor_id
                    AND visitor_limits.limit_id = NEW.limit_id;
                -- or reset if expired
                UPDATE visitor_limits
                SET count = 0
                    , resets_at = unixepoch() + (60 * (SELECT period_minutes FROM limits l WHERE l.id = NEW.limit_id))
                    , denied = 0
                WHERE visitor_limits.visitor_id = NEW.visitor_id
                    AND visitor_limits.limit_id = NEW.limit_id
                    AND unixepoch() > visitor_limits.resets_at;
            END;"
    )?;
    conn.execute_batch(
        "insert into visitors(name, created_at)
        values ('my', STRFTIME('%s'));",
    )?;

    Ok(conn_pool_am)
}

pub fn run(
    throttle_conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    headers: &hyper::HeaderMap,
) -> Result<http::StatusCode, rusqlite::Error> {
    let api_key: Option<&http::HeaderValue> = headers.get("api-key");
    if api_key == None {
        log::error!("api-key in header is required");
        return Ok(http::StatusCode::BAD_REQUEST);
    }
    let api_key: &str = api_key.unwrap().to_str().unwrap();
    log::info!("api-key = {:?}", api_key);

    let has_visitor: isize = throttle_conn
        .query_row(
            "SELECT exists (SELECT 1 FROM visitors WHERE id = ?1)",
            [api_key],
            |row| row.get(0),
        )
        .unwrap();
    if has_visitor == 0 {
        log::error!("Thers is no such visitor: {}", api_key);
        return Ok(http::StatusCode::UNAUTHORIZED);
    }
    log::info!("has_visitor = {}", has_visitor);

    if let Err(err) = throttle_conn.execute(
        "INSERT INTO visitor_limits (visitor_id, limit_id)
        VALUES (?1, 1)
        ON CONFLICT (visitor_id, limit_id) DO UPDATE SET count = count + 1;",
        &[api_key],
    ) {
        log::error!("{}", err);
        return Err(err);
    }

    let denied: isize = throttle_conn
        .query_row(
            "SELECT denied FROM visitor_limits WHERE visitor_id = ?1",
            [api_key],
            |row| row.get(0),
        )
        .unwrap();

    if denied == 1 {
        log::error!("denied = {}", denied);
        return Ok(http::StatusCode::TOO_MANY_REQUESTS);
    }
    log::info!("denied = {}", denied);

    Ok(http::StatusCode::OK)
}
