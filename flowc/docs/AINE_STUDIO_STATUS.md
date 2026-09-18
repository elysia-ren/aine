# Aine Studio 实现状态

> 最后更新: 与 git commit 同步（master HEAD）

## 概览

Aine Studio 是 Aine 语言的完整 IDE，Rust/egui 实现（`src/bin/aine_studio.rs` 约 4400 行）。
设计蓝图见 `AINE_IDE_ARCH.md`；本文记录**实际已实现/部分实现/未实现**的逐项状态。

## 编辑器

| 功能 | 状态 | 说明 |
|---|---|---|
| 语法高亮 | ✅ | 关键词/类型/字符串/注释/数字/搜索匹配高亮；hover 去重 |
| 括号匹配 | ✅ | 光标处括号对高亮（(){}[]，跳字符串注释，字节偏移精确） |
| 行号 | ✅ | 点击定位、Ctrl+右键断点（红●标记）、⛔⚠错误警告标记 |
| 自动缩进 | ✅ | 换行继承上一行缩进，`{`/`:`/`=>` 加一级 |
| Tab 键 | ✅ | 在光标处插入空格 |
| 多标签 | ✅ | 打开/关闭/✕按钮/脏标记●，会话恢复（重启恢复上次标签） |
| Language Veil 表面切换 | ✅ | 6 语言词表（zh/en/ja/de/fr/ru），显示层 render_to/编辑回写 canonical，查看(View)菜单+状态栏+双视图预览+canonical 导出 |
| 源映射 (T51) | ✅ | `veil::source_map()` 表面↔canonical 行级对齐 |
| 搜索替换 | ✅ | 实时匹配计数、下一个、全部替换、高亮 |
| Code Lens | ✅ | 首错提示条（错误消息+行号+跳转）；大纲面板含引用计数 |
| 大纲视图 | ✅ | 活动栏📋视图：符号列表（ƒ/S/E），点击跳行，check 后自动同步 |
| Go to Definition | ✅ | F12/Ctrl+右键/LSP 精确跳转 |
| Find References | ✅ | Shift+F12，PROBLEMS 面板显示 |
| F2 重命名 | ✅ | ide_rename 后端，改完自动 check |
| hover | ✅ | LSP 请求通道（光标静止 800ms 触发），同位置去重 |
| 自动缩进 | ✅ | 换行继承 + `{`/`:`/`=>` 加一级 |
| 真实光标追踪 | ✅ | 状态栏 Ln/Col 实时、当前行号高亮 |
| 括号自动闭合 | ❌ | |
| 多光标/列选择 | ❌ | 需自绘内核 |
| Code Folding | ❌ | 需自绘内核 |
| Ctrl+滚轮缩放 | ❌ | |
| Indent Guides | ❌ | |

## 工作区

| 功能 | 状态 | 说明 |
|---|---|---|
| 文件树 | ✅ | 递归折叠(▼/▶)、缓存(TTL 2s)、新建/删除/刷新工具条、子目录 |
| Git | ✅ | Refresh/Diff/Stage All/Commit(消息输入)、status+diff |
| 全工程搜索 | ✅ | 匹配计数，点击跳文件+行 |
| 任务系统 | ✅ | 活动栏⚙视图：aine_tasks.json 自定义任务，点击后台执行 |
| 布局持久化 | ✅ | 侧栏宽度存 settings 重启恢复 |
| 会话恢复 | ✅ | 打开标签+活动标签存 settings 重启恢复 |
| 文件重命名 | ❌ | 后端 rename 已有，缺 UI 入口 |
| 文件监视 | ❌ | 外部增删不同步（手动 🔄 刷新） |
| 编辑器分屏 | ❌ | |

## 构建/运行/调试

| 功能 | 状态 | 说明 |
|---|---|---|
| Build (F7) | ✅ | aine build → zig cc 原生 exe，后台执行 |
| Run (F6) | ✅ | 运行构建产物，后台执行 |
| Check (F5) | ✅ | 全流水线诊断，去抖（同 revision 跳过），首错自动跳转 |
| 测试面板 | ✅ | aine test 聚合（5/5 通过实测） |
| 调试器 | ✅ | F9/命令面板：aine debug 断点+局部变量报告进终端 |
| 单步调试 | ❌ | step into/over/out/变量窗 |
| 任务系统 | ✅ | aine_tasks.json 自定义任务 |
| 格式化 | ✅ | aine fmt 输出规范源码 |

