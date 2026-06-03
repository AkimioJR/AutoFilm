---
name: git-commit-message
description: Write or review Git commit messages for this workspace. Use when Codex needs to suggest, create, validate, or adjust commit messages for AutoFilm changes.
---

# Git Commit Message

Use this skill when writing or reviewing commit messages in this workspace.

## Format

Use one of these forms:

- `type: description`
- `type(scope): description`

Allowed `type` values:

- `feat`: new behavior or capability
- `perfect`: polish or improve existing behavior without changing its core purpose
- `fix`: bug fix
- `refactor`: code structure change without intended behavior change
- `style`: formatting, UI style, or non-functional presentation changes
- `docs`: documentation-only change
- `test`: tests or test fixtures
- `update`: dependency, config, version, or routine maintenance update

## Scope

Add a scope when it makes the affected area clearer. Prefer concise module or feature names already used in the repository, such as:

- `Alist2Strm`
- `Config`
- `Runner`
- `Main`

Omit the scope for broad or simple changes.

## Description

- Write the description in Chinese or English; Chinese is acceptable when clearer.
- Keep the first line short and specific, ideally no more than about 50 Chinese characters or 72 English characters.
- Describe what changed, not the implementation struggle.
- Do not add trailing punctuation unless it improves clarity.

## Examples

- `feat(Alist2Strm): 支持 from_str 配置解析`
- `fix(Config): 修复默认路径读取异常`
- `perfect: 优化任务运行日志输出`
- `refactor(Runner): 拆分任务执行流程`
- `docs: 更新 alist2strm 使用说明`
