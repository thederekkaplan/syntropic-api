use crate::schema::message as message_schema;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use dataloader::cached::Loader;
use dataloader::BatchFn;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use juniper::async_trait;
use std::collections::HashMap;
use std::error::Error;
use time::OffsetDateTime;

#[derive(Queryable, Insertable, Clone)]
#[diesel(table_name = message_schema)]
pub struct Message {
    pub id: Vec<u8>,
    pub timestamp: OffsetDateTime,
    pub body: String,
}

#[juniper::graphql_object(Context = crate::PostgresContext)]
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
