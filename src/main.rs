#![feature(async_closure)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use actix_web::http::StatusCode;
use actix_web::{body, middleware, web, App, HttpResponse, HttpServer};
use bb8::Pool;
use bb8_postgres::{tokio_postgres, PostgresConnectionManager};
use std::io;

async fn greeting(
    db: web::Data<Pool<PostgresConnectionManager<tokio_postgres::NoTls>>>,
) -> Result<HttpResponse, !> {
    let mut res: i32 = 0;
    match db
        .get_ref()
        .run(
            async move |connection| match connection.prepare("select 1").await {
                Ok(select) => match connection.query_one(&select, &[]).await {
                    Ok(row) => {
                        res = row.get::<usize, i32>(0);
                        Ok(((), connection))
                    }
                    Err(e) => Err((e, connection)),
                },
                Err(e) => Err((e, connection)),
            },
        )
        .await
    {
        Ok(_) => Ok(HttpResponse::with_body(
            StatusCode::OK,
            body::Body::from(format!("{}", res)),
        )),
        Err(_) => Ok(HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    const DB_URL: &str = "postgres://postgres:docker@localhost/postgres";
    let pg_mgr =
        PostgresConnectionManager::new_from_stringlike(DB_URL, tokio_postgres::NoTls)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let pool = Pool::builder()
        .build(pg_mgr)
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::Other))?;

    println!("Started http server: 127.0.0.1:8000");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::DefaultHeaders::new().header("X-Version", "0.2"))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .service(web::resource("/greeting").to(greeting))
    })
    .bind("127.0.0.1:8000")?
    .workers(1)
    .run()
}
