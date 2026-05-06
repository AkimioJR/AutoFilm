# AutoFilm API 使用文档

## 项目简介

AutoFilm 是一个为 Emby、Jellyfin 服务器提供直链播放功能的自动化工具。通过生成 STRM 文件，可以让媒体服务器直接播放 Alist 网盘中的视频文件，无需占用本地存储空间。

## 主要功能

### 1. Alist2Strm
自动扫描 Alist 服务器上的视频文件，并生成对应的 `.strm` 文件，支持多种模式和自定义配置。

### 2. Ani2Alist
自动从 AniOpen 项目下载最新番剧并上传到 Alist 服务器，实现自动化追番。

### 3. LibraryPoster
自动为 Emby/Jellyfin 媒体库生成美化的海报封面，提升视觉效果。

## 部署方式

### Docker 部署（推荐）

```bash
docker run -d \
  --name autofilm \
  -v ./config:/config \
  -v ./media:/media \
  -v ./logs:/logs \
  -p 8000:8000 \
  akimio/autofilm
```

### Python 环境运行

```bash
# 安装依赖
pip install -r requirements.txt

# 启动主程序（定时任务模式）
python app/main.py
```

## API 使用说明

### 启用 API 服务

在 `config/config.yaml` 中配置：

```yaml
Settings:
  API_ENABLE: True              # 启用API接口
  API_PORT: 8000                # API服务端口
  API_KEY: your_secret_key      # API访问密钥（请使用强密码）
```

### 认证方式

所有 API 请求都需要在请求头中携带 API Key：

```http
X-API-Key: your_secret_key
```

### API 端点

#### 1. 触发 Alist2Strm 任务

**请求**
```http
POST /trigger/alist2strm?server_id=<任务ID>
```

**参数**
- `server_id`: 配置文件中定义的任务 ID（必需）

**示例**
```bash
curl -X POST "http://localhost:8000/trigger/alist2strm?server_id=动漫" \
  -H "X-API-Key: your_secret_key"
```

**成功响应**
```json
{
  "status": "success"
}
```

**错误响应**
```json
{
  "detail": "Server not found"
}
```

#### 2. 触发 Ani2Alist 任务

**请求**
```http
POST /trigger/ani2alist?server_id=<任务ID>
```

**参数**
- `server_id`: 配置文件中定义的任务 ID（必需）

**示例**
```bash
curl -X POST "http://localhost:8000/trigger/ani2alist?server_id=新番追更" \
  -H "X-API-Key: your_secret_key"
```

**成功响应**
```json
{
  "status": "success"
}
```

#### 3. 触发 LibraryPoster 任务

**请求**
```http
POST /trigger/libraryposter?server_id=<任务ID>
```

**参数**
- `server_id`: 配置文件中定义的任务 ID（必需）

**示例**
```bash
curl -X POST "http://localhost:8000/trigger/libraryposter?server_id=我的Jellyfin" \
  -H "X-API-Key: your_secret_key"
```

**成功响应**
```json
{
  "status": "success"
}
```

### HTTP 状态码说明

| 状态码 | 说明 |
|--------|------|
| 200 | 请求成功 |
| 401 | 未授权（缺少或无效的 API Key） |
| 404 | 任务不存在 |
| 500 | 服务器内部错误 |

## 命令行使用说明

### 查看所有可用任务

```bash
python app/run.py
```

输出示例：
```
使用方法: python run.py <任务ID>
可用的任务ID:
  Alist2Strm: 动漫
  Alist2Strm: 电影
  Ani2Alist: 新番追更
  LibraryPoster: 我的Jellyfin
  LibraryPoster: emby
```

### 执行特定任务

```bash
# 执行 Alist2Strm 任务
python app/run.py 动漫

# 执行 Ani2Alist 任务
python app/run.py 新番追更

# 执行 LibraryPoster 任务
python app/run.py 我的Jellyfin
```

### Docker 容器内执行

```bash
# 查看可用任务
docker exec autofilm python /app/run.py

# 执行特定任务
docker exec autofilm python /app/run.py 动漫
```

## 配置说明

### Alist2Strm 配置

```yaml
Alist2StrmList:
  - id: 动漫                          # 任务唯一标识
    cron: 0 20 * * *                  # 定时任务（Cron 表达式）
    url: http://alist:5244            # Alist 服务器地址
    public_url: https://alist.example.com  # 公共访问地址（可选）
    username: admin                   # 用户名
    password: adminadmin              # 密码
    token: alist-xxx                  # 永久令牌（可选）
    source_dir: /ani/                 # 源目录
    target_dir: /media/               # 目标目录
    mode: AlistURL                    # 模式：AlistURL/RawURL/AlistPath
    flatten_mode: False               # 平铺模式
    subtitle: False                   # 下载字幕
    image: False                      # 下载图片
    nfo: False                        # 下载 NFO 文件
    overwrite: False                  # 覆盖已存在文件
    sync_server: True                 # 同步服务器
    max_workers: 50                   # 最大并发数
    max_downloaders: 5                # 最大下载数
```

