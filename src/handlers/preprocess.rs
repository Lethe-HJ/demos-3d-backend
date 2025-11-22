use std::time::Instant;

use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::performance::{get_thread_id, get_unix_timestamp_ms, PerformanceRecord};
use crate::task::{ChunkDescriptor, TaskData};

#[derive(Deserialize)]
pub struct PreprocessRequest {
    pub file: String,
    pub chunk_size: usize,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PreprocessResponse {
    pub task_id: String,
    pub file: String,
    pub file_size: u64,
    pub shape: [usize; 3],
    pub data_length: usize,
    pub chunk_size: usize,
    pub chunks: Vec<ChunkDescriptor>,
}

#[post("/voxel-grid/preprocess")]
pub async fn preprocess_voxel_grid(
    data: web::Data<AppState>,
    payload: web::Json<PreprocessRequest>,
) -> impl Responder {
    let session_id = payload.session_id.clone();
    let start_time = get_unix_timestamp_ms();
    let thread_id = get_thread_id();
    let channel_index = format!("preprocess_{}", thread_id);

    let result = run_preprocess(
        data.get_ref(),
        &payload.file,
        payload.chunk_size,
        session_id.clone(),
    );

    let end_time = get_unix_timestamp_ms();

    // 记录性能数据
    if let Some(ref sid) = session_id {
        let record = PerformanceRecord {
            start_time,
            end_time,
            channel_group: "rust后端".to_string(),
            channel_index: channel_index.clone(),
            msg: format!("预处理请求: {}", payload.file),
        };
        eprintln!("[性能数据记录] 预处理接口 - session_id: {}, channel_index: {}", sid, channel_index);
        data.performance_store.add_record(sid, record);
    } else {
        eprintln!("[性能数据记录] 预处理接口 - session_id 为空，未记录性能数据");
    }

    match result {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(err) => err,
    }
}

/// 预处理体素网格文件：快速创建任务并启动后台解析
///
/// ## 功能概述
/// 这个函数是分块加载流程的预处理步骤，只做轻量级操作：
/// 1. 快速获取文件的 shape（只读取元数据，不解析完整数据）
/// 2. 获取文件大小
/// 3. 根据 chunk_size 计算分块信息
/// 4. 创建任务存储（task_id）
/// 5. 启动后台任务并行解析文件并分割成 chunk
///
/// ## 参数
/// - `app_state`: 应用全局状态，包含解析器注册表、资源目录、任务存储等
/// - `file`: 资源目录下的文件名（如 "CHGDIFF.vasp"）
/// - `chunk_size`: 每个分块包含的元素数量（Float64 个数）
///
/// ## 返回
/// - `Ok(PreprocessResponse)`: 预处理成功，返回 task_id、shape、chunks 等信息
/// - `Err(HttpResponse)`: 预处理失败，返回相应的 HTTP 错误响应
pub fn run_preprocess(
    app_state: &AppState,
    file: &str,
    chunk_size: usize,
    session_id: Option<String>,
) -> Result<PreprocessResponse, HttpResponse> {
    // ==================== 步骤 1: 参数验证与文件路径构建 ====================
    // 确保分块大小至少为 1，避免除零或无效分块
    let chunk_size = chunk_size.max(1);
    // 构建完整文件路径：{资源目录}/{文件名}
    let file_path = format!("{}/{}", app_state.resource_dir, file);

    // ==================== 步骤 2: 查找匹配的解析器 ====================
    // 根据文件扩展名（如 .vasp）从注册表中查找对应的解析器
    let parser = match app_state.parser_registry.find_parser_for_file(&file_path) {
        Some((p, _)) => p,
        None => {
            let supported = app_state.parser_registry.supported_extensions();
            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "不支持的文件格式",
                "file": file,
                "supported_extensions": supported,
            })));
        }
    };

    // ==================== 步骤 3: 获取文件大小 ====================
    let file_size = match std::fs::metadata(&file_path) {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            return Err(HttpResponse::NotFound().json(serde_json::json!({
                "error": "文件不存在或无法访问",
                "file": file,
                "details": e.to_string(),
            })));
        }
    };

    // ==================== 步骤 4: 快速获取 shape（只读取元数据） ====================
    // 使用解析器的轻量级方法，只读取文件的元数据部分（如 VASP 的前 29 行）
    // 不解析完整的体素数据，快速返回
    let shape = match parser.get_shape_from_file(&file_path) {
        Ok(s) => s,
        Err(e) => {
            return Err(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "获取文件 shape 失败",
                "file": file,
                "parser": parser.name(),
                "details": e.to_string(),
            })));
        }
    };

    // ==================== 步骤 5: 计算分块信息 ====================
    // 根据 shape 计算总元素数，然后按照 chunk_size 划分
    let data_length = shape[0] * shape[1] * shape[2];
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    while start < data_length {
        let end = (start + chunk_size).min(data_length);
        chunks.push(ChunkDescriptor { index, start, end });
        start = end;
        index += 1;
    }

    // ==================== 步骤 6: 创建任务存储 ====================
    // 创建 TaskData（此时 chunk 还未解析，chunk_data 中都是 None）
    let task_data = TaskData::new(shape, chunks.clone(), file_path.clone());
    let task_id = app_state.task_store.insert(task_data);

    // 获取任务引用，用于后台解析
    let Some(task) = app_state.task_store.get(&task_id) else {
        return Err(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "创建任务失败",
        })));
    };

    // ==================== 步骤 7: 启动后台任务并行解析文件 ====================
    // 完整解析文件，然后使用多个任务并行分割成多个 chunk 并存储
    // 使用 actix_web::rt::spawn 在后台异步执行，不阻塞预处理响应
    let parser_registry = app_state.parser_registry.clone();
    let task_clone = task.clone();
    let file_path_clone = file_path.clone();
    let chunks_clone = chunks.clone();
    let task_id_clone = task_id.clone();
    let performance_store = app_state.performance_store.clone();
    let session_id_clone = session_id.clone();
    
    actix_web::rt::spawn(async move {
        let parse_start = get_unix_timestamp_ms();
        let parse_thread_id = get_thread_id();
        let parse_channel_index = format!("parse_file_{}", parse_thread_id);
        
        // 步骤 7.1: 解析完整文件（顺序执行，因为文件格式是顺序的）
        let parser = match parser_registry.find_parser_for_file(&file_path_clone) {
            Some((p, _)) => p,
            None => {
                eprintln!("[后台解析] 任务 {task_id_clone} 解析失败：找不到解析器");
                return;
            }
        };

        let voxel_grid = match parser.parse_from_file(&file_path_clone) {
            Ok(grid) => grid,
            Err(e) => {
                eprintln!("[后台解析] 任务 {task_id_clone} 解析文件失败: {e}");
                return;
            }
        };

        let parse_end = get_unix_timestamp_ms();
        
        // 记录文件解析性能数据
        if let Some(ref sid) = session_id_clone {
            let record = PerformanceRecord {
                start_time: parse_start,
                end_time: parse_end,
                channel_group: "rust后端".to_string(),
                channel_index: parse_channel_index.clone(),
                msg: format!("后台解析文件: {}", task_id_clone),
            };
            eprintln!("[性能数据记录] 后台任务 - 解析文件 - session_id: {}, channel_index: {}", sid, parse_channel_index);
            performance_store.add_record(sid, record);
        }

        println!(
            "[后台解析] 任务 {} 文件解析完成，耗时 {:.2}ms",
            task_id_clone,
            parse_end - parse_start
        );

        // 步骤 7.2: 并行分割成多个 chunk（可以并行执行）
        let data = voxel_grid.get_data();
        let split_start = get_unix_timestamp_ms();

        // 使用多个后台任务并行分割和存储 chunk
        let mut handles = Vec::new();
        for descriptor in chunks_clone.iter() {
            let task_ref = task_clone.clone();
            let perf_store = performance_store.clone();
            let sid = session_id_clone.clone();
            // 为每个 chunk 复制对应的数据切片（因为多个任务需要并发读取不同部分）
            let chunk_values: Vec<f64> = data[descriptor.start..descriptor.end].to_vec();
            let chunk_index = descriptor.index;
            let split_thread_id = get_thread_id();
            let split_channel_index = format!("split_chunk_{}", split_thread_id);

            // 为每个 chunk 启动一个任务来存储数据
            let handle = actix_web::rt::spawn(async move {
                let chunk_start = get_unix_timestamp_ms();
                task_ref.set_chunk(chunk_index, chunk_values);
                let chunk_end = get_unix_timestamp_ms();
                
                // 记录分割 chunk 性能数据
                if let Some(ref session_id) = sid {
                    let record = PerformanceRecord {
                        start_time: chunk_start,
                        end_time: chunk_end,
                        channel_group: "rust后端".to_string(),
                        channel_index: split_channel_index.clone(),
                        msg: format!("后台分割 Chunk {}", chunk_index),
                    };
                    eprintln!("[性能数据记录] 后台任务 - 分割Chunk - session_id: {}, channel_index: {}", session_id, split_channel_index);
                    perf_store.add_record(session_id, record);
                }
            });
            handles.push(handle);
        }

        // 等待所有分割任务完成
        for handle in handles {
            let _ = handle.await;
        }

        let split_end = get_unix_timestamp_ms();
        println!(
            "[后台解析] 任务 {} 分割完成，共 {} 个 chunk，耗时 {:.2}ms",
            task_id_clone,
            task_clone.chunks.len(),
            split_end - split_start
        );
    });

    // ==================== 步骤 8: 构造并返回预处理响应 ====================
    // 立即返回，不等待文件解析完成
    // 前端可以通过 chunk 接口请求数据，如果 chunk 还未就绪会返回相应状态
    Ok(PreprocessResponse {
        task_id,
        file: file.to_string(),
        file_size,
        shape,
        data_length,
        chunk_size,
        chunks,
    })
}
