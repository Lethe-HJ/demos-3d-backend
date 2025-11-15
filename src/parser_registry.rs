use crate::parser::VoxelGridParser;

/// 解析器注册表
/// 管理所有可用的体素网格解析器，并根据文件扩展名匹配对应的解析器
pub struct ParserRegistry {
    parsers: Vec<Box<dyn VoxelGridParser>>,
}

impl ParserRegistry {
    /// 创建新的解析器注册表，自动注册所有可用的解析器
    pub fn new() -> Self {
        let parsers = crate::parsers::get_all_parsers();
        Self { parsers }
    }

    /// 根据文件扩展名查找匹配的解析器
    /// extension: 文件扩展名（不含点号），例如 "vasp"
    pub fn find_parser(&self, extension: &str) -> Option<&dyn VoxelGridParser> {
        self.parsers
            .iter()
            .find(|parser| parser.supports(extension))
            .map(|p| p.as_ref())
    }

    /// 根据文件路径查找匹配的解析器
    /// 自动提取文件扩展名
    pub fn find_parser_for_file(&self, file_path: &str) -> Option<(&dyn VoxelGridParser, String)> {
        // 提取文件扩展名
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())?
            .to_string();

        self.find_parser(&extension)
            .map(|parser| (parser, extension))
    }

    /// 获取所有支持的扩展名列表
    pub fn supported_extensions(&self) -> Vec<String> {
        let mut extensions = Vec::new();
        for parser in &self.parsers {
            extensions.extend(
                parser
                    .supported_extensions()
                    .iter()
                    .map(|s| s.to_lowercase()),
            );
        }
        extensions.sort();
        extensions.dedup();
        extensions
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