### Ani2Alist 配置

```yaml
Ani2AlistList:
  - id: 新番追更
    cron: 20 12 * * *
    url: https://127.0.0.1:5244
    username: admin
    password: myalist
    target_dir: /视频/动漫/新番
    rss_update: False                 # 使用 RSS 订阅更新
    year: 2024                        # 年份
    month: 7                          # 月份
```

### LibraryPoster 配置

```yaml
LibraryPosterList:
  - id: 我的Jellyfin
    cron: 50 13 * * *
    url: http://example.jellyfin.com:8096
    api_key: xxxxxxxxxxxxxxxx
    title_font_path: fonts/ch.ttf
    subtitle_font_path: fonts/en.otf
    configs:
      - library_name: 动漫
        title: 动漫
        subtitle: ANIME
```

## 使用场景

### 场景 1：自动化媒体库管理

结合定时任务，自动扫描 Alist 服务器并更新本地媒体库：

```yaml
Alist2StrmList:
  - id: 自动更新动漫
    cron: 0 2 * * *    # 每天凌晨 2 点执行
    # ... 其他配置
```

### 场景 2：外部触发任务

通过 API 或命令行在外部系统中触发任务：

```bash
# 在媒体服务器启动后触发扫描
curl -X POST "http://localhost:8000/trigger/alist2strm?server_id=动漫" \
  -H "X-API-Key: your_secret_key"
```

### 场景 3：定时追番

自动下载最新番剧并更新到 Alist：

```yaml
Ani2AlistList:
  - id: 每日追番
    cron: 0 18 * * *   # 每天 18 点执行
    rss_update: True   # 使用 RSS 订阅最新番剧
```

### 场景 4：媒体库美化

定期更新媒体库海报：

```yaml
LibraryPosterList:
  - id: 更新海报
    cron: 0 3 * * 0    # 每周日凌晨 3 点执行
    # ... 其他配置
```

## 错误处理

### 常见错误

#### 1. 认证失败
```json
{
  "detail": "Missing API Key"
}
```
**解决方案**：检查请求头中是否包含 `X-API-Key`

#### 2. 任务不存在
```json
{
  "detail": "Server not found"
}
```
**解决方案**：检查 `server_id` 是否正确，使用命令行查看可用任务 ID

#### 3. API 服务未启动
```
Connection refused
```
**解决方案**：检查配置文件中 `API_ENABLE` 是否为 `True`，以及服务是否正常运行

## 安全建议

1. **使用强密码**：API_KEY 应使用足够复杂的随机字符串
2. **限制访问**：建议通过防火墙限制 API 端口的访问来源
3. **使用 HTTPS**：在生产环境中建议使用反向代理（如 Nginx）配置 HTTPS
4. **定期更换密钥**：定期更换 API_KEY 以提高安全性

## 日志查看

### 查看实时日志

```bash
# Docker 环境
docker logs -f autofilm

# Python 环境
tail -f logs/AutoFilm.log
```

### 开发者模式

在配置文件中启用开发者模式以获取更详细的日志：

```yaml
Settings:
  DEV: True
```

日志将输出到 `logs/dev.log`

## 常见问题

### Q1: API 触发任务后如何知道执行结果？

A: 目前 API 仅返回任务启动状态，具体执行结果需要查看日志文件。后续版本会添加任务状态查询接口。

### Q2: 可以同时触发多个任务吗？

A: 可以，API 支持并发触发多个任务。但建议控制并发数量，避免对服务器造成过大压力。

### Q3: 任务执行失败会自动重试吗？

A: 任务本身包含重试机制，但对于严重错误（如配置错误），需要手动修复后重新触发。

### Q4: 如何在脚本中集成 API？

A: 可以使用任何支持 HTTP 请求的编程语言调用 API，示例：

**Python 示例**
```python
import requests

url = "http://localhost:8000/trigger/alist2strm"
params = {"server_id": "动漫"}
headers = {"X-API-Key": "your_secret_key"}

response = requests.post(url, params=params, headers=headers)
print(response.json())
```

**JavaScript 示例**
```javascript
const response = await fetch(
  'http://localhost:8000/trigger/alist2strm?server_id=动漫',
  {
    method: 'POST',
    headers: {
      'X-API-Key': 'your_secret_key'
    }
  }
);
const data = await response.json();
console.log(data);
```

## 技术支持

- 项目地址：[https://github.com/AkimioJR/AutoFilm](https://github.com/AkimioJR/AutoFilm)
- 问题反馈：[https://github.com/AkimioJR/AutoFilm/issues](https://github.com/AkimioJR/AutoFilm/issues)
- 详细文档：[AutoFilm 说明文档](https://blog.akimio.top/posts/1031/)

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
