# Flow 1.0 执行计划与完成定义

> **配套文档 2 / 3**
>
> 本计划把《Flow 语言完整设计方案定稿版》（V5.8）的 **Phase 0-7 技术路线** 与《Flow 1.0 最终交付物清单》的 **22 项交付物** 连接成一份可执行计划：
>
> - 交付物 × Phase 映射；
> - 1.0 的 MVP 边界（解决"22 项全部必须"导致的不可收敛问题）；
> - 每项交付物的完成定义（DoD）与验收方式；
> - 里程碑 M0-M8 与退出标准；
> - 风险主线（编译器分析器单列）；
> - 建议新增交付物（测试框架等）。
>
> 状态约定：**MVP（1.0 首个可发布版）** / **1.0 完整** / **1.1 后置**。

---

## 1. 交付物 × Phase 映射表

| # | 交付物 | 主依赖 Phase | 1.0 归属 | 备注 |
|---|---|---|---|---|
| 1 | flowc 编译器 | Phase 0-4 主线，Phase 6 codegen | MVP + 1.0 | 最大技术风险，见 §5 R1 |
| 2 | 标准库 | Phase 2 启动，Phase 3/4 扩充 | MVP（子集）+ 1.0 | MVP 子集：Core/Collections/String/File/JSON/SQLite/HTTP |
| 3 | Runtime | Phase 3 | MVP（最小）+ 1.0 | MVP 最小：Task/Scheduler/Cancellation/UI Domain |
| 4 | UI Framework | Phase 3（联合原型）/ Phase 5（完整） | MVP（最小）+ 1.0 | 见 D11：桌面优先，移动端 1.1 |
| 5 | IDE | Phase 4（LSP）/ Phase 7（Veil） | MVP 形态 = LSP + 编辑器扩展；独立 IDE 1.0 | 形态决策见 §6 |
| 6 | Debugger | Phase 4 | 1.0（独立或并入 IDE） | 可后置于编译器稳定 |
| 7 | Profiler | Phase 4/5 | 1.0 | 物化仪表盘（D2）为首要功能 |
| 8 | flowpkg / Registry | Phase 2 启动 / Phase 7 生态 | MVP（flowpkg 最小）；Registry 1.0 | Registry 冷启动见 §5 R4 |
| 9 | Build/Publish | Phase 6 | MVP（Windows 桌面）+ 1.0 | single/portable 先行 |
| 10 | FFI/Platform SDK | Phase 2/6 | 1.0 | C ABI 先行 |
| 11-19 | 文档体系 | 随各阶段并行 | Book 前半为 MVP；全套 1.0 | Book 是一级交付物，前置 |
| 20 | Compiler Internals | Phase 1 起持续 | 1.0 | 含 D6 精度不变量 |
| 21 | Specifications | Phase 1 起持续 | 1.0 | 与实现并行定稿 |
| 22 | Conformance | Phase 1 起持续 | MVP（核心子集）+ 1.0 | MVP：类型/Ownership/Value Flow/Summary |
| **23*** | **用户级测试框架（新增）** | Phase 2 起 | MVP（单元 + UI 最小） | 见 §6 |

> *23 为本次完善建议新增，用于补齐原清单"有 `flow test` 命令但无测试框架交付物"的缺口。

---

## 2. MVP 边界（Flow 1.0 首个可发布版）

> 原则：**MVP 必须能端到端做出一个真实桌面应用并发布**，其余能力按依赖后置。MVP 不是"半成品"，而是满足完成定义的第一个可对外版本。

### 2.1 MVP 包含

