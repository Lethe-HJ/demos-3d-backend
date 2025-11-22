use actix_web::{HttpResponse, Responder, get, web};
use serde::Deserialize;

use crate::app_state::AppState;

#[derive(Deserialize)]
pub struct PerformanceQuery {
    pub session_id: String,
}

/// 获取指定会话的性能数据
#[get("/performance")]
pub async fn get_performance(
    data: web::Data<AppState>,
    query: web::Query<PerformanceQuery>,
) -> impl Responder {
    // 调试：打印所有 session_id（通过反射获取，避免直接访问私有字段）
    // 先尝试获取记录来检查是否存在
    let test_records = data.performance_store.get_records(&query.session_id);
    eprintln!(
        "[性能数据查询] 请求的 session_id: {}, 记录数: {}",
        query.session_id,
        test_records.as_ref().map(|r| r.len()).unwrap_or(0)
    );

    let records = data.performance_store.get_records(&query.session_id);
    let record_count = records.as_ref().map(|r| r.len()).unwrap_or(0);
    
    eprintln!(
        "[性能数据查询] session_id: {}, 记录数: {}",
        query.session_id,
        record_count
    );

    // 即使没有记录，也返回空数组，而不是 404 错误
    // 因为可能所有数据都来自缓存，后端没有收到任何请求
    HttpResponse::Ok().json(serde_json::json!({
        "session_id": query.session_id,
        "records": records.unwrap_or_default(),
    }))
}

