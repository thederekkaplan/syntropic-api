use crate::models::message::{Message, MessageLoader};
use actix_web::web::{resource, Data, ServiceConfig};
use actix_web::{web, Error, HttpResponse};
use deadpool::managed::Manager as _;
use deadpool_diesel::postgres::Manager;
use deadpool_diesel::postgres::Pool;
use deadpool_diesel::Runtime;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use graphql::{Mutation, Query};
use juniper::{EmptySubscription, RootNode};
use juniper_actix::{graphiql_handler, graphql_handler, playground_handler};

mod graphql;
mod models;
mod schema;
mod snowflake;

const DB_URL: &str = "postgres://postgres:password@localhost/syntropic";
const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type Schema = RootNode<'static, Query, Mutation, EmptySubscription<PostgresContext>>;

pub async fn run_migrations() {
    let manager = Manager::new(DB_URL, Runtime::Tokio1);
    let client = manager.create().await.unwrap();
    client
        .interact(|client| {
            client.run_pending_migrations(MIGRATIONS).unwrap();
        })
        .await
        .unwrap();
}

fn pool() -> Pool {
    let manager = Manager::new(DB_URL, Runtime::Tokio1);
    Pool::builder(manager).build().unwrap()
}

pub struct PostgresContext {
    pool: Pool,
    message_loader: MessageLoader,
}

impl juniper::Context for PostgresContext {}

fn schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}

async fn graphiql_route() -> Result<HttpResponse, Error> {
    graphiql_handler("/graphql", None).await
}

async fn playground_route() -> Result<HttpResponse, Error> {
    playground_handler("/graphql", None).await
}

async fn graphql_route(
    req: actix_web::HttpRequest,
    payload: web::Payload,
    data: Data<(Schema, Pool)>,
) -> Result<HttpResponse, Error> {
    let context = PostgresContext {
        pool: data.get_ref().1.clone(),
        message_loader: Message::loader(data.get_ref().1.clone()),
    };

    graphql_handler(&data.get_ref().0, &context, req, payload).await
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.app_data(Data::new((schema(), pool())))
        .service(
            resource("/graphql")
                .route(web::post().to(graphql_route))
                .route(web::get().to(graphql_route)),
        )
        .service(resource("/graphiql").route(web::get().to(graphiql_route)))
        .service(resource("/playground").route(web::get().to(playground_route)));
}
