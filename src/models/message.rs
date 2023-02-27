use crate::amqp::{AmqpError, Protobuf};
use crate::protos::Message as MessageProto;
use crate::schema::message as message_schema;
use crate::snowflake::{snowflake, time_in_millis};
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use dataloader::cached::Loader;
use dataloader::BatchFn;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use juniper::async_trait;
use protobuf::Message as ProtobufMessage;
use protobuf::SpecialFields;
use std::collections::HashMap;
use std::error::Error;
use time::OffsetDateTime;

#[derive(Queryable, Insertable, Clone)]
#[diesel(table_name = message_schema)]
pub struct Message {
    id: Vec<u8>,
    timestamp: OffsetDateTime,
    body: String,
}

impl Message {
    pub fn new(body: String) -> Self {
        let timestamp = time_in_millis();
        let id = snowflake(timestamp);
        Self {
            id,
            timestamp,
            body,
        }
    }
}

impl Protobuf for Message {
    fn try_to_protobuf(self) -> Result<Vec<u8>, AmqpError> {
        let message = MessageProto {
            id: self.id,
            timestamp: (self.timestamp.unix_timestamp_nanos() / 1_000_000) as u64,
            body: self.body,
            special_fields: SpecialFields::new(),
        };
        match message.write_to_bytes() {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(AmqpError {
                message: e.to_string(),
            }),
        }
    }

    fn try_from_protobuf(payload: &[u8]) -> Result<Self, AmqpError> {
        let message = crate::protos::Message::parse_from_bytes(payload).unwrap();
        Ok(Self {
            id: message.id.to_vec(),
            timestamp: match OffsetDateTime::from_unix_timestamp_nanos(
                message.timestamp as i128 * 1_000_000,
            ) {
                Ok(timestamp) => timestamp,
                Err(e) => {
                    return Err(AmqpError {
                        message: e.to_string(),
                    })
                }
            },
            body: message.body.to_string(),
        })
    }
}

#[juniper::graphql_object(Context = crate::Context)]
/// A chat message
impl Message {
    /// The message's unique ID
    pub fn id(&self) -> String {
        BASE64_URL_SAFE_NO_PAD.encode(&self.id)
    }

    /// The text of the message
    pub fn body(&self) -> String {
        self.body.clone()
    }

    /// The time the message was sent
    pub fn timestamp(&self) -> OffsetDateTime {
        self.timestamp
    }
}

pub struct MessageBatcher {
    pool: Pool,
}

#[async_trait]
impl BatchFn<Vec<u8>, Option<Message>> for MessageBatcher {
    async fn load(&mut self, keys: &[Vec<u8>]) -> HashMap<Vec<u8>, Option<Message>> {
        match async {
            let mut messages = HashMap::new();
            for key in keys {
                messages.insert(key.clone(), None);
            }

            let client = self.pool.get().await?;

            let new_keys = keys.to_vec();

            let results: Vec<Message> = client
                .interact(|client| {
                    message_schema::table
                        .filter(message_schema::id.eq_any(new_keys))
                        .load::<Message>(client)
                })
                .await??;

            for result in results {
                messages.insert(result.id.clone(), Some(result));
            }

            Ok(messages) as Result<_, Box<dyn Error>>
        }
        .await
        {
            Ok(messages) => messages,
            Err(_) => HashMap::new(),
        }
    }
}

pub type MessageLoader = Loader<Vec<u8>, Option<Message>, MessageBatcher>;

impl Message {
    pub fn loader(pool: Pool) -> MessageLoader {
        Loader::new(MessageBatcher { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snowflake::snowflake;
    use serial_test::parallel;

    #[test]
    #[parallel]
    fn test_message_id() {
        let timestamp = OffsetDateTime::from_unix_timestamp_nanos(946684800000000000).unwrap();
        let id = snowflake(timestamp);
        let message = Message {
            id,
            timestamp,
            body: "Hello, world!".to_string(),
        };
        assert_eq!(&message.id()[..7], "Dcas-sA");
    }
}
