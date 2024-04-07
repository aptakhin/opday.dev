use clap::Parser;


use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_postgres::{Client, Error, NoTls};
use uuid::Uuid;

use log::debug;

use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
// use serde::{Deserialize, Serialize}; // Add this line
use warp::{
    http::{Response, StatusCode},
    Filter,
};

#[derive(Deserialize, Serialize)]
struct MyObject {
    key1: String,
    key2: u32,
}

type DbClient = Arc<Mutex<Client>>;

async fn query(db: DbClient) -> Result<(), Error> {
    let db = db.lock().unwrap();
    let rows = db.query("SELECT * FROM health_check", &[]).await?;

    let value: &str = rows[0].get("name");

    Ok(())
}

// async fn hello(
//     req: Request<hyper::body::Incoming>,
//     db: DbClient,
// ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> {
//     // query(db).await.unwrap();

//     debug!("Query {}", req.uri().path());

//     match req.uri().path() {
//         // "/health" => {
//         //     let response = Response<BoxBody<Bytes, hyper::Error>>::builder()
//         //         .status(200)
//         //         .body(())
//         //         .unwrap();

//         //     Ok(response)
//         // }
//         _ => {
//             let response = Response<BoxBody<Bytes, hyper::Error>>::builder()
//                 .status(404)
//                 .body(())
//                 .unwrap();

//             Ok(response)
//         }
//     }

    // Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))

    // let response = Response::builder()
    //     .status(200)
    //     .body(())
    //     .unwrap();

    // Ok(response)
// }

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
async fn main() {
    let cli = Cli::parse();

    env_logger::init();

    // let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    // debug!("Listening on: {}", addr);

    let database_dsn =
        std::env::var("DATABASE_DSN").expect("Failed to parse DATABASE_DSN environment variable");

    let Ok((client, connection)) = tokio_postgres::connect(&database_dsn, NoTls).await else { todo!() };

    let client = Arc::new(Mutex::new(client));

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let example1 = warp::get()
        .and(warp::path("example1"))
        .and(warp::query::<HashMap<String, String>>())
        .map(|p: HashMap<String, String>| match p.get("key") {
            Some(key) => Response::builder().body(format!("key = {}", key)),
            None => Response::builder().body(String::from("No \"key\" param in query.")),
        });

    let example2 = warp::get()
        .and(warp::path("example2"))
        .and(warp::query::<MyObject>())
        .map(|p: MyObject| {
            Response::builder().body(format!("key1 = {}, key2 = {}", p.key1, p.key2))
        });

    warp::serve(example1.or(example2))
        .run(([127, 0, 0, 1], cli.port))
        .await

    // let listener = TcpListener::bind(addr).await?;

    // loop {
    //     let (stream, _) = listener.accept().await?;

    //     let io = TokioIo::new(stream);

    //     let client = client.clone();

    //     tokio::task::spawn_blocking(move || {
    //         tokio::runtime::Handle::current().block_on(async {
    //             if let Err(err) = http1::Builder::new()
    //                 .serve_connection(
    //                     io,
    //                     service_fn(move |_req| {
    //                         let client = Arc::clone(&client);
    //                         async { hello(_req, client).await }
    //                     }),
    //                 )
    //                 .await
    //             {
    //                 println!("Error serving connection: {:?}", err);
    //             }
    //         })
    //     });
    // }
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
