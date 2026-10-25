---
name: changelog
description: Standardize, write, and maintain CHANGELOG.md for AutoFilm following Keep a Changelog and Semantic Versioning principles. Use when creating, updating, or reviewing CHANGELOG entries.
---

# AutoFilm CHANGELOG 规范

本 Skill 用于规范 AutoFilm 仓库中 `CHANGELOG.md` 的创建、维护与更新，确保版本变更记录清晰、结构一致、面向用户且便于自动化工具与人类阅读。

---

## 1. 核心原则 (Core Principles)

1. **以人为本 (Human-Readable)**:
   - 更新日志是写给**用户与维护者**看的，重点说明“变更对使用者带来了什么价值、有什么影响、需要如何调整配置”，严禁机械照搬底层 Git 提交信息（如“update code”、“fix bug”）。
2. **遵循国际标准 (Standard-Compliant)**:
   - 严格遵循 [Keep a Changelog 1.1.0](https://keepachangelog.com/zh-CN/1.1.0/) 组织结构。
   - 版本号严格遵循 [语义化版本 2.0.0 (SemVer)](https://semver.org/lang/zh-CN/)（格式：`主版本号.次版本号.修订号`，如 `2.0.0`，前缀可带 `v` 用于 Git Tag）。
3. **单一事实来源 (Single Source of Truth)**:
   - 根目录的 `CHANGELOG.md` 是项目变更记录的唯一权威来源。
   - `README.md` 中不堆叠详细日志，仅保留指向 `CHANGELOG.md` 的跳转链接（或至多展示最近一个版本的速览）。
4. **模块作用域明确 (Scoped & Structured)**:
   - 每条记录必须以加粗的模块标签（Scope）开头（如 `**[Alist2Strm]**`、`**[Config]**`），以便用户快速定位关心的功能。

---

## 2. 文档结构 (Document Layout)

`CHANGELOG.md` 包含以下四个部分，自上而下严格按序排布：

```markdown
# 更新日志

本项目的所有重大变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
并且本项目遵循 [语义化版本 (Semantic Versioning)](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### ✨ 新增
- ...

## [2.0.0] - 2026-07-23

### ⚠️ 破坏性变更
- ...

### ✨ 新增
- ...

<!-- 历史版本按时间正序排列（从老到新，末尾为最新的 [Unreleased]） -->

<!-- 底部对比链接 -->
[Unreleased]: https://github.com/AkimioJR/AutoFilm/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.1...v2.0.0
```

---

## 3. 分类标准 (Change Categories)

每个版本（或 `[Unreleased]`）下，使用二级标题对变更进行归类。**仅保留该版本实际包含变更的分类**，推荐排布顺序如下：

| 分类标题 | 对应性质 | 关联 Git Commit Type | 说明与示例 |
| :--- | :--- | :--- | :--- |
| `### ⚠️ 破坏性变更` | 破坏向下兼容性的变更 | 带有 `!` 或 `BREAKING CHANGE` | 配置文件结构变动、弃用老旧行为、Rust 重构等 |
| `### ✨ 新增` | 全新功能或扩展 | `feat` | 新增模块、新增配置字段、新增命令参数 |
| `### ⚡ 优化` | 性能提升、逻辑优化与体验改进 | `perfect`, `perf`, `refactor` | 内存占用降低、并发速度提升、减少冗余请求 |
| `### 🔄 变更` | 现有功能行为微调（向下兼容） | `update`, `refactor` | 默认参数调整、底层实现更换但接口兼容 |
| `### 🐛 修复` | Bug 修复与异常处理 | `fix` | 崩溃修复、序列化异常修复、路径转义修复 |
| `### 🗑️ 移除` | 彻底移除的功能或配置 | `refactor` | 删除之前已弃用的特性或废弃文件 |
| `### ⚠️ 弃用` | 计划在未来版本移除的特性 | - | 提前告知用户替代方案与迁移预期 |
| `### 🔒 安全` | 安全漏洞修复与权限加固 | `fix` | 修复凭据泄露风险、更新受攻击依赖 |
| `### 🔧 工程与维护` | CI/CD、Docker 构建、关键依赖升级 | `ci`, `build`, `chore` | 仅当对终端部署、运行或打包有明确影响时收录 |

---

## 4. 条目撰写规范 (Entry Formatting Rules)

### 4.1 格式模板
```markdown
- **[模块标签]**: 变更内容陈述。说明具体背景、预期行为或迁移提示。
```

### 4.2 AutoFilm 模块标签 (Scopes)
必须使用方括号加粗标识具体模块：
- `**[Alist2Strm]**`: STRM 生成、Alist 扫库、智能防误删、内外网地址分离、302 直链
- `**[Ani2Alist]**`: 动漫追番、RSS 订阅解析、时间格式转换、存储器映射更新
- `**[LibraryPoster]**`: 媒体库海报合成、封面样式绘制（collage / split）、多路径媒体库处理
- `**[MediaServer]**`: Emby / Jellyfin 媒体库刷新接口调用与交互
- `**[Schedule]**`: 后台 Cron / 异步任务调度器
- `**[Config]**`: `config.yaml` 配置结构解析、字段重命名、校验逻辑
- `**[Docker]**`: 镜像构建（Docker Hub / GHCR）、Dockerfile、compose 配置
- `**[CLI]**`: 命令行参数解析、`--help` 文档、启动参数
- `**[Logging]**`: 日志格式、轮转规则、终端与文件输出

### 4.3 语言与格式细节
1. **统一日期格式**: 严格使用 ISO 8601 `YYYY-MM-DD`（如 `2026-07-23`）。禁止使用 `2026.7.23`、`2026/07/23` 等非标准形式。
2. **标点与术语**:
   - 中文行文，语句以中文句号（`。`）结尾；分项说明可用分号或展开为二级列表。
   - 代码标识符、文件名、配置键名、参数名一律用反引号包裹，例如 `public_url`、`config.yaml`、`--dry-run`。
   - 专有名词大小写规范：`Rust`、`Docker`、`Docker Hub`、`GHCR`、`Alist`、`Emby`、`Jellyfin`。
3. **破坏性变更必须提供迁移说明**:
   遇到配置变更或行为不兼容时，条目下方必须以引用块注明迁移办法：
   ```markdown
   ### ⚠️ 破坏性变更
   - **[Config]**: 重构配置解析，使用 Rust 强类型结构替代原 Python 配置模型。
     > **迁移说明**: 配置文件结构变动较大，旧版 `config.yaml` 无法直接运行，请参照最新的 `config/config.example.yaml` 重新配置。
   ```

---

## 5. 版本比对链接规范 (Comparison Links)

在 `CHANGELOG.md` 末尾统一列出 Git 比较链接（Markdown 引用式链接），确保点击版本号即可直接查看 GitHub 上的代码差异：

```markdown
[Unreleased]: https://github.com/AkimioJR/AutoFilm/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.1...v2.0.0
[1.5.1]: https://github.com/AkimioJR/AutoFilm/compare/v1.5.0...v1.5.1
[1.5.0]: https://github.com/AkimioJR/AutoFilm/compare/v1.4.0...v1.5.0
...
[1.0.0]: https://github.com/AkimioJR/AutoFilm/releases/tag/v1.0.0
```

> **注意**: 首个版本没有更早的 Tag 可供比较，链接指向该版本的 Release 页面或 Commit。

---

## 6. 日常维护工作流 (Maintenance Workflow)

### 6.1 开发期间（针对日常提交）
- 在完成功能开发或修复后，在 `CHANGELOG.md` 的 `## [Unreleased]` 下追加对应分类记录。

### 6.2 版本发布时（针对新 Release）
1. 在 `Cargo.toml` 中确认版本号已更新（如 `2.1.0`）。
2. 将 `## [Unreleased]` 下的所有内容剪切到新的版本标题下：
   ```markdown
   ## [2.1.0] - 2026-10-25
   ```
3. 在顶部重新保留一个空的 `## [Unreleased]` 占位区域。
4. 更新底部的对比链接：
   - 将 `[Unreleased]` 更新为指向 `v2.1.0...HEAD`。
   - 新增 `[2.1.0]` 指向 `v2.0.0...v2.1.0`。

### 6.3 从老旧 README 迁移历史记录的方法
当把旧版 `README.md` 的简陋更新日志迁移至 `CHANGELOG.md` 时，按以下步骤重构：
1. **拆解复合句**: 将原本用逗号或分号拼接在单行的一揽子变更拆分为独立条目。
2. **规范分类**: 根据变更意图归入 `✨ 新增`、`⚡ 优化`、`🐛 修复` 或 `⚠️ 破坏性变更`。
3. **补齐 Scope**: 为每条日志打上对应的 `**[模块名]**`。
4. **规范化日期**: 将 `2026.7.23` 形式统一转为 `2026-07-23`。
5. **精简 README**: 在 `README.md` 中移除长列表，改写为：
   ```markdown
   # 更新日志
   详细的更新记录请参阅 [CHANGELOG.md](CHANGELOG.md)。
   ```

---

## 7. 范例对比 (Examples)

### ❌ 不规范示例 (原有 README 写法)
```markdown
- 2026.6.3：v1.5.1，Alist2Strm 新增 public_url 支持内外网地址分离；新增 .strm 智能保护防止大量误删；重构 HTTP 客户端设计；更新 Docker Python 版本与 Pillow 依赖；更新项目相关链接
```
*问题：单行挤压多个完全不同的模块与改动；使用句号/分号混合；非标准日期；缺乏层次与分类。*

### ✅ 规范化示例 (CHANGELOG.md 标准写法)
```markdown
## [1.5.1] - 2026-06-03

### ✨ 新增
- **[Alist2Strm]**: 新增 `public_url` 配置支持，实现内网直连与外网访问地址分离。
- **[Alist2Strm]**: 新增 `.strm` 文件智能保护机制，防止因远程目录临时下线或网络波动导致本地文件被误删。

### ⚡ 优化
- **[Core]**: 重构统一的 HTTP 客户端设计，提升网络请求连接复用率与重试稳定性。

### 🔧 工程与维护
- **[Docker]**: 升级基础镜像 Python 版本，并更新 `Pillow` 等依赖库。
- **[Docs]**: 更新项目文档与相关链接。
```
