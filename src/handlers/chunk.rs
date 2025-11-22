use actix_web::{HttpResponse, Responder, get, http::header::ContentType, web};
use byteorder::{LittleEndian, WriteBytesExt};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::performance::{get_thread_id, get_unix_timestamp_ms, PerformanceRecord};

#[derive(Deserialize)]
pub struct ChunkQuery {
    pub task_id: String,
    pub chunk_index: usize,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[get("/voxel-grid/chunk")]
pub async fn get_voxel_chunk(
    data: web::Data<AppState>,
    query: web::Query<ChunkQuery>,
) -> impl Responder {
    let start_time = get_unix_timestamp_ms();
    let thread_id = get_thread_id();
    let channel_index = format!("get_chunk_{}", thread_id);
    
    let Some(task) = data.task_store.get(&query.task_id) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "无效的 task_id",
            "task_id": query.task_id,
        }));
    };

    let Some(descriptor) = task.chunks.get(query.chunk_index) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "无效的 chunk_index",
            "chunk_index": query.chunk_index,
        }));
    };

    // 检查 chunk 是否已就绪（后台解析是否完成）
    if !task.is_chunk_ready(query.chunk_index) {
        return HttpResponse::Accepted().json(serde_json::json!({
            "error": "chunk 正在解析中，请稍后重试",
            "task_id": query.task_id,
            "chunk_index": query.chunk_index,
            "status": "processing",
        }));
    }

    // 获取并移除 chunk 数据（请求后立即释放内存）
    // 如果 chunk 已被请求，take_chunk 会返回 None
    let Some(chunk_values) = task.take_chunk(query.chunk_index) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "chunk 已被请求或不存在",
            "task_id": query.task_id,
            "chunk_index": query.chunk_index,
        }));
    };

    // 将 chunk 数据序列化为二进制格式
    let mut bytes = Vec::with_capacity(chunk_values.len() * std::mem::size_of::<f64>());
    for value in chunk_values {
        if let Err(e) = bytes.write_f64::<LittleEndian>(value) {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "写入 chunk 数据失败",
                "details": e.to_string(),
            }));
        }
    }

    let end_time = get_unix_timestamp_ms();
    
    // 记录性能数据
    if let Some(ref session_id) = query.session_id {
        let record = PerformanceRecord {
            start_time,
            end_time,
            channel_group: "rust后端".to_string(),
            channel_index: channel_index.clone(),
            msg: format!("获取 Chunk {}", query.chunk_index),
        };
        eprintln!("[性能数据记录] Chunk接口 - session_id: {}, channel_index: {}", session_id, channel_index);
        data.performance_store.add_record(session_id, record);
    } else {
        eprintln!("[性能数据记录] Chunk接口 - session_id 为空，未记录性能数据");
    }

    HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .append_header(("X-Chunk-Index", descriptor.index.to_string()))
        .append_header(("X-Chunk-Start", descriptor.start.to_string()))
        .append_header(("X-Chunk-End", descriptor.end.to_string()))
        .append_header((
            "X-Chunk-Length",
            (descriptor.end - descriptor.start).to_string(),
        ))
        .append_header(("X-Chunk-Task", query.task_id.clone()))
        .body(bytes)
}
