use clap::Parser;

use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_postgres::{Client, Error, NoTls};
use uuid::Uuid;

use log::debug;

type DbClient = Arc<Mutex<Client>>;

async fn query(db: DbClient) -> Result<(), Error> {
    let db = db.lock().unwrap();
    let rows = db.query("SELECT * FROM health_check", &[]).await?;

    // let value: &str = rows[0].get(0);
    // assert_eq!(value, "hello world");

    Ok(())
}

async fn hello(
    _: Request<hyper::body::Incoming>,
    db: DbClient,
) -> Result<Response<Full<Bytes>>, Infallible> {
    query(db).await.unwrap();

    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Verbose level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Bind port
    #[arg(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    env_logger::init();

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    debug!("Listening on: {}", addr);

    let database_dsn =
        std::env::var("DATABASE_DSN").expect("Failed to parse DATABASE_DSN environment variable");

    let (client, connection) = tokio_postgres::connect(&database_dsn, NoTls).await?;

    let client = Arc::new(Mutex::new(client));

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let client = client.clone();

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |_req| {
                            let client = Arc::clone(&client);
                            async { hello(_req, client).await }
                        }),
                    )
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest(
        args,
        case::just_port(vec!["", "-p", "3000"]),
    )]
    fn test_config_for_any_order(args: Vec<&str>) {
        assert!(Cli::try_parse_from(args).is_ok());
    }

    #[fixture]
    async fn db() -> DbClient {
        let database_dsn = std::env::var("DATABASE_DSN")
            .expect("Failed to parse DATABASE_DSN environment variable");

        let (client, connection) = tokio_postgres::connect(&database_dsn, NoTls).await.unwrap();

        let client = Arc::new(Mutex::new(client));

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        client
    }

    #[fixture]
    async fn basic_health_check(#[future] db: DbClient) -> Result<Uuid, Error> {
        let db_awaited = db.await;
        let db = db_awaited.lock().unwrap();
        let rows = db
            .query("INSERT INTO health_check
            (organization_id, name, url, expected_status_code) VALUES ($1, $2, $3, $4) RETURNING (id)", &[&Uuid::new_v4(), &"test", &"http://...", &200])
            .await?;
        assert_eq!(rows.len(), 1);
        let id_str: Uuid = rows[0].get(0);
        Ok(id_str)
    }

    #[tokio::main]
    #[rstest]
    async fn test_query_health_check(
        #[future] db: DbClient,
        #[future] basic_health_check: Result<Uuid, Error>,
    ) -> Result<(), Error> {
        env_logger::init();

        let db_awaited = db.await;
        let health_check_id = basic_health_check.await?;
        let db = db_awaited.lock().unwrap();
        let rows = db
            .query(
                "SELECT * FROM health_check WHERE id=$1",
                &[&health_check_id],
            )
            .await?;
        assert_eq!(rows.len(), 1);
        let value: &str = rows[0].try_get("name").expect("Failed");
        debug!("Hellovalue {}", value);
        Ok(())
    }
}
