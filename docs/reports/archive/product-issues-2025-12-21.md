# 产品问题分析报告

> **日期**: 2025-12-21  
> **分析视角**: 产品经理  
> **版本**: v0.2.0

---

## 执行摘要

Calvin 在技术实现上已经非常扎实（515 测试，75%+ 覆盖率，Clean Architecture 100% 完成），但从产品市场适配 (PMF) 角度看，存在一些关键问题影响用户获取和留存。

---

## 重大问题 (P0)

### 1. 缺少 `calvin init` 命令

**问题描述**:
- 新用户需要手动创建 `.promptpack/` 目录结构
- 没有交互式向导帮助用户快速上手
- 高用户获取成本

**当前体验**:
```bash
# 用户现在需要这样开始
mkdir -p .promptpack/policies .promptpack/actions
touch .promptpack/policies/code-style.md
# 然后手动编写 frontmatter...
```

**期望体验**:
```bash
calvin init
# 交互式向导：
# ? 项目名称: my-project
# ? 目标平台: [x] Claude Code [x] Cursor [ ] VSCode
# ? 初始化示例资产: Yes
# ✓ Created .promptpack/
# ✓ Created .promptpack/policies/code-style.md
# ✓ Created .promptpack/actions/hello.md
# → Run `calvin deploy` to get started
```

**实现建议**:
1. 添加 `calvin init` 命令
2. 支持 `--template` 选项（minimal, standard, full）
3. 生成基础 config.toml 和示例文件
4. 考虑支持 `calvin init --from-existing` 从现有 .claude/.cursor 导入

**预估工作量**: 1-2 天

---

### 2. 安装说明与发布状态不一致

**问题描述**:
- README 提到 `brew install calvin`
- 但可能尚未发布到 Homebrew

**验证步骤**:
```bash
brew search calvin  # 确认是否存在
scoop search calvin # 确认 Windows 支持
cargo install calvin --version 0.2.0 # 确认 crates.io 状态
```

**解决方案**:
1. 完成包管理器发布流程
2. 或更新 README 说明当前安装方式
3. 添加 GitHub Actions 自动发布

---

## 高优先级问题 (P1)

### 3. 错误信息用户友好度

**问题描述**:
- 错误信息偏技术性
- 缺少修复建议

**示例**:
```
# 当前
Error: Failed to parse YAML at line 5: expected ':', found '='

# 期望
Error: Invalid frontmatter syntax at line 5
  → YAML uses ':' for key-value pairs, not '='
  → Fix: Change 'scope=user' to 'scope: user'
  → Docs: https://calvin.dev/docs/frontmatter
```

**实现建议**:
1. 为常见错误定义错误码
2. 在错误消息中包含修复建议
3. 可选：`calvin explain <error-code>`

---

### 4. 缺少示例仓库/文档

**问题描述**:
- `examples/` 目录已删除（Git 历史清理）
- 用户没有参考示例

**解决方案**:
1. 创建独立的 `calvin-examples` 仓库
2. 或在文档中添加完整的示例章节
3. 包含常见场景：
   - 基础规则定义
   - Slash command 创建
   - 多平台配置
   - 远程同步设置

---

## 中优先级问题 (P2)

### 5. MCP 支持完整性

**spec.md 承诺**:
> `mcp/` 代表需要统一分发的 MCP server 定义与环境变量引用

**当前状态**:
- 目录结构支持 `.promptpack/mcp/`
- 但功能实现可能不完整
- 需要验证：
  - MCP 配置解析
  - 多平台 MCP 配置生成
  - Server allowlist 验证

**验证命令**:
```bash
grep -r "mcp" src/ --include="*.rs" | wc -l
```

---

### 6. `--home` vs `scope: user` 概念混淆

**问题描述**:
用户可能不理解：
- `scope: user` in frontmatter = 资产应该安装到用户目录
- `--home` flag = 强制所有资产安装到用户目录

**解决方案**:
1. 改进 CLI help 文本
2. 在文档中明确区分
3. 考虑更好的命名：`--force-user-scope` 或 `--global`

---

### 7. 用户反馈闭环

**问题描述**:
- 无遥测数据
- 不知道用户使用哪些功能
- 不知道用户遇到什么问题

**解决方案** (可选，需权衡隐私):
1. 可选的匿名使用统计
2. `calvin feedback` 命令
3. GitHub Issues 模板优化

---

## 低优先级问题 (P3)

### 8. 版本号管理

- 刚做了 breaking change（删除 sync/install/doctor/audit）
- 应该是 v0.3.0 而非 v0.2.0

### 9. CI/CD 自动发布

- 需要 GitHub Actions 自动发布到：
  - crates.io
  - Homebrew tap
  - Scoop bucket
  - GitHub Releases

---

## 产品路线图建议

### v0.3.0 (近期)
- [ ] `calvin init` 命令
- [ ] 改进错误信息
- [ ] 发布到包管理器
- [ ] 更新版本号

### v0.4.0 (中期)
- [ ] MCP 功能完善
- [ ] 示例仓库/文档
- [ ] CLI help 改进

### v0.5.0 (远期)
- [ ] 插件系统
- [ ] 性能优化
- [ ] 可选遥测

---

## 附录：产品强项

| 能力 | 状态 | 说明 |
|------|------|------|
| 多平台支持 | ✅ | Claude, Cursor, VSCode, Antigravity, Codex |
| 安全默认 | ✅ | 自动 deny list |
| Watch 模式 | ✅ | 文件变更自动同步 |
| 远程同步 | ✅ | SSH/rsync + scp fallback |
| 冲突检测 | ✅ | 不覆盖用户修改 |
| JSON 输出 | ✅ | CI 友好 |
| 跨平台 | ✅ | macOS/Windows/Linux |
| 零依赖 | ✅ | 1.2MB 静态二进制 |
| 测试覆盖 | ✅ | 515 测试，75%+ 覆盖 |
| 架构 | ✅ | Clean Architecture |

