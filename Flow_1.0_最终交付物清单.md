# Flow 1.0 最终交付物清单

> 本清单描述 Flow 作为一门可以正式发布、安装、学习、开发和发布应用的完整编程语言，最终必须交付给用户和开发者的成品。
>
> **这里描述的是最终交付物，不是源码仓库目录。**

## 1. Flow 编译器软件

这是 Flow 最核心的一级交付物。

### 1.1 统一命令行工具

```text
flow
```

用户通过它完成：

```bash
flow new
flow check
flow run
flow build
flow test
flow fmt
flow doc
flow package
flow publish
```

### 1.2 核心编译器

```text
flowc
```

负责：

```text
Source
↓
Lexer
↓
Parser
↓
AST
↓
HIR
↓
Name Resolution
↓
Type Check
↓
VIR / Value Flow
↓
Ownership Analysis
↓
Escape Analysis
↓
Capture Analysis
↓
Semantic Summary
↓
MIR
↓
Optimization
↓
Codegen
↓
Native Object
↓
Link
```

### 1.3 编译器必须具备

- 类型检查
- Ownership / Move / Copy
- Borrow / Compiler View
- Escape Analysis
- Closure Capture Analysis
- Value Flow Graph
- VIR
- Function Semantic Summary
- Summary Composition
- 增量编译
- 内容寻址缓存
- 诊断系统
- Quick Fix 信息
- Debug 信息
- Source Map
- 多平台 Codegen
- FFI / ABI 支持

## 2. Flow 标准库

正式交付：

```text
Flow Standard Library
```

第一版至少包含：

- Core
- Collections
- String
- File
- Path
- HTTP
- JSON
- SQLite
- CSV
- ZIP
- Time
- Crypto
- Clipboard
- Process
- Logging
- Configuration
- Concurrency
- Channel
- Task

每个标准库 API 都必须明确：

- API 签名
- 参数
- 返回值
- 错误类型
- 阻塞 / 非阻塞
- 异步 / 同步
- Cancellation
- 并发安全
- Send / Sync
- 资源生命周期
- 是否产生分配
- 平台差异

## 3. Flow Runtime

Flow 的最小原生运行时。

负责：

- Future
- Task
- Scheduler
- Cancellation
- Channel
- Async I/O
- UI Domain
- State Runtime
- Component Identity
- Event Dispatch
- 平台运行时适配

原则：

> Flow Runtime 不是 JVM、Python VM 一类的大型解释运行时。

目标是：

- 原生编译
- 可裁剪
- 按需链接
- 尽量消除未使用功能
- 小型应用尽可能低运行时开销

## 4. Flow UI Framework

第一方声明式 UI 框架。

核心能力：

- `ui`
- `render`
- `@state`
- Props
- Components
- Events
- Layout
- Text
- Image
- Window
- Dialog
- Animation
- Clipboard
- Drag & Drop
- Notification
- Hot Reload

核心运行模型：

```text
UI Event
↓
go{}
↓
HTTP / SQLite / File / IO
↓
Task Result
↓
ui { ... }
↓
State Update
```

必须支持：

- Windows
- Linux
- macOS
- Android
- iOS

第一阶段桌面默认渲染后端：

```text
Skia
```

## 5. Flow IDE

官方集成开发环境。

至少包含：

- 代码编辑器
- 自动补全
- 语法高亮
- 跳转
- 查找引用
- 重构
- Diagnostics
- Quick Fix
- Formatter
- 项目管理
- 包管理
- Build
- Run
- Test
- Publish
- UI Preview
- Hot Reload
- Language Veil

Flow IDE 必须突出 Flow 自身的诊断能力，例如：

```text
data
├── Ownership: Owned
├── Borrow: 当前安全借用
├── Copy: 0
└── Allocation: 0
```

以及：

```text
性能提示：

这里物化了约 103 MB 数据。

原因：
无法在当前分析范围证明安全借用。

建议：
1. 接受当前拥有值实现
2. 拆分函数
3. 使用高级借用 API
```

## 6. Flow Debugger

可独立提供，也可以完整集成到 Flow IDE。

至少支持：

- Breakpoint
- Step Into
- Step Over
- Step Out
- Call Stack
- Variables
- Watch
- Exception / Panic
- Task
- Async Call Stack
- UI State
- Thread
- Source Map