| 交付物 | MVP 范围 |
|---|---|
| flowc | 完整流水线（Lexer→…→Codegen）桌面 x64（Windows 优先）；Ownership/Value Flow/Summary 单模块 + 跨模块；增量 + 缓存；诊断 + Quick Fix 信息；§71.1/71.2/71.4 通过 |
| 标准库 | Core、Collections、String、File、Path、JSON、SQLite、HTTP、Time、Logging、Configuration |
| Runtime | Task、go{}/go!{}、Cancellation、Channel、UI Domain、State Runtime、Component Identity |
| UI Framework | 窗口、Text、Button、TextInput、List、Row/Column、@state、ui{}、事件、Hot Reload（开发期） |
| flowpkg | 最小：依赖解析、flow.toml/flow.lock、本地缓存、离线构建 |
| 测试框架（23） | 单元测试 + 最小 UI 测试（事件驱动断言） |
| 文档 | The Flow Book 第 1-15 章（到 SQLite/HTTP）、Language Reference 初版、UI Guide 初版 |
| Build/Publish | single / portable（Windows）；签名预留接口 |
| Conformance | 类型 / Ownership / Value Flow / Summary / 诊断 核心子集 |

### 2.2 MVP 验收（端到端）

用 §69 的**记账本示例**作为 MVP 验收应用，必须满足 §72 联合验收：
```text
0 dangling reference / 0 data race / 0 illegal state access
1 ownership transfer / 0 unnecessary full-data copy
```
加上：`flow new → flow run → flow build → flow package → 产出可安装/便携包` 全链路可用；一名无 Rust 经验的开发者按 Book 前半完成记账本的认知负担验收（§73）通过。

### 2.3 明确排除在 MVP 之外（1.0 完整或 1.1）

- 移动端 UI 后端（1.1，见 D11）；
- Debugger / Profiler 完整版（1.0，置于 MVP 之后）；
- Registry 线上运营、mirror、本地化投影（1.0-1.1）；
- 五平台同时发布（1.0 完整：Windows/Linux/macOS 桌面；移动端 1.1）。

---

## 3. 22+1 项交付物完成定义（DoD）

> 每项：**完成 = 以下全部成立**。验收方式给出可执行断言。

| # | 交付物 | 完成定义（DoD） | 验收方式 |
|---|---|---|---|
| 1 | flowc | 全流水线可用；§71.1-71.7 全部通过；编译时间原则（§75）满足；D6 精度不变量成立；增量编译与缓存失效正确 | Conformance + 基准测试（10/100 MB/1 GB，§74） |
| 2 | 标准库 | 清单 §2 全模块；每个 API 有 12 项属性文档（§62/交付物 13）；普通/性能 API 分层正确 | 每模块单元测试 + API 文档抽查 |
| 3 | Runtime | Future/Task/Scheduler/Cancellation/Channel/Async I/O/UI Domain/State/Component Identity 全实现；可裁剪（未用功能不链接） | 并发压力测试 + 裁剪后体积报告 |
| 4 | UI Framework | §4 清单全能力；UI+Task 联合语义（§43）端到端；Hot Reload 可用 | 记账本/下载器示例 + §72 联合验收 |
| 5 | IDE | LSP 全能力（补全/跳转/引用/诊断/Quick Fix）；UI Preview；Hot Reload；物化仪表盘（D2）；Language Veil 投影 | 使用 IDE 完成 MVP 验收应用 |
| 6 | Debugger | §6 清单全能力（含 Task/Async Call Stack/UI State） | 断点/异步栈/状态检查测试集 |
| 7 | Profiler | CPU/Memory/Allocation/I-O/Task/UI 六区 + Flow 专属（Materialization/Large Copy/Borrow 结果） | 对基准应用产出可解释报告 |
| 8 | flowpkg/Registry | flow.toml/lock/workspace/feature/target/cache/offline；Registry 可发布/获取包 | 发布-拉取-离线构建闭环测试 |
| 9 | Build/Publish | single/split/bundle/portable/installer；签名；更新；差分更新 | 各模式产物安装/运行冒烟 |
| 10 | FFI/Platform SDK | C ABI/CStr/CString/CArray/ABI-safe struct；unsafe 边界；五平台适配 | FFI 互操作测试矩阵 |
| 11 | The Flow Book | §11 22 章全；前半（1-10 章）不出现生命周期/Borrow Checker 术语（§11 原则） | 无 Rust 经验测试者完成 §73 清单 |
| 12 | Language Reference | §12 清单全；为 D1-D11、D13 的已确认决策提供正式条文 | 一致性检查（与 Conformance 对齐） |
| 13 | Std Lib Reference | 每个 API 12 项属性 + 示例（§13） | 自动生成 + 人工审校 |
| 14 | UI Guide | §14 清单全；UI Domain/State 专章 | 读者按指南完成 UI 应用 |
| 15 | Concurrency Guide | §15 清单全；"先教怎么用再讲机制"原则 | 无并发经验读者完成 Task 章节练习 |
| 16 | Advanced Guide | §16 清单全（&T/&mut T/move/零拷贝/FFI/unsafe） | 高级读者完成零拷贝优化案例 |
| 17 | Package & Build Guide | §17 清单全 | 按指南完成发布流程 |
| 18 | Platform Guides | 五平台指南全 | 每平台完成一个发布案例 |
| 19 | Cookbook | §19 清单 12 个示例全部可运行 | 每个示例通过 MVP 联合验收 |
| 20 | Compiler Internals | §20 清单全；含 D6 不变量、D1 视图化 pass 说明 | 文档评审 + 新贡献者入职测试 |
| 21 | Specifications | §21 11 份规范全；与实现无冲突 | 规范-实现对照审计 |
| 22 | Conformance | §22 清单全覆盖 | 全绿通过 + 覆盖率报告 |
| 23* | 测试框架 | 单元/UI/快照测试；`flow test` 集成；测试运行器确定性 | 框架自测 + 示例项目测试集 |

