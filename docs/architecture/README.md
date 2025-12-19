# Calvin 架构

> **Created**: 2025-12-19  
> **Status**: Design Proposal

本目录包含 Calvin 的架构设计文档。

## 文档索引

| 文档 | 描述 |
|------|------|
| [overview.md](./overview.md) | 架构概览：设计目标与核心洞察 |
| [layers.md](./layers.md) | 分层架构详解 |
| [directory.md](./directory.md) | 目录结构规范 |
| [ports.md](./ports.md) | 接口定义 (Ports) |
| [platform.md](./platform.md) | 跨平台兼容层设计 (macOS/Linux/Windows) |
| [migration.md](./migration.md) | 迁移路径与对比 |
| [docs.md](./docs.md) | 文档架构设计 |

## 核心洞察

```
Assets → Outputs → Files
(源)     (中间)    (目标)
```

Calvin 是一个 **编译器 + 部署器**。

## 依赖规则

```
Presentation → Application → Domain ← Infrastructure
```

Domain 不依赖任何外部层，这是可测试性和可扩展性的关键。

