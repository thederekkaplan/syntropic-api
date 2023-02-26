use crate::amqp::AmqpClient;
use crate::graphql::Subscription;
use crate::models::message::{Message, MessageLoader};
use actix_web::web::{resource, Data, ServiceConfig};
use actix_web::{web, Error, HttpResponse};
use deadpool::managed::Manager as _;
use deadpool_diesel::postgres::Manager;
use deadpool_diesel::postgres::Pool;
use deadpool_diesel::Runtime;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use graphql::{Mutation, Query};
use juniper::RootNode;
use juniper_actix::subscriptions::subscriptions_handler;
use juniper_actix::{graphiql_handler, graphql_handler, playground_handler};
use juniper_graphql_ws::ConnectionConfig;
use std::env::var;
use std::sync::Arc;

mod protos {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
    pub use message::Message;
}

pub mod amqp;
pub mod graphql;
pub mod models;
mod schema;
mod snowflake;

fn db_url() -> String {
    var("DATABASE_URL")
        .unwrap_or("postgres://postgres:password@localhost:5432/syntropic".to_string())
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type Schema = RootNode<'static, Query, Mutation, Subscription>;

type AppData = Data<(Pool, AmqpClient)>;

pub async fn data() -> AppData {
    let manager = Manager::new(db_url(), Runtime::Tokio1);
    let client = manager.create().await.unwrap();
    client
        .interact(|client| {
            client.run_pending_migrations(MIGRATIONS).unwrap();
        })
        .await
        .unwrap();
    let pool = Pool::builder(manager).build().unwrap();
    let amqp_client = AmqpClient::new().await;
    Data::new((pool, amqp_client))
}

pub struct Context {
    pub pool: Pool,
    pub message_loader: MessageLoader,
    pub amqp_client: AmqpClient,
}

impl juniper::Context for Context {}

impl From<AppData> for Context {
    fn from(data: AppData) -> Self {
        Self {
            pool: data.as_ref().0.clone(),
            message_loader: Message::loader(data.as_ref().0.clone()),
            amqp_client: data.as_ref().1.clone(),
        }
    }
}

fn schema() -> Schema {
    Schema::new(Query, Mutation, Subscription)
}

async fn graphiql_route() -> Result<HttpResponse, Error> {
    graphiql_handler("/graphql", Some("/subscriptions")).await
}

async fn playground_route() -> Result<HttpResponse, Error> {
    playground_handler("/graphql", Some("/subscriptions")).await
}

async fn graphql_route(
    req: actix_web::HttpRequest,
    payload: web::Payload,
    data: Data<(Pool, AmqpClient)>,
) -> Result<HttpResponse, Error> {
    graphql_handler(&schema(), &data.into(), req, payload).await
}

async fn subscriptions_route(
    req: actix_web::HttpRequest,
    payload: web::Payload,
    data: Data<(Pool, AmqpClient)>,
) -> Result<HttpResponse, Error> {
    let config = ConnectionConfig::new(data.into());
    let config = config.with_keep_alive_interval(std::time::Duration::from_secs(15));

    subscriptions_handler(req, payload, Arc::new(schema()), config).await
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        resource("/graphql")
            .route(web::post().to(graphql_route))
            .route(web::get().to(graphql_route)),
    )
    .service(resource("/subscriptions").route(web::get().to(subscriptions_route)))
    .service(resource("/graphiql").route(web::get().to(graphiql_route)))
    .service(resource("/playground").route(web::get().to(playground_route)));
}
