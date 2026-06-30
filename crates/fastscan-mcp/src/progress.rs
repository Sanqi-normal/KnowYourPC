use std::sync::Arc;

pub trait ProgressCallback: Send + Sync {
    fn emit(&self, phase: &str, processed: u64, total: Option<u64>, message: &str);
}

pub struct NoopCallback;
impl ProgressCallback for NoopCallback {
    fn emit(&self, _phase: &str, _processed: u64, _total: Option<u64>, _message: &str) {}
}

pub struct ScanContext {
    pub progress: Arc<dyn ProgressCallback>,
    pub cluster_size: u64,
}

impl ScanContext {
    pub fn new(progress: Arc<dyn ProgressCallback>, cluster_size: u64) -> Self {
        Self { progress, cluster_size }
    }
}
