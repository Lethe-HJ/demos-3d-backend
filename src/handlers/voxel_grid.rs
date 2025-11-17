use std::time::Instant;

use actix_web::{HttpResponse, Responder, get, http::header::ContentType, web};
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::{Compression, write::GzEncoder};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::utils::voxel_grid::VoxelGrid;

#[derive(Deserialize)]
pub struct VoxelGridQuery {
    /// 文件名，例如 "CHGDIFF.vasp"
    pub file: String,
}

/// 体素网格接口，根据文件名自动识别文件格式并解析
/// 例如: /voxel-grid?file=CHGDIFF.vasp
#[get("/voxel-grid")]
pub async fn get_voxel_grid(
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

/// 创建压缩的二进制体素网格数据
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
