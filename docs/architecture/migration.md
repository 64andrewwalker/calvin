# 迁移路径

## 新需求如何快速上线？

### 场景 1: 新增一个 Target (如 Gemini)

1. 在 `infrastructure/adapters/` 添加 `gemini.rs`
2. 实现 `TargetAdapter` trait
3. 在 `Target` 枚举中添加 `Gemini`
4. 完成!

**不需要修改**: Domain、Application、Presentation

### 场景 2: 新增一个命令 (如 `calvin init`)

1. 在 `presentation/commands/` 添加 `init.rs`
2. 在 `application/` 添加 `init_use_case.rs`
3. 完成!

**不需要修改**: Domain、Infrastructure

### 场景 3: 新增同步策略 (如 S3)

1. 在 `infrastructure/fs/` 添加 `s3.rs`
2. 实现 `FileSystem` trait
3. 完成!

**不需要修改**: Domain、Application、Presentation

## 迁移阶段

**阶段 1: 建立骨架** (1-2 天)
- 创建新目录结构
- 定义核心 traits (Ports)
- 保持旧代码可用

**阶段 2: 提取 Domain** (2-3 天)
- 将纯业务逻辑提取到 `domain/services/`
- 定义 Entities 和 Value Objects

**阶段 3: 提取 Infrastructure** (2-3 天)
- 将 I/O 操作移到 `infrastructure/`
- 实现 Ports

**阶段 4: 重写 Application** (1-2 天)
- 用 Use Cases 替代 Runner
- 依赖注入

**阶段 5: 清理 Presentation** (1 天)
- 统一 UI 输出
- 移除命令中的业务逻辑

## 与当前架构的对比

| 维度 | 当前 | v2 |
|------|------|-----|
| 依赖方向 | 混乱 | 单向 |
| 测试 | 需要 mock 文件系统 | 每层可独立测试 |
| 新增 Target | 需要修改多处 | 只需实现 trait |
| 上帝对象 | SyncEngine (1600行) | 拆分为多个 Service |
| 耦合 | cmd.rs 直接调用 fs | 通过 Port 解耦 |

