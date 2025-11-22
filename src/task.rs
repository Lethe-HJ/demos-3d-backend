use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct ChunkDescriptor {
    pub index: usize,
    /// 开始位置（包含），单位：浮点元素索引
    pub start: usize,
    /// 结束位置（不包含），单位：浮点元素索引
    pub end: usize,
}

/// 任务数据，存储分块的体素网格数据
/// 使用 HashMap 独立存储每个 chunk，允许单独释放
pub struct TaskData {
    /// 网格维度 [nx, ny, nz]
    #[allow(dead_code)]
    pub shape: [usize; 3],
    /// 分块描述列表
    pub chunks: Vec<ChunkDescriptor>,
    /// 每个 chunk 的数据，key 是 chunk_index
    /// 当 chunk 被请求后，对应的数据会被移除以释放内存
    /// None 表示 chunk 正在解析中，Some(Vec) 表示已就绪
    pub chunk_data: RwLock<HashMap<usize, Option<Vec<f64>>>>,
    /// 任务创建时间，用于 TTL 过期检查
    pub created_at: Instant,
    /// 文件路径，用于后台解析
    #[allow(dead_code)]
    pub file_path: String,
}

impl TaskData {
    /// 创建新的 TaskData（预处理阶段，chunk 尚未解析）
    pub fn new(shape: [usize; 3], chunks: Vec<ChunkDescriptor>, file_path: String) -> Self {
        let mut chunk_data = HashMap::new();
        // 初始化所有 chunk 为 None（表示正在解析中）
        for descriptor in &chunks {
            chunk_data.insert(descriptor.index, None);
        }

        Self {
            shape,
            chunks,
            chunk_data: RwLock::new(chunk_data),
            created_at: Instant::now(),
            file_path,
        }
    }

    /// 设置指定 chunk 的数据（后台解析完成后调用）
    pub fn set_chunk(&self, chunk_index: usize, data: Vec<f64>) {
        self.chunk_data.write().insert(chunk_index, Some(data));
    }

    /// 获取并移除指定 chunk 的数据（用于请求后释放内存）
    /// 返回 None 如果：
    /// - chunk 不存在
    /// - chunk 正在解析中（还未就绪）
    /// - chunk 已被请求
    pub fn take_chunk(&self, chunk_index: usize) -> Option<Vec<f64>> {
        let mut chunk_data = self.chunk_data.write();
        if let Some(Some(data)) = chunk_data.remove(&chunk_index) {
            Some(data)
        } else {
            None
        }
    }

    /// 检查指定 chunk 是否已就绪
    pub fn is_chunk_ready(&self, chunk_index: usize) -> bool {
        self.chunk_data
            .read()
            .get(&chunk_index)
            .map(|opt| opt.is_some())
            .unwrap_or(false)
    }

    /// 检查是否还有未请求的 chunk
    #[allow(dead_code)]
    pub fn has_remaining_chunks(&self) -> bool {
        !self.chunk_data.read().is_empty()
    }

    /// 获取剩余的 chunk 数量
    #[allow(dead_code)]
    pub fn remaining_chunk_count(&self) -> usize {
        self.chunk_data.read().len()
    }
}

pub struct TaskStore {
    tasks: RwLock<HashMap<String, Arc<TaskData>>>,
    /// TTL（Time-To-Live）默认过期时间：30 分钟
    default_ttl: Duration,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            default_ttl: Duration::from_secs(30 * 60), // 30 分钟
        }
    }

    /// 创建带自定义 TTL 的 TaskStore
    #[allow(dead_code)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            default_ttl: ttl,
        }
    }

    pub fn insert(&self, data: TaskData) -> String {
        let task_id = Uuid::new_v4().to_string();
        self.tasks.write().insert(task_id.clone(), Arc::new(data));
        task_id
    }

    pub fn get(&self, task_id: &str) -> Option<Arc<TaskData>> {
        self.tasks.read().get(task_id).cloned()
    }

    /// 清理过期的任务
    /// 返回清理的任务数量
    pub fn cleanup_expired(&self) -> usize {
        let now = Instant::now();
        let mut tasks = self.tasks.write();
        let before_count = tasks.len();

        tasks.retain(|_, task| {
            // 保留未过期的任务
            now.duration_since(task.created_at) < self.default_ttl
        });

        before_count - tasks.len()
    }

    /// 清理所有任务（通常在服务关闭时调用）
    #[allow(dead_code)]
    pub fn clear_all(&self) {
        self.tasks.write().clear();
    }

    /// 获取当前任务数量
    pub fn task_count(&self) -> usize {
        self.tasks.read().len()
    }

    /// 获取默认 TTL
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }
}
