use async_trait::async_trait;
use nauto_model::{Job, JobResult, TaskSummary};
use anyhow::Result;
use uuid::Uuid;

#[async_trait]
pub trait JobStore: Send + Sync {
    async fn create_job(&self, job: &Job) -> Result<()>;
    async fn update_task_summary(&self, job_id: Uuid, summary: &TaskSummary) -> Result<()>;
    async fn complete_job(&self, job_id: Uuid, result: &JobResult) -> Result<()>;
}

pub struct NoOpJobStore;

#[async_trait]
impl JobStore for NoOpJobStore {
    async fn create_job(&self, _job: &Job) -> Result<()> { Ok(()) }
    async fn update_task_summary(&self, _job_id: Uuid, _summary: &TaskSummary) -> Result<()> { Ok(()) }
    async fn complete_job(&self, _job_id: Uuid, _result: &JobResult) -> Result<()> { Ok(()) }
}
