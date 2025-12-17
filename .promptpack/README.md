# Example PromptPack

这是一个完整的 PromptPack 示例，包含 36 个实际可用的开发工作流。

## 结构

```
.promptpack/
├── 0-discovery/          # 发现阶段 - 项目分析、文档同步
├── 1-planning/           # 规划阶段 - 架构设计、技术选型
├── 2-init/               # 初始化阶段 - CI/CD、容器化、i18n
├── 3-development/        # 开发阶段 - 重构、迁移、错误处理
├── 4-testing/            # 测试阶段 - TDD、回归、变体测试
├── 5-qa/                 # 质量保证 - 安全、性能、可访问性审计
├── 6-release/            # 发布阶段 - Changelog、文档整理
├── m-maintenance/        # 维护 - 代码清理、依赖审计、Git 审计
└── x-utility/            # 工具 - 调试、工作树审查
```

## 工作流清单

| 阶段 | 数量 | 描述 |
|------|------|------|
| **0-discovery** | 2 | 项目分析、文档/代码同步检查 |
| **1-planning** | 6 | API 版本策略、缓存策略、数据库优化、前端重构、项目初始化、架构重设计 |
| **2-init** | 6 | CI/CD 设置、容器化、环境配置、i18n、Monorepo、可观测性 |
| **3-development** | 4 | 错误处理、遗留代码现代化、框架迁移、代码重构 |
| **4-testing** | 6 | 关键路径测试、Bug 修复测试、完整测试、回归测试、TDD、变体测试 |
| **5-qa** | 5 | 可访问性审计、API 审查、依赖审计、性能分析、安全审计 |
| **6-release** | 2 | Changelog 生成、文档整理 |
| **m-maintenance** | 3 | 代码库清理、依赖清理、Git 审计 |
| **x-utility** | 2 | 异常调试、工作树审查 |

## Frontmatter 格式

每个文件只需要一个必填字段：

```yaml
---
description: 工作流描述
---
```

## 使用方式

编译后会生成各平台的命令：

```bash
# Claude Code
/0-disc-analyze-project

# Antigravity
/0-disc-analyze-project

# Cursor
/0-disc-analyze-project
```
