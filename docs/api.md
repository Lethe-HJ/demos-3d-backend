# API 文档

所有接口默认挂载在 `http://127.0.0.1:8080`。

> 若无特殊说明，响应中都会包含 `Content-Type: application/json`，错误时返回 `{"error": "..."}`。

---

## 1. `GET /`

返回当前服务的基础信息：

```json
{
  "message": "体素网格数据服务",
  "endpoint": "/voxel-grid?file=<filename>",
  "supported_extensions": ["vasp", ...],
  "resource_dir": "test/resource"
}
```

---

## 2. `GET /voxel-grid`

> 该接口现在只支持 **分块模式**。若缺少 `chunk_size` 参数会返回 400。

### Query 参数

| 参数名      | 类型     | 是否必填 | 说明                                |
|-------------|----------|----------|-------------------------------------|
| `file`      | string   | ✓        | 资源目录下的文件名，如 `CHGDIFF.vasp` |
| `chunk_size`| number   | ✓        | 分块大小（元素个数），如 `1_000_000` |

### 成功响应示例

```json
{
  "task_id": "6a4c7c5e-...",
  "file": "CHGDIFF.vasp",
  "file_size": 1234567,
  "shape": [112, 112, 108],
  "data_length": 1354752,
  "chunk_size": 1000000,
  "chunks": [
    { "index": 0, "start": 0, "end": 1000000 },
    { "index": 1, "start": 1000000, "end": 1354752 }
  ]
}
```

**注意**：
- 此接口只做轻量级预处理，**立即返回**，不等待文件解析完成
- 文件解析在后台异步进行，解析完成后 chunk 才可用
- 请求 chunk 时，如果 chunk 还未就绪，会返回 `202 Accepted` 状态码

响应中的字段说明：
- `task_id`: 后续 `chunk` 接口所需的任务 ID
- `shape`: 三维网格维度 `[nx, ny, nz]`
- `data_length`: 总元素数量（`shape[0] * shape[1] * shape[2]`）
- `chunks`: 每个分块在原始数组中的 `[start, end)` 索引（单位：元素）
- **`min/max`**: 不再在此接口返回，由前端 worker 在解析各自 chunk 时计算并整合

---

## 3. `GET /voxel-grid/chunk`

根据预处理时返回的 `task_id` 与 `chunk_index` 获取某个分块的二进制数据。

### Query 参数

| 参数名         | 类型   | 是否必填 | 说明                             |
|----------------|--------|----------|----------------------------------|
| `task_id`      | string | ✓        | 预处理返回的 `task_id`           |
| `chunk_index`  | number | ✓        | 预处理返回的 `chunks[i].index`   |

### 响应状态

**1. 成功响应（200 OK）**：
- `Content-Type: application/octet-stream`
- body: 小端序 Float64Array
- 响应头包含：
  - `X-Chunk-Index`
  - `X-Chunk-Start`
  - `X-Chunk-End`
  - `X-Chunk-Length`
  - `X-Chunk-Task`

**2. 处理中（202 Accepted）**：
```json
{
  "error": "chunk 正在解析中，请稍后重试",
  "task_id": "...",
  "chunk_index": 0,
  "status": "processing"
}
```
表示 chunk 还在后台解析中，客户端应该稍后重试。

**3. 错误响应（400 Bad Request）**：
- chunk 已被请求（只能请求一次）
- 无效的 task_id 或 chunk_index

> 客户端建议直接以 `response.arrayBuffer()` 读取，再用 `Float64Array` 解析。如果收到 202 状态，建议使用指数退避策略重试。

---

## 4. `POST /voxel-grid/preprocess`

功能与 `GET /voxel-grid` 的分块模式相同，只是通过 POST 提供参数。

### Request Body

```json
{
  "file": "CHGDIFF.vasp",
  "chunk_size": 1000000
}
```

### Response

同 `GET /voxel-grid` 的成功示例。

> 业务上推荐优先使用 `GET /voxel-grid`，若需要自定义请求体或未来扩展则可使用 `POST /voxel-grid/preprocess`。

---

## 5. 错误响应示例

```json
{
  "error": "不支持的文件格式",
  "file": "xxx.xyz",
  "supported_extensions": ["vasp"]
}
```

常见状态码：
- 400: 参数缺失或格式不支持、chunk 已请求
- 404: 文件不存在
- 202: chunk 正在解析中（仅 chunk 接口）
- 500: 解析或分块失败

