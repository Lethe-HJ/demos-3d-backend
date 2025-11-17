# 项目结构说明

以下结构遵循常见的 Actix Web 后端拆分方式：入口 `main.rs` 只负责启动流程，业务逻辑拆分到单独模块，方便扩展与测试。

```
demos-3d-backend/
├── src/
│   ├── main.rs                // 程序入口：初始化状态、启动 HttpServer
│   ├── app_state.rs           // 全局共享状态（解析器注册表、资源目录等）
│   ├── routes.rs              // 统一的路由注册入口
│   ├── handlers/              // 所有 HTTP handler（按领域继续细分）
│   │   ├── mod.rs             // handler 子模块聚合 & 对外导出
│   │   ├── health.rs          // 根路径 / 健康检查 & 服务说明
│   │   └── voxel_grid.rs      // /voxel-grid 主业务接口
│   ├── parsers/               // 各类格式解析器实现
│   │   ├── mod.rs
│   │   └── vasp.rs
│   └── utils/                 // 领域通用能力的集中出口
│       ├── mod.rs
│       ├── parser.rs          // Parser trait 定义
│       ├── parser_registry.rs // 动态选择合适解析器的注册表
│       └── voxel_grid.rs      // 体素网格结构与数据访问封装
└── docs/
    └── PROJECT_STRUCTURE.md   // 当前文档
```

## 模块职责

- `main.rs`：拼装依赖、输出运行信息，并调用 `routes::configure` 注册路由。
- `app_state::AppState`：集中承载 `ParserRegistry` 与资源目录，借助 `web::Data` 注入到每个 handler。
- `routes::configure`：对外唯一的路由注册点，新增接口时仅需在此注册对应 handler。
- `handlers` 目录：按业务拆分具体接口逻辑；`voxel_grid` 中包含压缩体素数据的辅助函数，`health` 提供基本服务说明。
- `utils` 目录：沉淀复用逻辑，`parser*` 负责体素文件解析接口与注册表，`voxel_grid` 存放核心数据结构。

## 扩展建议

1. **新增接口**：在 `handlers/` 下创建新文件实现 `#[get]`/`#[post]` 等函数，并在 `routes::configure` 中注册。
2. **扩展状态**：把新的共享依赖加入 `AppState`，即可在所有 handler 中通过 `web::Data<AppState>` 访问。
3. **新增解析器**：在 `parsers/` 下实现对应模块并在 `parser_registry` 中注册，即可自动被 `voxel_grid` handler 识别。