## AI 接入 (Umber Runtime)

| 功能 | 状态 | 说明 |
|---|---|---|
| 多厂商 Deployment | ✅ | models.toml 多 [[model]] 并存，独立 api_key/protocol/base_url |
| 5 协议透传 | ✅ | openai_chat / openai_responses / anthropic_messages / gemini / provider_native |
| T34 能力路由 | ✅ | 按 capabilities(chat/explain/fix/reason) 跨厂商选路 |
| catalog 模型选择器 | ✅ | 9624 deployments 惰性解析，搜索+点击选择 |
| 真流式 | ✅ | text_delta 增量渲染，reasoning_delta 思维链灰字，usage_updated token 用量 |
| 停止按钮 | ✅ | runtime_stream_cancel 并发取消 |
| 系统提示+文件上下文 | ✅ | 角色+当前文件名+前 6000 字符 |
| Markdown 渲染 | ✅ | ```代码块等宽深底框 |
| ai/explain | ✅ | 诊断→AI 解释→聊天面板 |
| ai/fix → Diff 审批 → 闭环 | ✅ | 诊断→AI→代码块提取→审批窗→Accept 应用→重编译→反馈 |
| Demo 模式 | ✅ | Umber 内置假流，无需 API Key |
| T39 Model Center | ✅ | models.toml 可视化编辑+持久化 |
| ai/ask (自由对话) | ✅ | |
| AI Task 步骤 UI (T42) | ❌ | |
| 会话持久化 | ❌ | |

## 架构

| 功能 | 状态 | 说明 |
|---|---|---|
| 异步任务层 | ✅ | spawn_task/poll_tasks，UI 零冻结 |
| 统一命令注册表 | ✅ | Cmd enum 单源（双语 label/shortcut/execute） |
| 通知 toast | ✅ | 右下角 4 秒过期 |
| 双实例检测 | ✅ | 锁文件+PID 存活检查 |
| 编码检测 | ✅ | BOM/UTF-16 拒绝/GBK 真解码（Win32 API） |
| check 去抖 | ✅ | 同 revision 跳过 |
| 命令面板模糊匹配 | ✅ | 子序列打分+Enter 执行 |
| LSP 诊断推送 | ✅ | aine lsp 子进程，didOpen/didChange 全文，publishDiagnostics 入面板 |
| LSP hover/completion/definition 请求 | ✅ | id 挂起匹配 |
| LSP 增量诊断 | ⚠️ | 全文 didChange（ane lsp 按全文处理） |
| Tooling Protocol 完整版 | ❌ | 目前直调（同进程），未来换子进程 |

## 语言特性

| 功能 | 状态 |
|---|---|
| 编译器 L1 自举 | ✅ |
| Language Veil 6 语言词表 | ✅ |
| 源映射 T51 | ✅ |
| UI 组件名冲突 N0001 警告 | ✅ |
| Vec.push 链式语义恢复 | ✅ |
| Option.unwrap 确认 | ✅ |

## 尚未开始

| 功能 | 归属 |
|---|---|
| E1 FFI (T44-47) | S0.5 |
| E3 多语言工程 (T54-60) | S0.7 |
| 编辑器自绘内核（折叠/多光标/Code Lens 行内） | 长期 |
| 全 Aine 化 (T64-68) | S4 |
| T42 AI Task 步骤 UI | S3 |
| 架构图渲染 (T41) | S3 |
| 调试器单步/变量窗 | S3 |
| LSP workspace symbol（本地已有，LSP 协议侧未注册） | S3 |
| accesskit 无障碍 | 低优先 |
| 主题切换 / About / 拖放打开 | 低优先 |

## 构建

```
cargo build --bin aine-studio
cargo run --bin aine-studio
```

依赖：eframe/egui 0.27 (wgpu)、umer_ffi.dll（AI 流式，可选）。
