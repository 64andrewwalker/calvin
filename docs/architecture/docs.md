# 文档架构

> **Status**: Planned (website/ 目录尚未创建)
> **Updated**: 2025-12-20

## 文档即代码 (Docs-as-Code)

```
┌─────────────────────────────────────────────────────────────┐
│                      代码仓库                                │
│                                                             │
│  src/               ← Rust 代码                             │
│  docs/              ← 设计文档、RFC、ADR                     │
│  website/           ← Docusaurus 文档站点                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
          │
          ▼ (CI: build + deploy)
┌─────────────────────────────────────────────────────────────┐
│  https://calvin.docs.example.com                            │
│  (Docusaurus 静态站点)                                       │
└─────────────────────────────────────────────────────────────┘
```

## 设计原则

**核心洞察**：Calvin 遵循"工具即教程"原则

这意味着：
- CLI 本身是教程（`--help`、交互式菜单）
- 用户来看文档 = 遇到了**复杂问题**或**搞不懂的问题**
- 不需要写"入门教程"——CLI 自己就是教程

**两种受众，两种设计**：

| 受众 | 阅读方式 | 设计目标 | 信息密度 |
|------|----------|----------|----------|
| **人类用户** | 线性阅读、搜索 | 解决复杂问题 | 中等，有上下文 |
| **AI Agent** | 索引、定位 | 快速找到 API 细节 | 高密度，结构化 |

## 目录结构

```
docs/                      # 开发者文档 (与代码同仓库)
├── architecture/          # 架构文档
├── design/                # 设计文档
├── implementation/        # 实现计划
└── tdd/                   # TDD 会话记录

website/                   # Docusaurus 用户文档
├── docs/
│   ├── concepts/          # FOR HUMANS: 概念解释
│   ├── guides/            # FOR HUMANS: 问题解决指南
│   ├── api/               # FOR AI AGENTS: API 参考
│   └── contributing/      # FOR CONTRIBUTORS
└── ...
```

## 文档同步策略

| 类型 | 位置 | 同步方式 | 受众 |
|------|------|----------|------|
| 用户指南 | `website/docs/guides/` | 手动 | 最终用户 |
| CLI 参考 | `website/docs/api/cli.md` | **自动生成** | 最终用户/Agent |
| 配置参考 | `website/docs/api/config.md` | **自动生成** | 最终用户/Agent |
| 适配器参考 | `website/docs/api/adapters.md` | **自动生成** | 最终用户/Agent |
| 架构文档 | `docs/architecture/` | 手动 + 链接检查 | 贡献者 |

## 架构与文档的映射

```
代码层                          文档
──────────────────────────────────────────────────────
Presentation (CLI)      ←→     CLI Reference (自动)
Application (UseCases)  ←→     User Guides (手动)
Domain (Services)       ←→     Architecture Docs (手动)
Infrastructure          ←→     Adapter Reference (自动)
  └─ Adapters           ←→     Adapter Docs (自动)
  └─ Config             ←→     Config Reference (自动)
```

## CI 工作流

```yaml
# .github/workflows/docs.yml
name: Docs
on:
  push:
    branches: [main]
    paths:
      - 'src/**'
      - 'docs/**'
      - 'website/**'

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # 生成 CLI 文档
      - name: Generate CLI docs
        run: cargo run -- --help > website/docs/api/cli.md
      
      # 生成 Config 文档
      - name: Generate Config docs
        run: scripts/generate-config-docs.sh
      
      # 检查文档一致性
      - name: Check docs consistency
        run: scripts/check-docs-sync.sh
      
      # 构建 Docusaurus
      - name: Build Docusaurus
        run: |
          cd website
          npm ci
          npm run build
      
      # 部署
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./website/build
```