---

## 4. 里程碑与退出标准

| 里程碑 | 内容 | 退出标准 |
|---|---|---|
| **M0 设计收口** | 《Flow 设计决策登记簿》D1-D11、D13 全部评审确认 | 登记簿全绿；正式文档同步 |
| **M1 核心语言**（Phase 0） | Lexer→Parser→AST→HIR→Name Res→Type Check；fn/let/if/for/struct/enum/Option/Result；诊断；格式化 | 设计稿 §70 Phase 0 清单全；D5/D6/D7/D9/D13 生效 |
| **M2 值流核心**（Phase 1A-1D） | Owned/Copy/Move/Projection/Consume/Borrow/Escape/Capture/Materialize；VIR；Summary + 组合 + 预算；Optimization Explain | §71.1 跨函数组合通过；精度降级行为一致（D6） |
| **M3 大对象验证**（Phase 1E） | 10 MB/100 MB/1 GB 字符串/Vec/图像/二进制/DB/HTTP | §71.4 + §74 统计达标；诊断计数断言通过 |
| **M4 类型与模块**（Phase 2） | 泛型/trait/impl/where/module/FFI；基础标准库；flowpkg 最小；测试框架（23） | §71.2 跨模块无强制 Materialize；flowpkg 闭环 |
| **M5 并发 + UI 联合**（Phase 3） | Future/Task/go/go!/cancellation/channel/Send/Sync；UI Domain/Component ID/State Slot/payload | §72 联合验收全绿；D3/D4/D8 负例全通过 |
| **M6 MVP 发布** | MVP 边界（§2.1）全交付；记账本端到端；Windows single/portable | §2.2 MVP 验收通过；可对外发布 |
| **M7 完整 UI 与工具链**（Phase 5/4） | 完整 UI（props/回调/动画/热重载）；IDE(LSP)/Debugger/Profiler | §5 DoD 5/6/7 通过 |
| **M8 五平台与 1.0**（Phase 6/7） | 桌面三平台 + 移动端桥；Build/Publish 全模式；Registry；文档全套；Conformance 全绿 | 22+1 项 DoD 全绿 → Flow 1.0 完整 |

---

## 5. 风险主线

> 风险按"若失败则整个项目失败"排序。**R1 是唯一的技术存亡风险，必须单列管理。**

### R1　编译器分析器（单列最大风险）