## 7. Flow Profiler

正式交付性能分析工具。

### CPU

- 函数耗时
- 调用热点
- 启动耗时

### Memory

- 内存占用
- 峰值
- 生命周期

### Allocation

- 分配次数
- 分配大小
- 大对象分配

### I/O

- 文件
- 网络
- SQLite

### Async / Task

- Task 创建
- Task 等待
- Cancellation
- 阻塞点

### UI

- UI 响应
- Render 时间
- State 更新
- Frame 性能

### Flow 专属

- Materialization
- Large Copy
- Borrow 优化结果
- Allocation Elimination

## 8. Flow Package Manager

正式提供：

```text
flowpkg
```

负责：

- 依赖解析
- 版本管理
- `flow.toml`
- `flow.lock`
- Workspace
- Feature
- Target
- Cache
- Offline Build
- Mirror
- Registry

最终配套：

```text
Flow Registry
```

用于发布和获取第三方包。

## 9. Flow Build & Publish Toolchain

正式提供：

```text
flow build
flow package
flow publish
```

支持：

```text
single
split
bundle
portable
installer
```

并处理：

- 编译
- 链接
- 资源打包
- 图标
- 文件关联
- URL Scheme
- 签名
- 安装包
- 更新
- 差分更新
- 发布渠道
- Release Metadata

## 10. Flow FFI / Platform SDK

正式提供：

- C ABI
- CStr
- CString
- CArray
- ABI-safe Struct
- Native Library Linking
- Platform API
- `unsafe`
- 平台 SDK 适配

目标平台：

```text
Windows
Linux
macOS
Android
iOS
```

普通开发者尽量不需要直接接触裸指针。

## 11. Flow 用户手册

这是一级正式交付物，不是附属资料。

### 11.1 《The Flow Book》

面向完全没有 Flow 经验的普通开发者。

内容：

1. 安装 Flow
2. 第一个程序
3. 变量
4. 表达式
5. 函数
6. 控制流
7. `struct`
8. `enum`
9. `Option`
10. `Result`
11. 错误处理
12. 集合
13. 文件
14. JSON
15. SQLite
16. HTTP
17. UI
18. Task
19. 项目组织
20. 构建
21. 打包
22. 发布

原则：

> **前半部分不要求用户学习生命周期、Borrow Checker 或生命周期参数。**

## 12. Flow Language Reference

正式语言参考手册。

用于定义：

- 词法
- Grammar
- 类型系统
- 类型推导
- Evaluation Order
- Ownership
- Move / Copy
- Borrow
- Compiler View
- Value Flow
- Closure Capture
- Escape
- `defer`
- `unsafe`
- Generics
- Trait
- Modules
- Task
- Future
- UI Domain
- FFI

这是：

> **Flow 语言行为的正式规范。**

## 13. Flow Standard Library Reference

标准库 API 参考手册。

每个 API 必须包含：

```text
签名
参数
返回值
错误
同步/异步
阻塞性
取消
Send / Sync
资源生命周期
分配行为
平台差异
示例
```

## 14. Flow UI Guide

专门讲 UI。

包括：

- `ui`
- `render`
- `@state`
- Component
- Props
- Events
- Layout
- Window
- Dialog
- Animation
- Hot Reload
- Async UI
- UI Domain
- State Management
- Desktop
- Mobile

重点解释：

```text
UI Event
↓
Task
↓
IO
↓
ui {}
↓
State
```

## 15. Flow Concurrency Guide

专门讲并发。

包括：

- `go{}`
- `go!{}`
- Future
- Task
- Channel
- Cancellation
- Send
- Sync
- Structured Concurrency
- Detached Task
- Async I/O
- UI Task Interaction

原则：

> **先教怎么用，再解释内部安全机制。**

## 16. Flow Advanced Programming Guide

给高级开发者。

包括：

- `&T`
- `&mut T`
- `move`
- 零拷贝
- Compiler View
- 性能优化
- 内存布局
- FFI
- ABI
- `unsafe`
- 平台 API
- 高级并发
- 高级泛型

普通开发者不需要先阅读本手册。

## 17. Flow Package & Build Guide

专门解释：

