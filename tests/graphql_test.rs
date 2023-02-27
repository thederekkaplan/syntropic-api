use juniper::futures::StreamExt;
use serial_test::serial;
use std::thread::sleep;
use std::time::Duration;
use syntropic_api::graphql::{Mutation, Query, Subscription};
use syntropic_api::{data, Context};

#[actix_rt::test]
#[serial]
async fn test_store_and_retrieve_message() {
    let context = Context::from(data().await);
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
    let context = Context::from(data().await);
    let message = Mutation::send_message(&context, "Hello, world!".to_string())
        .await
        .unwrap();
    let messages = Query::messages(&context).await.unwrap();
    assert_eq!(messages[0].body(), message.body());
    assert_eq!(messages[0].timestamp(), message.timestamp());
    assert_eq!(messages[0].id(), message.id());
}

#[actix_rt::test]
#[serial]
async fn test_subscription() {
    let context = Context::from(data().await);
    let mut subscription = Subscription::message_received(&context).await;
    let subscription = subscription.as_mut();
    let message = Mutation::send_message(&context, "Hello, world!".to_string())
        .await
        .unwrap();
    sleep(Duration::from_secs(1));
    let new_message = subscription.into_future().await.0;
    match new_message {
        Some(new_message) => {
            assert_eq!(new_message.body(), message.body());
            assert_eq!(new_message.timestamp(), message.timestamp());
            assert_eq!(new_message.id(), message.id());
        }
        None => panic!("Subscription did not return a new message"),
    }
}