- **内容**：VFG + VIR + Ownership/Escape/Capture + Semantic Summary + 组合 + 预算 + 增量 + 诊断。本质是"用户友好的 borrow checker + 摘要化"，属于编译器工程最难级别。设计稿已把风险意识写入（§8/§19/§59/§79），但交付物清单把它藏在"第 1 项"内部，未单独暴露。
- **缓解**：① M2 提前做分析器原型（不做 codegen 也能验证）；② 预算 + Level 0 兜底（任何分析失败退化为 Owned + 诊断，不阻塞编译）；③ D6 不变量作为架构约束；④ M3 用真实大对象数据验证"物化频率"是否符合认知负担目标（§73 的 90% 指标是最终裁判）。
- **失败判定**：M3 结束时，若"普通应用任务中 Materialize 比例 > 30%"且诊断无法降低，则 §79 的技术假设不成立，需要回到设计层。

### R2　UI + 并发联合语义（Phase 3/5）

- 一级核心场景，runtime 最复杂部分。缓解：M5 作为独立里程碑而非顺带功能；负例测试集（D3/D8）先行。

### R3　多平台持续成本

- 五平台 + 签名 + 商店 + 工具链。缓解：MVP 只承诺 Windows 桌面；1.0 完整 = 桌面三平台；移动端 1.1（D11）。

### R4　生态冷启动

- Registry 从空开始。缓解：1.0 前 Registry 只服务官方包；Cookbook 12 示例承担"生态样板"角色；第三方生态是 1.1 目标。

### R5　规模与人力

- 22+1 项 vs 个人/小团队。缓解：MVP 边界（§2）为执行纪律，任何里程碑超期时**优先砍功能而非加人**；M6 是"可发布"的硬门槛，M6 之前不投入 IDE 完整版/Debugger/Profiler。

---

## 6. 建议新增与形态决策

### 6.1 新增交付物

| # | 新增交付物 | 理由 | 归属 |
|---|---|---|---|
| 23 | 用户级测试框架（单元 + UI + 快照） | 原清单有 `flow test` 命令但无测试框架交付物；"可发布应用"需要测试能力 | MVP（最小）+ 1.0 |
| 24 | 崩溃上报与遥测开关（可后置） | 发布后基础设施；1.0 发布前应有最小崩溃收集 | 1.0（最小） |
| 25 | 物化/性能仪表盘（并入 IDE 与 Profiler） | D2 诊断策略的可视化承载 | 1.0 |

### 6.2 IDE 形态决策

**MVP 阶段：LSP + 编辑器扩展**（vscode/neovim 优先），不建独立 IDE；
**1.0 完整：独立 IDE**（含 UI Preview/Debugger/Profiler 集成）——仅在 M7 之后投入，避免与 M6 抢资源。

### 6.3 Codegen 决策引用

Codegen 采用 LLVM（D10）；flowc 依赖 LLVM 工具链，需在 M1 前确定版本锁定与分发策略。

---

## 7. 与既有文档的关系

```text
《Flow 语言完整设计方案定稿版》(V5.8)   ← 语义层：是什么、怎么想（冻结）
《Flow 1.0 最终交付物清单》             ← 产品层：交付什么（最终愿望清单）
《Flow 设计决策登记簿》(本文档配套 1)    ← 决策层：实现前必须确定的定义
《Flow 1.0 执行计划与完成定义》(本文件)  ← 执行层：何时、以何顺序、如何验收
```

执行顺序依赖：**决策登记簿（M0 全确认）→ 本计划里程碑（M1-M8）→ 每阶段 DoD 全绿才算阶段完成。**

---

## 8. 一句结论

> Flow 1.0 的完成定义不是"22 项清单全部做出"，而是 **"M6 的 MVP 通过 §72 联合验收并发布" + "M8 时 22+1 项 DoD 全绿"**。所有执行决策以 M6 为第一目标，R1 为第一风险，D1-D11、D13 为第一优先的待决事项。
