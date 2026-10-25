# 更新日志

本项目的所有重大变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
并且本项目遵循 [语义化版本 (Semantic Versioning)](https://semver.org/lang/zh-CN/)。

## [0.1.0] - 2024-01-27

### ✨ 新增
- **[Core]**: 初始预览原型版本。

## [1.0.0] - 2024-02-01

### ✨ 新增
- **[Core]**: AutoFilm 首个正式版本发布，支持将 Alist 挂载资源自动生成对应的 Strm 文件，便于 Emby / Jellyfin 刮削与播放。

## [1.0.1] - 2024-03-02

### ⚡ 优化
- **[Core]**: 重构为面向对象编程结构，规范代码与日志输出。

## [1.0.2] - 2024-05-29

### ✨ 新增
- **[Docker]**: 新增 Dockerfile 与 GitHub Actions 自动化打包 Docker 镜像。

### ⚡ 优化
- **[Core]**: 优化运行线程模型与异常退出处理。

## [1.1.0] - 2024-06-03

### ⚠️ 破坏性变更
- **[Config]**: 配置文件结构有改动调整。
  > **迁移说明**: 请参照最新的 `config/config.example.yaml` 调整 `config.yaml` 结构。

### ⚡ 优化
- **[Core]**: 使用 Alist 官方 REST API 替代旧版 WebDAV 方式进行扫库，支持非基础路径与无 WebDAV 权限用户。
- **[Core]**: 采用异步并发处理，大幅提升扫库与文件处理性能。

## [1.1.2] - 2024-06-29

### ✨ 新增
- **[Alist2Strm]**: 新增 `overwrite` 覆写参数，控制是否跳过同名文件。
- **[Alist2Strm]**: 支持设置字幕、图片、nfo 等非视频资源异步或同步下载。

## [1.2.0] - 2024-07-08

### ⚡ 优化
- **[Core]**: 重构程序运行逻辑，使用 `AsyncIOScheduler` / `APScheduler` 实现后台异步定时任务。
- **[Logging]**: 引入 `LoggerManager` 统一管理程序各模块日志。

## [1.2.2] - 2024-07-17

### ✨ 新增
- **[Ani2Strm]**: 全新引入 Ani2Strm 模块。

### ⚡ 优化
- **[Alist2Strm]**: 适配新版 Alist API。
- **[Alist2Strm]**: 引入通用重试装饰器，提高程序运行稳定性。

## [1.2.3] - 2024-07-24

### 🐛 修复
- **[Ani2Alist]**: 修复请求 URL 编码问题，提升网络请求稳定性。
- **[Alist2Strm]**: 修复未开启覆写模式时重新生成 Strm 文件的 Bug。

### ⚡ 优化
- **[Alist2Strm]**: 减少不必要的网络请求，提升整体处理速度。

## [1.2.4] - 2024-08-26

### ⚠️ 破坏性变更
- **[Alist2Strm]**: 移除旧版 `raw_url` 配置，引入 `mode` 选项替代。
  > **迁移说明**: 请将 `config.yaml` 中 Alist2Strm 模块的 `raw_url` 配置删除，并参照最新的 `config/config.example.yaml` 改用 `mode` 选项重新配置。

### ⚡ 优化
- **[Alist2Strm]**: 区分 `max_workers` 与 `max_downloader` 配置，分别控制并发扫描与并发下载。
- **[Alist2Strm]**: 优化过滤判断逻辑，`AlistPath` 对象的 `suffix` 属性返回值与 `pathlib.Path` 特性保持一致。
- **[Core]**: 提高 Python 3.11 兼容性。

## [1.2.4-1] - 2024-09-24

### ⚡ 优化
- **[Alist2Strm]**: 使用 `AlistPath.download_url` 作为下载地址，避免某些网盘因验证 User-Agent 而导致下载失败。
- **[Alist2Strm]**: 根据 `mode` 模式判断是否需要请求 `async_api_fs_list`，减少不必要的接口调用，降低触发网盘风控概率。
- **[Alist2Strm]**: 完善中文 URL 字符编码处理，提升特殊字符路径的下载稳定性。

## [1.2.5] - 2024-11-08

### ✨ 新增
- **[Alist2Strm]**: 新增同步功能 (#65)，支持将远端目录结构与本地 Strm 文件保持一致。
- **[Alist2Strm]**: 支持使用永久令牌 (Token) 连接 Alist，简化鉴权配置。

### ⚡ 优化
- **[Core]**: 核心模块全面采用单例/多例模式优化，降低重复初始化开销。
- **[Logging]**: 增强日志日期轮换与自定义格式化能力。

### 🐛 修复
- **[Alist2Strm]**: 修复相对路径替换逻辑错误。
- **[Core]**: 强制转换用户名密码为字符串，减少因配置类型不当导致的报错。

## [1.2.6] - 2025-01-10

### ✨ 新增
- **[Alist2Strm]**: 新增 `sync_ignore` 选项，支持在同步时忽略指定的同步规则 (#78)。

### ⚡ 优化
- **[Core]**: 引入 `RequestUtils` 作为全局统一 HTTP 请求出口，基于 `httpx` 替代旧请求库，统一超时、重试与请求头管理。
- **[Logging]**: 增强 `LoggerManager`，开发模式不再单独分离日志文件，Debug 日志支持持久化写入文件，便于问题回溯。

### 🔄 变更
- **[Docker]**: 基础镜像升级为 Alpine 3.12.7。

## [1.3.0] - 2025-03-12

### ✨ 新增
- **[Ani2Alist]**: RSS 订阅模式支持根据 URL 自动生成多级目录与带时间戳的地址树。
- **[Alist2Strm]**: 支持根据本地文件时间戳与状态判断 `.strm` 文件是否过期或损坏，进而自动重新处理。
- **[Utils]**: 新增汉字转拼音工具模块。

### ⚡ 优化
- **[Ani2Alist]**: 优化运行逻辑，在 `run()` 中校验参数，并减少 URL 解码次数以提升执行效率。

### 🐛 修复
- **[Ani2Alist]**: 修复 RSS 订阅更新时对 `storage.addition2dict` 结构中 `url_structure` 的处理错误。
- **[AlistClient]**: 修复仅传递 `token` 时无法实例化客户端的问题。
- **[AlistClient]**: 修复存储创建时 `AlistStorage` 属性调用错误。
- **[AlistClient]**: 修复 HTTP 超时异常未正确抛出的问题。

## [1.3.1] - 2025-03-15

### ✨ 新增
- **[Docs]**: 新增项目贡献者信息展示。

### 🐛 修复
- **[Core]**: 修复重试装饰器参数类型错误，将异常类型参数修正为 `Type[Exception]`。
- **[Core]**: 在 `AlistStorage` 中添加 `model_config` 以忽略特定未知类型，避免 Cython 编译后报错，并精确捕获 `AlistClient` 异常类型。

### 🔧 工程与维护
- **[Docker]**: 使用 Cython 预编译容器内的 Python 文件以提升运行效率，并精简镜像构建、移除无用文件。

## [1.3.2] - 2025-04-04

### ✨ 新增
- **[Alist2Strm]**: 新增 `.mpg` 视频文件后缀支持。
- **[Alist2Strm]**: 新增遍历文件间隔时间配置，避免高频请求触发网盘风控（#96）。

### ⚡ 优化
- **[Core]**: 优化重试装饰器，移除冗余的默认值参数，并完善 Type Hint 异常类型标注。

### 🔄 变更
- **[Config]**: 修正示例配置中的视频扩展名格式。
- **[Logging]**: 统一使用双引号日志格式。

### 🐛 修复
- **[Alist2Strm]**: 捕获路径遍历与文件处理过程中的运行时错误，防止任务意外中断。
- **[Ani2Alist]**: 修正存储更新接口调用，改用异步方法 `async_api_admin_storage_update`。

### 🔧 工程与维护
- **[Docker]**: 优化 Docker 镜像构建，采用多阶段分层构建以减小镜像体积。

## [1.3.3] - 2025-05-29

### ✨ 新增
- **[Alist2Strm]**: 新增空目录添加与删除功能，生成 `.strm` 时可同步维护本地目录结构。
- **[Alist2Strm]**: 新增 `m2ts` 视频文件后缀支持。

### 🐛 修复
- **[Alist2Strm]**: 修复视频扩展名列表中 `.wmv` 缺失前缀点（`.`）导致该格式无法被识别的问题。
- **[Alist2Strm]**: 捕获本地路径过长文件名引发的 `OSError`，避免单个异常中断整体任务。
- **[Alist2Strm]**: 提高 Alist V3.45 兼容性：适配接口返回的 `id` 与 `path` 属性。

### ⚡ 优化
- **[Alist2Strm]**: 完善属性类型注解并补充单元测试，提升代码健壮性。

## [1.3.3-1] - 2025-05-30

### 🐛 修复
- **[AlistClient]**: 修复 `client.py` 与 `alist2strm.py` 中 `_path` 属性引用错误问题。

## [1.3.3-2] - 2025-07-13

### 🐛 修复
- **[Alist2Strm]**: 修复运行模式设置为 `RawURL` 时引发后续错误的问题；更新属性引用，将 `path` 字段替换为 `full_path`，并将部分属性调整为可选类型。

## [1.4.0] - 2025-07-15

### ✨ 新增
- **[LibraryPoster]**: 全新引入 LibraryPoster 媒体库海报美化模块，支持动态阴影效果，支持自动提取主题色去色生成海报背景，并使用项目元数据字典替代单纯 ID 进行处理。
- **[Config]**: 添加默认海报字体文件，并同步更新配置示例。

### 🐛 修复
- **[Ani2Alist]**: 修复模块时间解析问题，确保跟随 ANiOpen 数据更新时间准确修改文件时间戳。

### 🔧 工程与维护
- **[Docker]**: 容器构建中移除 Cython 编译，恢复纯 Python 方式，简化构建与维护流程。
- **[Docs]**: 更新项目文档说明。

## [1.5.0] - 2025-09-26

### ✨ 新增
- **[Alist2Strm]**: 新增 BDMV 蓝光原盘文件结构支持（#133），可正确识别蓝光目录并生成对应盘片的 STRM 文件。

### ⚡ 优化
- **[Alist2Strm]**: 引入 `Alist2StrmMode` 枚举统一管理运行模式，并移除废弃的 fallback 功能。
- **[LibraryPoster]**: 优化对多路径媒体库的处理逻辑，重构媒体库项目的获取与递归处理流程，增强异常场景下的错误容错能力。

## [1.5.0-1] - 2025-10-07

### 🐛 修复
- **[Alist2Strm]**: 修复 `Alist2StrmMode.from_str` 方法无法正确返回枚举值的问题。

## [1.5.1] - 2026-06-03

### ✨ 新增
- **[Alist2Strm]**: 新增 `public_url` 配置支持，实现内网直连与外网访问地址分离，内外网地址处理逻辑自 Alist 基础模块移入 Alist2Strm 统一管理。
- **[Alist2Strm]**: 新增 `.strm` 文件智能保护机制，防止 Alist 服务异常或离线时导致本地大量 STRM 文件被误删。

### ⚡ 优化
- **[Core]**: 重构 HTTP 客户端设计，移除旧版 `RequestUtils` 静态封装，全局复用单例 HTTP 客户端，提升连接复用率与请求稳定性。

### 🔧 工程与维护
- **[Docker]**: 更新容器 Python 版本，并将 `Pillow` 依赖升级至 12.2.0。
- **[Docs]**: 更新项目相关链接与文档说明。

## [2.0.0] - 2026-07-23

### ⚠️ 破坏性变更
- **[Core]**: 使用 Rust 全面重写 AutoFilm，彻底移除原 Python 实现，并发吞吐性能大幅提升，内存开销显著降低。
  > **迁移说明**: 本版本为架构级完全重写，无法在旧版 Python 环境上直接覆盖升级，请做好配置与数据备份后全新部署。
- **[Config]**: 配置文件全面重构为强类型 YAML 结构，配置项层级与字段名称发生较大调整，与 v1.x 版本不兼容。
  > **迁移说明**: 旧版 `config.yaml` 无法直接运行，请参照最新的 `config/config.example.yaml` 重新配置。

### ✨ 新增
- **[Alist2Strm]**: 采用目录扫描与文件处理并行的异步流水线架构，显著提升大规模媒体库的处理效率。
- **[Alist2Strm]**: 新增 `.strm` 智能同步保护机制，防止远程 Alist 服务异常离线或网络波动时导致本地文件被误删。
- **[Alist2Strm]**: 新增 `public_url` 参数支持，实现内网直连与外网访问地址分离。
- **[Alist2Strm]**: 支持多任务共享复用 Alist 客户端实例，并统一限制 API 请求速率以防触发网盘风控。
- **[Ani2Alist]**: 实现基于 ANI 的动漫追番与 UrlTree 映射存储自动化更新。
- **[Ani2Alist]**: 新增 `latest` 追番更新模式，仅增量同步最新剧集。
- **[Ani2Alist]**: 泛型化时区处理，增强对多源时间数据格式的解析兼容性。
- **[LibraryPoster]**: 全新引入媒体库海报封面合成模块，内置 `collage`（拼贴）、`blur`（模糊）、`card`（卡片）、`split`（分割）四种风格，支持旋转与错位调节。
- **[LibraryPoster]**: 提供动态阴影渲染算法，并支持多路径媒体库海报批量处理与更新。
- **[MediaServer]**: 新增 Emby 与 Jellyfin 媒体服务器客户端，海报生成后可自动调用 API 触发媒体库元数据刷新。
- **[Schedule]**: 引入标准 6 字段 Cron 异步调度器，支持基于本地时区的精确任务周期调度。
- **[Config]**: 采用强类型配置解析，在配置文件缺失时支持自动生成标准示例配置。
- **[CLI]**: 基于 `clap` 重构命令行交互，提供完善的 `--help` 帮助文档、自定义配置文件路径支持与 `--debug` 调试模式。
- **[Logging]**: 实现时区感知的按天自动轮转归档日志，支持模块与行号标记，并可自由配置终端彩色高亮输出。

### 🔧 工程与维护
- **[Docker]**: 提供基于 Musl 的轻量级跨平台多架构 Docker 镜像（`linux/amd64` 与 `linux/arm64`）。
- **[CI]**: 搭建 GitHub Actions 自动化 CI/CD 流水线，全面覆盖跨平台编译、Docker 镜像打包与 GitHub Release 发布。

## [2.0.1] - 2026-10-25

### ✨ 新增
- **[Docker]**: Docker 构建工作流支持同时发布多架构镜像至 Docker Hub 与 GitHub Container Registry (GHCR)，便于灵活选择镜像拉取源。

### ⚡ 优化
- **[Build]**: 使用纯 Rust 实现的 `vergen-gix` 重构 `build.rs`，直接解析 `.git` 目录生成编译期元数据，彻底移除对系统 `git` 和 `rustc` 命令行的子进程依赖。
- **[Core]**: 将应用信息中的 `build_time` 字段类型优化为 `DateTime<Local>` 本地时间，序列化 JSON 输出时保持扁平化展示。
- **[Core]**: 开启 Cargo Release 模式下的编译优化（`lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`），大幅减小二进制体积并提升运行性能。
- **[Core]**: 应用信息结构体字段类型由静态字符串调整为动态字符串，优化内存管理。

### 🐛 修复
- **[Ani2Alist]**: 兼容存储器更新接口返回 `null` 数据的响应，避免空数据引发反序列化失败。
- **[Ani2Alist]**: 修复存储器更新请求 ID 被序列化为字符串而触发 400 错误的问题。
- **[Alist2Strm]**: 修复下载链接签名被二次 URL 编码的问题，确保生成的 302 直链可正常跳转播放。
- **[Build]**: 二进制产物名称统一规范为 `autofilm`，修复 CI 构建产物与打包上传路径不匹配的问题。
- **[Docker]**: 修复 Docker 构建工作流中 `tags` 标签配置缩进，确保正确生成多仓库标签规则。

### 🔄 变更
- **[CLI]**: 拆分 Rust 库入口 (`lib.rs`) 与二进制入口 (`bin/main.rs`)，规范程序结构与构建边界。

### 🔒 安全
- **[Deps]**: 升级 `tokio` 至 `1.54.1`，修复多线程运行时下的竞态条件与 Use-After-Free 风险（RUSTSEC-2026-0384 / RUSTSEC-2026-0386）；同步更新 `hyper`、`h2`、`rustls`、`aws-lc-rs` 等 40+ 项上游依赖。

### 🔧 工程与维护
- **[CI]**: 引入 `rust-toolchain.toml` 锁定编译器版本，并在 CI 与多架构 Release 流水线中自动对齐编译器。
- **[CI]**: 重构 GitHub Release 发布工作流，优先从 `CHANGELOG.md` 中动态提取版本变更说明并自动追加配置样例。
- **[CI]**: 扩展单元测试覆盖 Linux (amd64 / arm64)、macOS、Windows 等多平台架构，并将测试与代码格式校验整合至统一 CI 流水线。
- **[Docs]**: 规范化 `CHANGELOG.md` 撰写流程与 Agent Skill，将更新日志从 `README.md` 完整迁移并独立维护。
- **[Core]**: 添加 `rustfmt.toml` 配置并统一全仓代码格式。

---

<!-- ==================== 底部版本比对链接 (Reference Links) ==================== -->
<!-- 说明：以下链接定义用于使各版本标题 [版本号] 具备超链接跳转能力，渲染时不占用页面可见内容 -->
[0.1.0]: https://github.com/AkimioJR/AutoFilm/releases/tag/v0.1.0
[1.0.0]: https://github.com/AkimioJR/AutoFilm/compare/v0.1.0...v1.0.0
[1.0.1]: https://github.com/AkimioJR/AutoFilm/compare/v1.0.0...v1.0.1
[1.0.2]: https://github.com/AkimioJR/AutoFilm/compare/v1.0.1...v1.0.2
[1.1.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.0.2...v1.1.0
[1.1.2]: https://github.com/AkimioJR/AutoFilm/compare/v1.1.0...v1.1.2
[1.2.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.1.2...v1.2.0
[1.2.2]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.0...v1.2.2
[1.2.3]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.2...v1.2.3
[1.2.4]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.3...v1.2.4
[1.2.4-1]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.4...v1.2.4-1
[1.2.5]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.4-1...v1.2.5
[1.2.6]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.5...v1.2.6
[1.3.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.2.6...v1.3.0
[1.3.1]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.0...v1.3.1
[1.3.2]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.1...v1.3.2
[1.3.3]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.2...v1.3.3
[1.3.3-1]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.3...v1.3.3-1
[1.3.3-2]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.3-1...v1.3.3-2
[1.4.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.3.3-2...v1.4.0
[1.5.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.4.0...v1.5.0
[1.5.0-1]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.0...v1.5.0-1
[1.5.1]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.0-1...v1.5.1
[2.0.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.1...v2.0.0
[2.0.1]: https://github.com/AkimioJR/AutoFilm/compare/v2.0.0...v2.0.1
