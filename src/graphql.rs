use crate::amqp::Exchange;
use crate::models::message::Message;
use crate::schema::message as message_schema;
use crate::Context;
use actix_rt::spawn;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use diesel::prelude::*;
use futures::stream::empty;
use futures::Stream;
use juniper::{futures, FieldResult};
use std::pin::Pin;

pub struct Query;
pub struct Mutation;
pub struct Subscription;

#[juniper::graphql_object(Context = crate::Context)]
impl Query {
    /// A list of all messages
    pub async fn messages(context: &Context) -> FieldResult<Vec<Message>> {
        let client = context.pool.get().await?;
        let results: Vec<Message> = client
            .interact(|client| {
                message_schema::table
                    .order(message_schema::id.desc())
                    .load::<Message>(client)
            })
            .await??;

        Ok(results)
    }

    /// The message with the given ID
    pub async fn message(context: &Context, id: String) -> FieldResult<Option<Message>> {
        Ok(context
            .message_loader
            .load(BASE64_URL_SAFE_NO_PAD.decode(id)?)
            .await)
    }
}

#[juniper::graphql_object(Context = crate::Context)]
impl Mutation {
    /// Send a message
    pub async fn send_message(context: &Context, body: String) -> FieldResult<Message> {
        let message = Message::new(body);

        let client = context.pool.get().await?;
        let message: Message = client
            .interact(|client| {
                diesel::insert_into(message_schema::table)
                    .values(message)
                    .get_result(client)
            })
            .await??;

        let client = context.amqp_client.clone();
        let cloned_message = message.clone();

        spawn(async move {
            match client
                .produce(cloned_message, Exchange::Messages, "message")
                .await
            {
                Ok(_) => (),
                Err(err) => println!("Error sending message: {}", err),
            }
        });

        Ok(message)
    }
}

#[juniper::graphql_subscription(Context = crate::Context)]
impl Subscription {
    pub async fn message_received(
        context: &Context,
    ) -> Pin<Box<dyn Stream<Item = Message> + Send>> {
        let stream = context
            .amqp_client
            .clone()
            .consume::<Message>(Exchange::Messages, "message")
            .await;
        match stream {
            Ok(stream) => Box::pin(stream),
            Err(e) => {
                println!("Error consuming messages: {}", e);
                Box::pin(empty())
            }
        }
    }
}
