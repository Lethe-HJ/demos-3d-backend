use std::sync::Arc;

use crate::performance::PerformanceStore;
use crate::task::TaskStore;
use crate::utils::parser_registry::ParserRegistry;

/// 全局应用状态，负责在各个 handler 之间共享解析器与资源目录
pub struct AppState {
    pub parser_registry: Arc<ParserRegistry>,
    pub resource_dir: String,
    pub task_store: Arc<TaskStore>,
    pub performance_store: Arc<PerformanceStore>,
}
