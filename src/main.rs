mod app_state;
mod handlers;
mod parsers;
mod routes;
mod task;
mod utils;

use std::sync::Arc;

use actix_web::{App, HttpServer, web};

use crate::utils::parser_registry::ParserRegistry;
use app_state::AppState;
use task::TaskStore;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化解析器注册表
    let parser_registry = Arc::new(ParserRegistry::new());
    let resource_dir = "test/resource".to_string();

    let supported_extensions = parser_registry.supported_extensions();
    println!("已注册的解析器:");
    for ext in &supported_extensions {
        println!("  - .{}", ext);
    }

    let task_store = Arc::new(TaskStore::new());
    let app_state = web::Data::new(AppState {
        parser_registry,
        resource_dir: resource_dir.clone(),
        task_store: task_store.clone(),
    });

    // 启动后台清理任务：定期清理过期的任务
    // 每 5 分钟执行一次清理，避免长期占用内存
    let cleanup_store = task_store.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(std::time::Duration::from_secs(5 * 60));
        loop {
            interval.tick().await;
            let cleaned_count = cleanup_store.cleanup_expired();
            if cleaned_count > 0 {
                println!(
                    "[清理任务] 清理了 {} 个过期任务，当前剩余: {} 个任务",
                    cleaned_count,
                    cleanup_store.task_count()
                );
            }
        }
    });

    println!("\n服务器启动在 http://127.0.0.1:8080");
    println!("资源目录: {}", resource_dir);
    println!("任务 TTL: {} 分钟", task_store.default_ttl().as_secs() / 60);
    println!("\n可用接口:");
    println!("  GET / - API 信息");
    println!("  GET /voxel-grid?file=<filename>&chunk_size=<size> - 预处理体素网格数据");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
