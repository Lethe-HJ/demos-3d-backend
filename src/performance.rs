use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 性能数据记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    /// 开始时间 (Unix 时间戳，毫秒)
    pub start_time: u64,
    /// 结束时间 (Unix 时间戳，毫秒)
    pub end_time: u64,
    /// 行组 (同一行组颜色相同)
    pub channel_group: String,
    /// 行号 (在同一行组内的索引，这里使用字符串标识，如 "preprocess_1", "parse_file_2")
    pub channel_index: String,
    /// 消息 (hover 时除了时间外的显示信息)
    pub msg: String,
}

/// 性能数据存储
/// 按 session_id 存储性能记录
pub struct PerformanceStore {
    /// session_id -> 性能记录列表
    pub records: RwLock<HashMap<String, Vec<PerformanceRecord>>>,
    /// TTL（Time-To-Live）默认过期时间：30 分钟
    default_ttl: Duration,
    /// session_id -> 创建时间
    session_times: RwLock<HashMap<String, SystemTime>>,
}

impl PerformanceStore {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            default_ttl: Duration::from_secs(30 * 60), // 30 分钟
            session_times: RwLock::new(HashMap::new()),
        }
    }

    /// 添加性能记录
    pub fn add_record(&self, session_id: &str, record: PerformanceRecord) {
        let mut records = self.records.write();
        let entry = records.entry(session_id.to_string()).or_insert_with(Vec::new);
        entry.push(record);

        // 记录会话创建时间（如果还没有）
        let mut session_times = self.session_times.write();
        session_times
            .entry(session_id.to_string())
            .or_insert_with(SystemTime::now);
    }

    /// 批量添加性能记录
    pub fn add_records(&self, session_id: &str, records: Vec<PerformanceRecord>) {
        let mut all_records = self.records.write();
        let entry = all_records
            .entry(session_id.to_string())
            .or_insert_with(Vec::new);
        entry.extend(records);

        // 记录会话创建时间（如果还没有）
        let mut session_times = self.session_times.write();
        session_times
            .entry(session_id.to_string())
            .or_insert_with(SystemTime::now);
    }

    /// 获取指定会话的所有性能记录
    pub fn get_records(&self, session_id: &str) -> Option<Vec<PerformanceRecord>> {
        self.records.read().get(session_id).cloned()
    }

    /// 清理过期的会话
    pub fn cleanup_expired(&self) -> usize {
        let now = SystemTime::now();
        let mut records = self.records.write();
        let mut session_times = self.session_times.write();
        let before_count = records.len();

        let expired_sessions: Vec<String> = session_times
            .iter()
            .filter_map(|(session_id, created_at)| {
                if now.duration_since(*created_at).unwrap_or(Duration::ZERO) > self.default_ttl {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
            .collect();

        for session_id in &expired_sessions {
            records.remove(session_id);
            session_times.remove(session_id);
        }

        before_count - records.len()
    }

    /// 清理所有数据
    pub fn clear_all(&self) {
        self.records.write().clear();
        self.session_times.write().clear();
    }
}

/// 获取当前线程 ID（用于标识）
/// 使用线程本地存储的计数器生成唯一标识
thread_local! {
    static THREAD_COUNTER: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

static NEXT_THREAD_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

pub fn get_thread_id() -> usize {
    THREAD_COUNTER.with(|counter| {
        let id = counter.get();
        if id == 0 {
            let new_id = NEXT_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            counter.set(new_id);
            new_id
        } else {
            id
        }
    })
}

/// 获取 Unix 时间戳（毫秒）
pub fn get_unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

