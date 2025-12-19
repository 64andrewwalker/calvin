# 架构概览

> **Status**: Design Proposal

## 设计目标

1. **明确分层**：每层只依赖下层，不依赖上层
2. **单向依赖**：避免循环依赖
3. **快速扩展**：新增 Target/命令 不需要修改核心逻辑
4. **易于测试**：每层可以独立 mock 测试

## 核心洞察

**Calvin 的本质是什么？**

```
Assets → Outputs → Files
(源)     (中间)    (目标)
```

Calvin 是一个 **编译器 + 部署器**：
- **编译**：将人类友好的资产格式转换为工具特定格式
- **部署**：将编译结果安全地写入目标位置

## 依赖规则

```
Presentation → Application → Domain ← Infrastructure
```

**关键点**：
- Presentation 依赖 Application
- Application 依赖 Domain
- Infrastructure **实现** Domain 定义的接口
- Domain **不依赖** Infrastructure（通过接口反转）

