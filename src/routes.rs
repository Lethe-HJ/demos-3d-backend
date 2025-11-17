use actix_web::web;

use crate::handlers;

/// 统一注册 HTTP 路由，方便集中管理
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::hello)
        .service(handlers::get_voxel_grid);
}
