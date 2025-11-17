use crate::utils::parser::VoxelGridParser;
use crate::utils::voxel_grid::VoxelGrid;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};

/// VASP 文件格式解析器
pub struct VaspParser;

impl VaspParser {
    pub fn new() -> Self {
        VaspParser
    }
}

impl VoxelGridParser for VaspParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["vasp"]
    }

    fn name(&self) -> &'static str {
        "VASP Parser"
    }

    fn parse_from_file(&self, file_path: &str) -> Result<VoxelGrid, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        // 第29行（索引28）包含shape信息
        if lines.len() < 29 {
            return Err(Box::new(Error::new(
                ErrorKind::InvalidData,
                "文件行数不足，无法读取shape信息",
            )));
        }

        // 解析shape: "112  112  108"
        let shape_line = &lines[28]; // 第29行（0-indexed是28）
        let shape: Vec<usize> = shape_line
            .split_whitespace()
            .map(|s| s.parse::<usize>())
            .collect::<Result<_, _>>()
            .map_err(|e| Error::new(ErrorKind::InvalidData, format!("无法解析shape: {}", e)))?;

        if shape.len() != 3 {
            return Err(Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("shape应该包含3个维度，但得到{}个", shape.len()),
            )));
        }

        let shape_array = [shape[0], shape[1], shape[2]];
        let total_elements = shape_array[0] * shape_array[1] * shape_array[2];

        // 从第30行（索引29）开始解析数据
        let mut data = Vec::with_capacity(total_elements);

        for line in lines.iter().skip(29) {
            // 解析每行的浮点数（可能有多个值，用空格分隔）
            for token in line.split_whitespace() {
                // 处理科学计数法（如 0.14631837E+00）
                match token.parse::<f64>() {
                    Ok(value) => data.push(value),
                    Err(_) => {
                        // 如果不是有效的浮点数，跳过（可能是行尾的空格或空行）
                        if !token.trim().is_empty() {
                            eprintln!("警告: 无法解析值 '{}'，已跳过", token);
                        }
                    }
                }
            }
        }

        // 创建体素网格
        VoxelGrid::new(shape_array, data).map_err(|e| {
            Box::new(Error::new(ErrorKind::InvalidData, e)) as Box<dyn std::error::Error>
        })
    }
}
