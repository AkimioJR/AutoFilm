---
name: git-atomic-commit
description: Plan and create atomic Git commits for larger workspace changes. Use when Codex executes a multi-step plan, prepares commits, splits staged changes, or reviews whether each commit is independently coherent and buildable.
---

# Git Atomic Commit

Use this skill when a task produces multiple logical changes or when preparing commits after a larger plan.

## Core Rule

Create small, atomic commits. Each commit should represent one coherent change and should be understandable, reviewable, and preferably buildable on its own.

## Commit Splitting

- Split unrelated changes into separate commits, even if they were implemented during the same plan.
- Group code, tests, fixtures, config, and docs only when they directly support the same logical change.
- Do not create a commit that depends on code introduced only by a later commit.
- Put prerequisites before dependents: types/helpers first, callers next, integration or polish after.
- Avoid one large "everything" commit when the work can be reviewed as independent steps.

## Buildability

- Prefer every commit to compile or build successfully.
- If a commit introduces a new call site, include the required function, type, import, feature flag, and configuration in the same commit or an earlier commit.
- If a temporary non-buildable commit is unavoidable, avoid leaving it in the final history; squash or reorder before presenting the result.
- When validation is available, run the narrowest relevant build/test after each commit-sized unit or before committing that unit.

## Workflow

1. Inspect the working tree with `git status --short` and identify unrelated file groups.
2. Review diffs with `git diff` and decide the intended commit sequence before staging.
3. Stage only one logical unit at a time using pathspecs or interactive staging.
4. Verify the staged diff with `git diff --cached`.
5. Commit with a message that follows the workspace commit message convention.
6. Repeat until all intended changes are committed, leaving unrelated user changes untouched.

## Examples

Good split:

- Commit A: `feat(Config): 添加 alist2strm 配置结构`
- Commit B: `feat(Alist2Strm): 实现 strm 生成流程`
- Commit C: `test(Alist2Strm): 覆盖路径转换逻辑`

Bad split:

- Commit A calls `build_strm_path()` but does not define it.
- Commit B defines `build_strm_path()` afterward.
- Commit C mixes config parsing, runner refactor, README edits, and unrelated formatting.
