#![feature(async_closure)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use actix_web::http::StatusCode;
use actix_web::{body, middleware, web, App, HttpResponse, HttpServer};
use bb8::Pool;
use bb8_postgres::{tokio_postgres, PostgresConnectionManager};
use futures::future::lazy;

async fn selector(
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
                        Ok((res, connection))
                    }
                    Err(e) => Err((e, connection)),
                },
                Err(e) => Err((e, connection)),
            },
        )
        .await
    {
        Ok(res) => Ok(HttpResponse::with_body(
            StatusCode::OK,
            body::Body::from(format!("{}", res)),
        )),
        Err(e) => {
            println!("{}", e);
            Ok(HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR))
        },
    }
}

async fn create_pool(
    runner: &mut actix_rt::SystemRunner,
    db_url: &str
) -> Pool<PostgresConnectionManager<tokio_postgres::NoTls>> {
    let pg_mgr = PostgresConnectionManager::new_from_stringlike(db_url, tokio_postgres::NoTls).unwrap();
    runner.block_on(lazy(|_| Pool::builder().build(pg_mgr))).await.unwrap()
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let mut sys = actix_rt::System::new("greeter");
    let pool = futures::executor::block_on(create_pool(&mut sys, "postgres://postgres:docker@localhost/postgres"));

    println!("Started http server: 127.0.0.1:8000");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .service(web::resource("/select1").to(selector))
    })
    .bind("127.0.0.1:8000")?
    .workers(1)
    .start();

    sys.run()
}
