use crate::utils::voxel_grid::VoxelGrid;

/// 体素网格解析器 trait
/// 不同文件格式需要实现这个 trait
pub trait VoxelGridParser: Send + Sync {
    /// 获取支持的文件扩展名（不含点号），例如: "vasp"
    fn supported_extensions(&self) -> Vec<&'static str>;

    /// 检查文件扩展名是否被支持
    fn supports(&self, extension: &str) -> bool {
        self.supported_extensions()
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(extension))
    }

    /// 从文件路径解析体素网格数据
    fn parse_from_file(&self, file_path: &str) -> Result<VoxelGrid, Box<dyn std::error::Error>>;

    /// 快速获取文件的 shape（只读取元数据，不解析完整数据）
    /// 用于预处理阶段快速返回基本信息
    fn get_shape_from_file(
        &self,
        file_path: &str,
    ) -> Result<[usize; 3], Box<dyn std::error::Error>>;

    /// 获取解析器名称（用于日志和错误信息）
    fn name(&self) -> &'static str;
}
