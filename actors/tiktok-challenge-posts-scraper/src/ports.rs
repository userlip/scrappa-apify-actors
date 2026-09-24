use anyhow::Result;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushResult {
    pub saved: usize,
    pub limit_reached: bool,
}

pub trait ResultsSink {
    fn available_capacity(&self, requested: usize) -> usize;

    async fn push_videos(&mut self, rows: &[Value]) -> Result<PushResult>;
}
