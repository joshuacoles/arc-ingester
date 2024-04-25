use sqlx::{Connection, Executor};
use testcontainers::core::env::Os;

#[tokio::main]
async fn main() {
    let tc = testcontainers::clients::Cli::new::<Os>();
    let pg_spec = testcontainers_modules::postgres::Postgres::default();
    let pg_container = tc.run(pg_spec);
    pg_container.start();
    println!("postgres running");
    let mut pg = sqlx::postgres::PgConnection::connect(&format!("postgres://postgres:postgres@localhost:{}/postgres", pg_container.get_host_port_ipv4(5432)),)
        .await
        .unwrap();

    dbg!(pg.execute("select 1").await.unwrap());
}
