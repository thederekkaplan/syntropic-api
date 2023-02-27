use crate::snowflake::snowflake;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use deadpool::managed::{Manager, Object, Pool, RecycleError};
use juniper::futures::{Stream, StreamExt};
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions, QueueDeclareOptions};
use lapin::{Channel, Connection, ConnectionProperties};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr};
use time::OffsetDateTime;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct AmqpError {
    pub message: String,
}

impl Display for AmqpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<T: Error> From<T> for AmqpError {
    fn from(e: T) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

pub trait Protobuf: Sized {
    fn try_to_protobuf(self) -> Result<Vec<u8>, AmqpError>;
    fn try_from_protobuf(payload: &[u8]) -> Result<Self, AmqpError>;
}

struct ChannelManager {
    url: String,
    connection: Mutex<Connection>,
}

impl ChannelManager {
    pub async fn new(url: String) -> Self {
        let connection = Connection::connect(url.as_str(), ConnectionProperties::default())
            .await
            .unwrap();
        Self {
            url,
            connection: Mutex::new(connection),
        }
    }
}

#[async_trait::async_trait]
impl Manager for ChannelManager {
    type Type = Channel;
    type Error = AmqpError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let mut conn = self.connection.lock().await;

        if !conn.status().connected() {
            let connection =
                Connection::connect(self.url.as_str(), ConnectionProperties::default()).await?;
            *conn = connection;
        }

        Ok(conn.create_channel().await?)
    }

    async fn recycle(
        &self,
        channel: &mut Self::Type,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        match channel.status().connected() {
            true => Ok(()),
            false => Err(RecycleError::Message("Channel disconnected".to_string())),
        }
    }
}

#[derive(IntoStaticStr, EnumIter)]
pub enum Exchange {
    Messages,
}

#[derive(Clone)]
pub struct AmqpClient {
    producer: Pool<ChannelManager>,
    consumer: Pool<ChannelManager>,
}

impl AmqpClient {
    pub async fn new(url: String) -> Self {
        let producer = Pool::builder(ChannelManager::new(url.clone()).await)
            .build()
            .unwrap();

        let object: Object<ChannelManager> = producer.get().await.unwrap();
        let channel: &Channel = object.as_ref();
        for exchange in Exchange::iter() {
            channel
                .exchange_declare(
                    exchange.into(),
                    lapin::ExchangeKind::Topic,
                    ExchangeDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    Default::default(),
                )
                .await
                .unwrap();
        }

        let consumer = Pool::builder(ChannelManager::new(url.clone()).await)
            .build()
            .unwrap();

        Self { producer, consumer }
    }

    pub async fn produce(
        &self,
        payload: impl Protobuf,
        exchange: Exchange,
        routing_key: &str,
    ) -> Result<(), AmqpError> {
        let object = self.producer.get().await.map_err(|e| AmqpError {
            message: e.to_string(),
        })?;
        let channel: &Channel = object.as_ref();
        channel
            .basic_publish(
                exchange.into(),
                routing_key,
                BasicPublishOptions::default(),
                &payload.try_to_protobuf()?,
                Default::default(),
            )
            .await?;
        Ok(())
    }

    pub async fn consume<T: Protobuf>(
        &self,
        exchange: Exchange,
        routing_key: &str,
    ) -> Result<impl Stream<Item = T>, AmqpError> {
        let object = self.consumer.get().await.map_err(|e| AmqpError {
            message: e.to_string(),
        })?;
        let channel: &Channel = object.as_ref();
        let queue_id = BASE64_URL_SAFE_NO_PAD.encode(snowflake(OffsetDateTime::now_utc()));

        channel
            .queue_declare(
                queue_id.as_str(),
                QueueDeclareOptions {
                    exclusive: true,
                    auto_delete: true,
                    ..Default::default()
                },
                Default::default(),
            )
            .await?;
        channel
            .queue_bind(
                queue_id.as_str(),
                exchange.into(),
                routing_key,
                Default::default(),
                Default::default(),
            )
            .await?;
        let consumer = channel
            .basic_consume(
                queue_id.as_str(),
                queue_id.as_str(),
                Default::default(),
                Default::default(),
            )
            .await?;

        let result = consumer.filter_map(|message| async {
            let message = message
                .map_err(|err| println!("Error consuming message: {}", err))
                .ok()?;
            let payload = T::try_from_protobuf(&message.data)
                .map_err(|err| println!("Error decoding message: {}", err))
                .ok()?;
            message
                .ack(Default::default())
                .await
                .unwrap_or_else(|err| println!("Error acknowledging message: {}", err));
            Some(payload)
        });

        Ok(result)
    }
}
