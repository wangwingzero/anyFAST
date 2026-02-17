//! 健康检查模块（已移除自动切换逻辑）
//! 仅保留基准延迟跟踪功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 基准延迟跟踪器
/// 记录每个域名的基准延迟，用于测速结果展示
pub struct BaselineTracker {
    baselines: Arc<Mutex<HashMap<String, f64>>>,
}

impl BaselineTracker {
    pub fn new() -> Self {
        Self {
            baselines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取 baselines 的 Arc 克隆（避免长时间持有锁）
    pub fn get_baselines_arc(&self) -> Arc<Mutex<HashMap<String, f64>>> {
        self.baselines.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_baseline_tracker_new() {
        let tracker = BaselineTracker::new();
        let baselines = tracker.get_baselines_arc();
        let b = baselines.lock().await;
        assert!(b.is_empty());
    }

    #[tokio::test]
    async fn test_baseline_tracker_set_and_get() {
        let tracker = BaselineTracker::new();
        let baselines = tracker.get_baselines_arc();
        {
            let mut b = baselines.lock().await;
            b.insert("test.com".to_string(), 100.0);
        }
        let b = baselines.lock().await;
        assert_eq!(b.get("test.com"), Some(&100.0));
    }
}
