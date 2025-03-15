# RepoActivity

一个用于分析Git仓库活动的Rust工具，可以检查提交历史并生成详细的统计信息。

## 功能特点

- 分析Git仓库的提交历史
- 统计提交次数、代码行变更和文件变更
- 按贡献者分析提交情况
- 支持时间范围过滤
- 将分析结果存储到SQLite数据库
- 使用异步处理和并行计算处理大型仓库

## 安装

确保您已安装Rust和Cargo。然后克隆此仓库并构建项目：

```bash
git clone https://github.com/yourusername/RepoActivity.git
cd RepoActivity
cargo build --release
```

## 使用方法

```bash
# 基本用法
cargo run -- --repo-path /path/to/git/repo

# 指定数据库路径
cargo run -- --repo-path /path/to/git/repo --db-path custom_database.db

# 按时间范围过滤
cargo run -- --repo-path /path/to/git/repo --start-date 2023-01-01 --end-date 2023-12-31
```

### 命令行参数

- `--repo-path, -r`: Git仓库的路径（必需）
- `--db-path, -d`: 数据库文件路径（默认：repo_activity.db）
- `--start-date, -s`: 分析的开始日期，格式为YYYY-MM-DD（可选）
- `--end-date, -e`: 分析的结束日期，格式为YYYY-MM-DD（可选）

## 技术实现

- 使用`git2`库访问Git仓库数据
- 使用`tokio`进行异步处理
- 使用`sqlx`进行数据库操作
- 使用`futures`库实现并行处理提交
- 使用`clap`解析命令行参数

## 数据库结构

分析结果存储在SQLite数据库中，包含以下表：

- `repositories`: 仓库级别的统计信息
- `contributors`: 按贡献者分组的统计信息
- `commits`: 每个提交的详细信息

## 许可证

本项目采用MIT许可证 - 详见[LICENSE](LICENSE)文件
