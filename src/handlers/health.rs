use actix_web::{HttpResponse, Responder, get, web};

use crate::app_state::AppState;

/// 根路径健康检查/服务说明
#[get("/")]
pub async fn hello(data: web::Data<AppState>) -> impl Responder {
    let supported = data.parser_registry.supported_extensions();
    HttpResponse::Ok().json(serde_json::json!({
        "message": "体素网格数据服务",
        "endpoint": "/voxel-grid?file=<filename>",
        "supported_extensions": supported,
        "resource_dir": data.resource_dir,
    }))
}
