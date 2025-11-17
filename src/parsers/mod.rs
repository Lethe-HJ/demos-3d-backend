mod vasp;

pub use vasp::VaspParser;

/// 获取所有可用的解析器
pub fn get_all_parsers() -> Vec<Box<dyn crate::utils::parser::VoxelGridParser>> {
    vec![Box::new(VaspParser::new())]
}