- `flow.toml`
- `flow.lock`
- Dependency
- Workspace
- Feature
- Target
- Cache
- Incremental Build
- Cross Compilation
- `single`
- `split`
- `bundle`
- `portable`
- `installer`
- Signing
- Update
- Delta Update

## 18. Flow Platform Guides

至少：

```text
Windows Guide
Linux Guide
macOS Guide
Android Guide
iOS Guide
```

内容：

- 权限
- 系统 API
- 文件系统
- 通知
- 窗口
- URL Scheme
- 文件关联
- 签名
- 平台发布
- 商店发布

## 19. Flow Cookbook / Examples

官方必须提供完整、可以直接运行的真实项目。

至少：

- Hello World
- Counter
- 记账本
- 下载器
- Markdown 编辑器
- SQLite 数据库工具
- OCR 工具
- 图片管理器
- 文件管理器
- HTTP 客户端
- AI 客户端
- 数据处理工具

目标：

> 用户可以直接从完整项目开始，而不是先读完语言理论。

## 20. Flow Compiler Internals

面向：

- 编译器贡献者
- 高级开发者
- 编译器研究人员

内容：

- AST
- HIR
- VIR
- Value Flow Graph
- Ownership Analysis
- Escape Analysis
- Capture Analysis
- Semantic Summary
- Summary Composition
- MIR
- Optimization
- Codegen
- Incremental Compilation
- Cache
- Diagnostics
- Source Mapping

## 21. Flow Specification

正式规范集合：

```text
Language Specification
Type System Specification
Ownership Specification
Value Flow Specification
Semantic Summary Specification
Concurrency Specification
UI Specification
FFI Specification
Package Specification
Build Specification
Diagnostic Specification
```

## 22. Flow Conformance Test Suite

正式一致性测试。

至少覆盖：

- Lexer
- Parser
- Type System
- Ownership
- Borrow
- Compiler View
- Value Flow
- Semantic Summary
- Cross-module Analysis
- Closure
- Concurrency
- UI
- FFI
- Standard Library
- Diagnostics
- Incremental Build
- Packaging

目的：

> 保证 Flow 实现严格符合正式规范。

# Flow 1.0 最终交付物总表

| # | 交付物 | 必须 |
|---|---|---|
| 1 | **Flow 编译器软件** | ✅ |
| 2 | **Flow 标准库** | ✅ |
| 3 | **Flow Runtime** | ✅ |
| 4 | **Flow UI Framework** | ✅ |
| 5 | **Flow IDE** | ✅ |
| 6 | **Flow Debugger** | ✅ |
| 7 | **Flow Profiler** | ✅ |
| 8 | **Flow Package Manager / Registry** | ✅ |
| 9 | **Flow Build / Publish Toolchain** | ✅ |
| 10 | **Flow FFI / Platform SDK** | ✅ |
| 11 | **The Flow Book 用户手册** | ✅ |
| 12 | **Language Reference** | ✅ |
| 13 | **Standard Library Reference** | ✅ |
| 14 | **UI Guide** | ✅ |
| 15 | **Concurrency Guide** | ✅ |
| 16 | **Advanced Programming Guide** | ✅ |
| 17 | **Package & Build Guide** | ✅ |
| 18 | **Platform Guides** | ✅ |
| 19 | **Cookbook / Examples** | ✅ |
| 20 | **Compiler Internals** | ✅ |
| 21 | **Formal Specifications** | ✅ |
| 22 | **Conformance Test Suite** | ✅ |

# Flow 1.0 的完成定义

Flow 1.0 不能只以：

> “编译器能够编译 Flow 代码”

作为完成标准。

完整完成必须意味着：

```text
Flow Source
      ↓
Flow Compiler
      ↓
Standard Library
      ↓
Runtime
      ↓
UI Framework
      ↓
Native Build
      ↓
Package / Sign / Publish
      ↓
最终应用
```

与此同时，开发者能够通过：

```text
The Flow Book
Language Reference
Standard Library Reference
UI Guide
Concurrency Guide
Advanced Guide
Build Guide
Platform Guides
Cookbook
```

从零开始学习并完成真实应用。

最终目标是：

> **Flow 不只是“一个编译器项目”，而是一套可以独立交付给开发者使用的完整编程语言产品。**
