use actix_web::{HttpResponse, Responder, get, web};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::handlers::preprocess::run_preprocess;

#[derive(Deserialize)]
pub struct VoxelGridQuery {
    /// 文件名，例如 "CHGDIFF.vasp"
    pub file: String,
    /// 分块大小（元素数量），必须指定
    pub chunk_size: Option<usize>,
}

/// 体素网格接口，根据文件名自动识别文件格式并解析
/// 例如: /voxel-grid?file=CHGDIFF.vasp
#[get("/voxel-grid")]
pub async fn get_voxel_grid(
    data: web::Data<AppState>,
    query: web::Query<VoxelGridQuery>,
) -> impl Responder {
    let Some(chunk_size) = query.chunk_size else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "缺少 chunk_size 参数",
            "message": "请提供分块大小（元素个数），例如 /voxel-grid?file=xxx&chunk_size=1000000",
        }));
    };

    match run_preprocess(data.get_ref(), &query.file, chunk_size) {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(err) => err,
    }
}
