# Calvin v0.2.0 设计原则

> **版本**: v0.2.0  
> **日期**: 2025-12-17  
> **状态**: Draft

---

## 核心理念

```
+==============================================================+
|                                                              |
|   交互即教程，教程结束配置也完成了                              |
|                                                              |
|   用户不需要读文档，通过使用工具本身学会使用工具                  |
|                                                              |
+==============================================================+
```

---

## 设计原则

### DP-1: 用户不会读文档

```
假设: 用户会先阅读 README 和文档
现实: 99% 的用户直接输入 calvin 或 calvin --help
```

**结论**:
- 文档是给搜索引擎和 AI 的，不是给新用户的
- CLI 本身必须是最好的教程
- 每一步交互都要传递知识

**验证方式**: 新用户能否在不读任何文档的情况下完成首次配置

---

### DP-2: 渐进式复杂度

```
+------------------+------------------------------------------+
| 用户类型          | 体验                                      |
+------------------+------------------------------------------+
| 首次用户          | 回答 3 个问题，工具开始工作                  |
| 重复用户          | 直接运行，记住上次选择                       |
| 高级用户          | 使用参数完全跳过交互                         |
| AI 辅助          | --json / --explain 让 AI 帮用户配置         |
+------------------+------------------------------------------+
```

**实现方式**:
1. 无参数运行 `calvin` → 交互式菜单
2. `calvin deploy` → 使用配置文件默认值
3. `calvin deploy --yes --targets claude-code` → 完全非交互

---

### DP-3: 命令即概念

```
旧设计: sync vs install vs doctor vs audit
        用户需要理解 4 个概念和它们的区别

新设计: deploy vs check
        部署 vs 检查，概念清晰，无歧义
```

**命令职责**:
| 命令 | 职责 | 替代 |
|------|------|------|
| `calvin` | 交互式入口 | (新增) |
| `calvin deploy` | 部署到任意目标 | sync, install |
| `calvin check` | 验证配置 | doctor, audit |
| `calvin watch` | 监听模式 | (保留) |
| `calvin explain` | AI 自述 | (新增) |

---

### DP-4: 错误即教程

```
错误不应该只告诉用户"出错了"，应该告诉用户：
1. 什么出错了（问题）
2. 为什么出错（原因）
3. 怎么修复（解决方案）
4. 去哪里了解更多（可选）
```

**示例**:
```
[ERROR] .promptpack/actions/review.md
        Line 2: YAML parsing error

Your file:

  1 | ---
  2 | description: My: Review  <-- The colon breaks YAML
  3 | ---

FIX: Wrap strings with colons in quotes:
     description: "My: Review"
```

---

### DP-5: AI 可理解

```
设计目标: AI 助手（Claude/Cursor/Antigravity）可以帮用户配置 Calvin

实现方式:
1. calvin explain        人类可读的自述
2. calvin explain --json 机器可读的结构化数据
3. 所有命令支持 --json   可解析的输出
```

**AI 配置流程**:
```bash
# AI 运行以下命令帮用户配置
calvin explain --json                    # 了解工具
mkdir -p .promptpack/actions             # 创建目录
cat > .promptpack/actions/example.md     # 创建文件
calvin deploy --yes                      # 部署
```

---

### DP-6: 字符画优先

```
原则: 终端界面使用纯 ASCII 字符，保证等宽对齐

列表选项:
  > [x] Option A    Description
    [ ] Option B    Description
    [ ] Option C    Description

边框使用 +---+ 或 ====:
  +----------------------------------------------+
  |  Title                                       |
  +----------------------------------------------+

状态标记:
  [OK]   成功
  [!!]   警告/冲突
  [ERROR] 错误
  [WARN]  警告

Emoji 仅用于:
  - 命令开头的图标提示（非必需信息）
  - 成功/失败的大状态
```

---

### DP-7: 配置即记忆

```
用户的选择应该被记住，下次自动应用

首次交互:
  - 选择目标平台 -> 保存到 config.toml
  - 选择安全模式 -> 保存到 config.toml
  - 选择示例模板 -> 创建对应文件

后续运行:
  - 直接使用上次配置
  - 无需再次选择
```

**配置文件结构**:
```toml
# .promptpack/config.toml
[defaults]
targets = ["claude-code", "cursor"]
security_mode = "balanced"
skip_git_warning = true
```

---

### DP-8: 幂等操作

```
原则: 相同输入，相同输出，运行多次无副作用

calvin deploy 的幂等性:
  - 内容相同 -> 不重写文件
  - 内容不同 -> 覆盖（或提示冲突）
  - 用户手动修改 -> 检测并提示
```

---

### DP-9: 安全是默认值

```
默认安全:
  - 自动屏蔽 .env, *.key, *.pem, id_rsa, .git/
  - balanced 模式作为默认

可配置:
  - 用户可以选择 minimal 模式自行管理
  - 通过 config.toml 完全控制
```

---

### DP-10: 迁移友好

```
对现有用户:
  - 旧命令继续工作（带 deprecation warning）
  - 提供清晰的迁移映射
  - v0.3.0 才移除旧命令

映射关系:
  calvin sync             -> calvin deploy
  calvin install --global -> calvin deploy --home
  calvin doctor           -> calvin check
```

---

## 设计原则检查清单

在实现新功能时，检查以下问题：

- [ ] 新用户能否在不读文档的情况下使用？(DP-1)
- [ ] 是否支持渐进式复杂度？(DP-2)
- [ ] 命令名称是否清晰表达意图？(DP-3)
- [ ] 错误信息是否包含修复建议？(DP-4)
- [ ] 是否支持 --json 输出？(DP-5)
- [ ] 终端输出是否使用 ASCII 字符对齐？(DP-6)
- [ ] 用户选择是否被记住？(DP-7)
- [ ] 操作是否幂等？(DP-8)
- [ ] 默认值是否安全？(DP-9)
- [ ] 是否向后兼容？(DP-10)

---

## 相关文档

- [产品重构方案](./product-reflection.md) - 完整的交互设计
- [TODO](./TODO.md) - 实现任务清单
