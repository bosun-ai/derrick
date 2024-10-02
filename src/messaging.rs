use anyhow::Result;
pub use async_nats::Subscriber;
use base64::Engine;
use crate::config;

pub async fn establish_connection() -> Result<async_nats::client::Client> {
    let nats_creds_b64 = crate::config()
        .nats_creds
        .clone()
        .ok_or_else(|| anyhow::anyhow!("NATS_CREDS not set"))?;

    let nats_endpoint = crate::config()
        .nats_endpoint
        .clone()
        .ok_or_else(|| anyhow::anyhow!("NATS_ENDPOINT not set"))?;

    let nats_creds_vec = base64::prelude::BASE64_STANDARD
        .decode(nats_creds_b64)
        .map_err(|e| anyhow::anyhow!("Failed to decode NATS creds: {}", e))?;

    let nats_creds = std::str::from_utf8(&nats_creds_vec)
        .map_err(|e| anyhow::anyhow!("Failed to parse NATS creds: {}", e))?;

    async_nats::ConnectOptions::with_credentials(&nats_creds)
        .map_err(anyhow::Error::msg)?
        .connect(&nats_endpoint)
        .await
        .map_err(anyhow::Error::msg)
}

#[derive(Debug)]
pub struct Channel {
    client: async_nats::client::Client,
    pub channel_topic: String,
    pub channel_instance_subject: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelAnnouncementMessage {
    pub channel_topic: String,
    pub channel_instance_subject: String,
    pub initial_message: String,
}

pub fn random_hex(len: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let hex_chars = "0123456789abcdef";

    let bytes: Vec<u8> = thread_rng().sample_iter(&Alphanumeric).take(len).collect();
    bytes
        .iter()
        .map(|b| hex_chars.chars().nth(*b as usize % 16).unwrap())
        .collect()
}

impl Channel {
    // The idea of a messaging channel is that there is an announcement subject that is used to announce the channel
    // and a channel topic that is used to indicate what the channel is about. The channel instance subject is a unique
    // subject that is used to communicate with the channel.
    pub async fn establish_and_announce(
        announcement_subject: String,
        channel_topic: String,
        initial_message: String,
    ) -> Result<(Self, Subscriber)> {
        let channel_instance_subject = format!("{}.{}", channel_topic, random_hex(8));

        // TODO clients could be reused, no reason to establish every time
        let client = establish_connection().await?;

        let subscriber = client.subscribe(channel_instance_subject.clone()).await?;

        let announcement = ChannelAnnouncementMessage {
            channel_topic: channel_topic.clone(),
            channel_instance_subject: channel_instance_subject.clone(),
            initial_message,
        };

        let announcement_serialized = serde_json::to_string(&announcement)?;

        // announce the channel
        client
            .publish(announcement_subject.clone(), announcement_serialized.into())
            .await?;

        Ok((
            Self {
                channel_topic,
                channel_instance_subject,
                client,
            },
            subscriber,
        ))
    }

    pub async fn establish(topic: String) -> Result<(Self)> {
        let channel_instance_subject = format!("{}.{}", topic, random_hex(8));

        let client = establish_connection().await?;

        Ok((Self {
            channel_topic: topic,
            channel_instance_subject,
            client,
        }))
    }

    pub async fn subscribe(&self) -> Result<Subscriber> {
        self.client
            .subscribe(self.channel_instance_subject.clone())
            .await
            .map_err(anyhow::Error::msg)
    }

    pub async fn publish(&self, message: String) -> Result<()> {
        self.client
            .publish(self.channel_instance_subject.clone(), message.into())
            .await
            .map_err(anyhow::Error::msg)
    }

    pub async fn request(&self, message: String) -> Result<String> {
        let response = self
            .client
            .request(self.channel_instance_subject.clone(), message.into())
            .await
            .map_err(anyhow::Error::msg)?;

        let response_bytes = response.payload;
        let response_str = std::str::from_utf8(&response_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
        Ok(response_str.to_string())
    }
}
