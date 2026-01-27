//! History record manager
//! 存储测试历史记录，支持统计分析

use crate::models::{HistoryRecord, HistoryStats};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// 历史记录保留天数
const HISTORY_RETENTION_DAYS: i64 = 7;

#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct HistoryManager {
    path: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Self {
        let path = if let Some(dirs) = ProjectDirs::from("com", "anyrouter", "fast") {
            let config_dir = dirs.config_dir();
            fs::create_dir_all(config_dir).ok();
            config_dir.join("history.json")
        } else {
            PathBuf::from("history.json")
        };

        Self { path }
    }

    /// 获取当前 Unix 时间戳（秒）
    fn now_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// 加载所有历史记录
    fn load_records(&self) -> Result<Vec<HistoryRecord>, HistoryError> {
        if self.path.exists() {
            let content = fs::read_to_string(&self.path)?;
            let records: Vec<HistoryRecord> = serde_json::from_str(&content)?;
            Ok(records)
        } else {
            Ok(Vec::new())
        }
    }

    /// 保存历史记录
    fn save_records(&self, records: &[HistoryRecord]) -> Result<(), HistoryError> {
        let content = serde_json::to_string_pretty(records)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// 添加一条历史记录
    pub fn add_record(&self, record: HistoryRecord) -> Result<(), HistoryError> {
        let mut records = self.load_records()?;
        records.push(record);

        // 自动清理过期记录
        let cutoff = Self::now_timestamp() - (HISTORY_RETENTION_DAYS * 24 * 60 * 60);
        records.retain(|r| r.timestamp > cutoff);

        self.save_records(&records)
    }

    /// 批量添加历史记录
    pub fn add_records(&self, new_records: Vec<HistoryRecord>) -> Result<(), HistoryError> {
        if new_records.is_empty() {
            return Ok(());
        }

        let mut records = self.load_records()?;
        records.extend(new_records);

        // 自动清理过期记录
        let cutoff = Self::now_timestamp() - (HISTORY_RETENTION_DAYS * 24 * 60 * 60);
        records.retain(|r| r.timestamp > cutoff);

        self.save_records(&records)
    }

    /// 获取指定时间段内的统计数据
    /// hours: 过去多少小时的数据，0 表示全部
    pub fn get_stats(&self, hours: u32) -> Result<HistoryStats, HistoryError> {
        let records = self.load_records()?;

        let cutoff = if hours > 0 {
            Self::now_timestamp() - (hours as i64 * 60 * 60)
        } else {
            0
        };

        let filtered: Vec<HistoryRecord> = records
            .into_iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        if filtered.is_empty() {
            return Ok(HistoryStats::default());
        }

        let total_tests = filtered.len() as u32;

        // 计算累计节省的时间（只计算成功应用且有加速效果的）
        let total_speedup_ms: f64 = filtered
            .iter()
            .filter(|r| r.applied && r.speedup_percent > 0.0)
            .map(|r| r.original_latency - r.optimized_latency)
            .sum();

        // 计算平均加速百分比
        let speedup_records: Vec<&HistoryRecord> = filtered
            .iter()
            .filter(|r| r.speedup_percent > 0.0)
            .collect();

        let avg_speedup_percent = if !speedup_records.is_empty() {
            speedup_records.iter().map(|r| r.speedup_percent).sum::<f64>()
                / speedup_records.len() as f64
        } else {
            0.0
        };

        // 返回最近的记录（最多 100 条，按时间倒序）
        let mut recent_records = filtered;
        recent_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_records.truncate(100);

        Ok(HistoryStats {
            total_tests,
            total_speedup_ms,
            avg_speedup_percent,
            records: recent_records,
        })
    }

    /// 清理过期记录
    pub fn clear_old(&self) -> Result<u32, HistoryError> {
        let records = self.load_records()?;
        let original_count = records.len();

        let cutoff = Self::now_timestamp() - (HISTORY_RETENTION_DAYS * 24 * 60 * 60);
        let filtered: Vec<HistoryRecord> = records
            .into_iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        let removed_count = (original_count - filtered.len()) as u32;

        if removed_count > 0 {
            self.save_records(&filtered)?;
        }

        Ok(removed_count)
    }

    /// 清空所有历史记录
    pub fn clear_all(&self) -> Result<(), HistoryError> {
        self.save_records(&[])
    }
}
