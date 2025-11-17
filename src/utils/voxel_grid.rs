/// 体素网格数据结构
/// 表示三维规则网格上的标量场数据
#[derive(Debug, Clone)]
pub struct VoxelGrid {
    /// 网格维度 [nx, ny, nz]
    pub shape: [usize; 3],
    /// 数据数组，按 C 语言顺序存储 (x变化最快，y其次，z最慢)
    /// 索引计算: index = k * nx * ny + j * nx + i
    pub data: Vec<f64>,
}

impl VoxelGrid {
    /// 创建新的体素网格
    pub fn new(shape: [usize; 3], data: Vec<f64>) -> Result<Self, String> {
        let total_elements = shape[0] * shape[1] * shape[2];

        if data.len() != total_elements {
            return Err(format!(
                "数据量不匹配: shape {:?} 需要 {} 个元素，但提供了 {} 个",
                shape,
                total_elements,
                data.len()
            ));
        }

        Ok(VoxelGrid { shape, data })
    }

    /// 获取整个数据向量的引用
    pub fn get_data(&self) -> &Vec<f64> {
        &self.data
    }

    /// 获取 shape
    pub fn get_shape(&self) -> [usize; 3] {
        self.shape
    }
}
