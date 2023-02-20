use serial_test::{parallel, serial};
use syntropic_api::graphql::{Mutation, Query};
use syntropic_api::models::message::Message;
use syntropic_api::{pool, run_migrations, PostgresContext};

async fn context() -> PostgresContext {
    run_migrations().await;
    let pool = pool();
    PostgresContext {
        pool: pool.clone(),
        message_loader: Message::loader(pool.clone()),
    }
}

#[actix_rt::test]
#[parallel]
async fn test_store_and_retrieve_message() {
    let context = context().await;
    let message = Mutation::send_message(&context, "Hello, world!".to_string())
        .await
        .unwrap();
    assert_eq!(message.body(), "Hello, world!");

    let new_message = Query::message(&context, message.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(new_message.body(), message.body());
    assert_eq!(new_message.timestamp(), message.timestamp());
    assert_eq!(new_message.id(), message.id());
}

#[actix_rt::test]
#[serial]
async fn test_retrieve_all_messages() {
    let context = context().await;
    let message = Mutation::send_message(&context, "Hello, world!".to_string())
        .await
        .unwrap();
    let messages = Query::messages(&context).await.unwrap();
    assert_eq!(messages[0].body(), message.body());
    assert_eq!(messages[0].timestamp(), message.timestamp());
    assert_eq!(messages[0].id(), message.id());
}
