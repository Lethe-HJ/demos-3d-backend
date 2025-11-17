mod parser;
mod parser_registry;
mod parsers;
mod voxel_grid;

use actix_web::{web, App, HttpResponse, HttpServer, Responder, get, http::header::ContentType};
use serde::Deserialize;
use std::sync::Arc;
use flate2::write::GzEncoder;
use flate2::Compression;
use byteorder::{LittleEndian, WriteBytesExt};
use std::time::Instant;

use parser_registry::ParserRegistry;
use voxel_grid::VoxelGrid;

/// 应用状态
struct AppState {
    parser_registry: Arc<ParserRegistry>,
    resource_dir: String,
}

#[derive(Deserialize)]
struct VoxelGridQuery {
    file: String,  // 文件名，例如 "CHGDIFF.vasp"
}

/// 创建压缩的二进制体素网格数据
/// 格式：
/// 1. 元数据（未压缩）：
///    - shape[0]: u64 (8 bytes)
///    - shape[1]: u64 (8 bytes)
///    - shape[2]: u64 (8 bytes)
///    - file_size: u64 (8 bytes)
///    - data_length: u64 (数据元素数量，8 bytes)
/// 2. 压缩数据（gzip）：
///    - 体素数据：Float64Array (每个元素 8 bytes)
/// 所有数据按小端序（Little Endian）存储
fn create_compressed_voxel_data(
    shape: [usize; 3],
    file_size: u64,
    data: &[f64],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 创建压缩器
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    
    // 写入元数据到压缩流
    encoder.write_u64::<LittleEndian>(shape[0] as u64)?;
    encoder.write_u64::<LittleEndian>(shape[1] as u64)?;
    encoder.write_u64::<LittleEndian>(shape[2] as u64)?;
    encoder.write_u64::<LittleEndian>(file_size)?;
    encoder.write_u64::<LittleEndian>(data.len() as u64)?;
    
    // 写入体素数据
    for &value in data {
        encoder.write_f64::<LittleEndian>(value)?;
    }
    
    // 完成压缩
    let compressed = encoder.finish()?;
    Ok(compressed)
}

/// 体素网格接口
/// 根据文件名自动识别文件格式并解析
/// 例如: /voxel-grid?file=CHGDIFF.vasp
#[get("/voxel-grid")]
async fn get_voxel_grid(
    data: web::Data<AppState>,
    query: web::Query<VoxelGridQuery>,
) -> impl Responder {
    // 构建完整文件路径
    let file_path = format!("{}/{}", data.resource_dir, query.file);

    let t_total_start = Instant::now();

    // 查找匹配的解析器
    let parser = match data.parser_registry.find_parser_for_file(&file_path) {
        Some((p, _)) => p,
        None => {
            let supported = data.parser_registry.supported_extensions();
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "不支持的文件格式",
                "file": query.file,
                "supported_extensions": supported,
            }));
        }
    };

    // 获取文件大小
    let file_size = match std::fs::metadata(&file_path) {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "文件不存在或无法访问",
                "file": query.file,
                "details": e.to_string(),
            }));
        }
    };

    // 解析文件计时
    let t_parse_start = Instant::now();
    let voxel_grid: VoxelGrid = match parser.parse_from_file(&file_path) {
        Ok(grid) => grid,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "解析文件失败",
                "file": query.file,
                "parser": parser.name(),
                "details": e.to_string(),
            }));
        }
    };
    let parse_ms = t_parse_start.elapsed().as_millis();

    // 压缩计时
    let t_comp_start = Instant::now();
    let compressed_data = match create_compressed_voxel_data(
        voxel_grid.get_shape(),
        file_size,
        voxel_grid.get_data(),
    ) {
        Ok(data) => data,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "压缩数据失败",
                "details": e.to_string(),
            }));
        }
    };
    let comp_ms = t_comp_start.elapsed().as_millis();
    let total_ms = t_total_start.elapsed().as_millis();

    // 返回压缩的二进制数据 + 耗时头
    HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .append_header(("X-Parse-Duration-ms", parse_ms.to_string()))
        .append_header(("X-Compress-Duration-ms", comp_ms.to_string()))
        .append_header(("X-Total-Duration-ms", total_ms.to_string()))
        .append_header(("X-File-Size", file_size.to_string()))
        .body(compressed_data)
}

#[get("/")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    let supported = data.parser_registry.supported_extensions();
    HttpResponse::Ok().json(serde_json::json!({
        "message": "体素网格数据服务",
        "endpoint": "/voxel-grid?file=<filename>",
        "supported_extensions": supported,
        "resource_dir": data.resource_dir,
    }))
}

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

    let app_state = web::Data::new(AppState {
        parser_registry,
        resource_dir: resource_dir.clone(),
    });

    println!("\n服务器启动在 http://127.0.0.1:8080");
    println!("资源目录: {}", resource_dir);
    println!("\n可用接口:");
    println!("  GET / - API 信息");
    println!("  GET /voxel-grid?file=<filename> - 获取体素网格数据（自动识别格式）");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(hello)
            .service(get_voxel_grid)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
