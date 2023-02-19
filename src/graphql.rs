use crate::models::message::Message;
use crate::schema::message as message_schema;
use crate::snowflake::{snowflake, time_millis};
use crate::PostgresContext;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use diesel::prelude::*;
use juniper::FieldResult;

pub struct Query;
pub struct Mutation;

#[juniper::graphql_object(Context = crate::PostgresContext)]
impl Query {
    /// A list of all messages
    pub async fn messages(context: &PostgresContext) -> FieldResult<Vec<Message>> {
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
    pub async fn message(id: String, context: &PostgresContext) -> FieldResult<Option<Message>> {
        Ok(context
            .message_loader
            .load(BASE64_URL_SAFE_NO_PAD.decode(id)?)
            .await)
    }
}

#[juniper::graphql_object(Context = crate::PostgresContext)]
impl Mutation {
    /// Send a message
    pub async fn send_message(context: &PostgresContext, body: String) -> FieldResult<Message> {
        let timestamp = time_millis()?;
        let id = snowflake(timestamp);
        let message = Message {
            id,
            timestamp,
            body,
        };

        let client = context.pool.get().await?;
        let message = client
            .interact(|client| {
                diesel::insert_into(message_schema::table)
                    .values(message)
                    .get_result(client)
            })
            .await??;

        Ok(message)
    }
}
