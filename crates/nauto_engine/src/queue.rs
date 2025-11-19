use anyhow::{Context, Result};
use async_trait::async_trait;
use nauto_model::Job;
use redis::AsyncCommands;
use std::path::PathBuf;

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job: &Job, inventory_path: &str) -> Result<()>;
    async fn dequeue(&self) -> Result<Option<(Job, String)>>;
}

pub struct FileJobQueue {
    path: PathBuf,
}

impl FileJobQueue {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl JobQueue for FileJobQueue {
    async fn enqueue(&self, _job: &Job, _inventory_path: &str) -> Result<()> {
        // Implementation postponed or use existing CLI logic wrapper
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<(Job, String)>> {
        Ok(None)
    }
}

pub struct RedisJobQueue {
    client: redis::Client,
    queue_key: String,
}

impl RedisJobQueue {
    pub fn new(url: &str, queue_key: &str) -> Result<Self> {
        let client = redis::Client::open(url).context("invalid redis url")?;
        Ok(Self {
            client,
            queue_key: queue_key.to_string(),
        })
    }
}

#[async_trait]
impl JobQueue for RedisJobQueue {
    async fn enqueue(&self, job: &Job, inventory_path: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let payload = serde_json::json!({
            "job": job,
            "inventory": inventory_path
        });
        let json = serde_json::to_string(&payload)?;
        let _: () = conn
            .rpush(&self.queue_key, json)
            .await
            .context("redis enqueue")?;
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<(Job, String)>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        // Explicitly specify the return type for lpop
        let result: Option<String> = conn
            .lpop(&self.queue_key, None)
            .await
            .context("redis dequeue")?;
        
        match result {
            Some(json_str) => {
                let payload: serde_json::Value = serde_json::from_str(&json_str)?;
                let job: Job = serde_json::from_value(payload["job"].clone())?;
                let inventory = payload["inventory"].as_str().unwrap_or("").to_string();
                Ok(Some((job, inventory)))
            }
            None => Ok(None),
        }
    }
}
