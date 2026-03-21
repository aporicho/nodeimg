# Git 协作工作流程

## 整体流程

```
主分支 (main)
    │
    ├─── 创建 Issue（明确任务）
    │         在 GitHub 页面新建 Issue
    │         描述要做什么、为什么做
    │         → 自动分配编号，如 #12
    │
    ├─── 克隆/拉取最新代码
    │         git clone <仓库地址>
    │         git pull origin main
    │
    ├─── 创建功能分支（关联 Issue 编号）
    │         方式 A：在当前目录切换分支
    │           git checkout -b feature/12-xxx
    │
    │         方式 B：用 worktree 在独立目录中开发（不影响当前工作）
    │           git worktree add ../project-12-xxx feature/12-xxx
    │           cd ../project-12-xxx
    │           → 适合：正在开发别的功能时，需要并行处理新任务
    │
    ├─── 开发 & 提交（提交信息引用 Issue）
    │         git add .
    │         git commit -m "feat: 添加xxx功能 #12"
    │         （可多次提交）
    │
    ├─── 推送到远程
    │         git push origin feature/12-xxx
    │
    ├─── 发起 Pull Request (PR)（关闭 Issue）
    │         在 GitHub/GitLab 页面
    │         选择：feature/12-xxx → main（必须是默认分支，关键词才生效）
    │         描述中写：Closes #12 / Fixes #12 / Resolves #12
    │         → PR 合并后，Issue #12 自动关闭
    │
    ├─── Code Review
    │         同事查看代码，留下评论
    │         ├── 有问题 → 继续修改，push，PR 自动更新
    │         └── 通过 → Approve（批准）
    │
    ├─── 合并（Merge）
    │         维护者或开发者点击 "Merge PR"
    │         feature/12-xxx 的代码进入 main
    │         Issue #12 自动关闭 ✓
    │
    └─── 清理（可选）
              git branch -d feature/12-xxx
              git push origin --delete feature/12-xxx
              如果用了 worktree：
              git worktree remove ../project-12-xxx
```

---

## 常用命令速查

### 初始化 & 克隆

```bash
git init                        # 初始化本地仓库
git clone <url>                 # 克隆远程仓库
```

### 分支操作

```bash
git branch                      # 查看本地分支
git branch -a                   # 查看所有分支（含远程）
git checkout -b feature/xxx     # 创建并切换到新分支
git checkout main               # 切换回 main
git branch -d feature/xxx       # 删除本地分支
```

### Worktree（并行开发）

```bash
# 用已有分支创建 worktree
git worktree add ../project-feature feature/xxx
#                 ↑ 独立工作目录      ↑ 要检出的分支

# 创建 worktree 的同时创建新分支
git worktree add -b feature/new ../project-new

# 查看所有 worktree
git worktree list

# 完成后清理
git worktree remove ../project-feature
```

> **Worktree vs checkout**：`checkout` 在同一目录切换分支，一次只能在一个分支上工作；`worktree` 为每个分支创建独立目录，可以同时在多个分支上工作，且共享同一个 `.git` 仓库（提交历史互通、磁盘占用小）。

### 提交流程

```bash
git status                      # 查看当前状态
git add .                       # 暂存所有改动
git add <文件名>                 # 暂存指定文件
git commit -m "描述信息"        # 提交
git log --oneline               # 查看提交历史
```

### 同步远程

```bash
git pull origin main            # 拉取远程最新代码
git push origin feature/xxx     # 推送分支到远程
git fetch origin                # 拉取远程信息但不合并
```

### 合并 & 变基

```bash
git merge main                  # 将 main 合并到当前分支
git rebase main                 # 变基（保持线性历史）
```

---

## Issue 关闭关键词

在 PR 描述中使用以下关键词，合并后自动关闭 Issue：

| 关键词 | 适用场景 |
|--------|---------|
| `Closes #12` | 通用，功能完成 |
| `Fixes #12` | Bug 修复 |
| `Resolves #12` | 通用 |

> **注意**：关键词只在 PR 目标是默认分支（main）时才生效。合并到其他分支不会自动关闭 Issue。

关闭多个 Issue：
```
Closes #12, Fixes #15, Resolves #20
```

---

## Commit 信息规范

| 前缀 | 含义 |
|------|------|
| `feat:` | 新功能 |
| `fix:` | Bug 修复 |
| `docs:` | 文档更新 |
| `style:` | 格式调整（不影响逻辑） |
| `refactor:` | 代码重构 |
| `test:` | 添加测试 |
| `chore:` | 构建/工具等杂项 |

示例：
```
feat: 添加用户登录功能
fix: 修复首页图片加载失败问题
docs: 更新 README 安装说明
```

---

## PR 流程 vs 直接 Push 对比

| 直接 push 到 main | 走 PR 流程 |
|---|---|
| 无人审查，bug 直接上线 | 至少一人 review |
| 历史混乱 | 每个功能清晰可追溯 |
| 冲突难处理 | 分支隔离，冲突可控 |
| 无自动测试保障 | CI/CD 在 PR 阶段自动运行 |

---

## 冲突解决

```bash
# 拉取最新代码后出现冲突时
git pull origin main

# 打开冲突文件，手动选择保留哪部分
# 冲突标记示例：
# <<<<<<< HEAD
# 你的代码
# =======
# 别人的代码
# >>>>>>> origin/main

# 解决后重新提交
git add .
git commit -m "fix: 解决合并冲突"
```

> **Worktree 的优势**：使用 worktree 时，每个分支在独立目录中工作，不存在未提交代码被覆盖的风险。解决冲突时也不需要先 stash 当前工作——直接在对应的 worktree 目录中处理即可。

---

## 关键原则

- **每个功能/修复 = 一个独立分支 + 一个 PR**，互不干扰
- **PR 合并前，main 始终是经过审查的干净代码**
- **分支命名规范**：`feature/功能名`、`fix/问题描述`、`hotfix/紧急修复`
- **定期同步 main**：长期开发的分支要定期 `git rebase main`，避免大量冲突积累
- **多任务并行用 worktree**：需要同时处理多个分支时，优先用 `git worktree` 而非反复 stash/checkout；注意同一分支不能被两个 worktree 同时检出，完成后及时 `git worktree remove` 清理
