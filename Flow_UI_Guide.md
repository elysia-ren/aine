# Flow UI Guide

> **版本**：V0.1（初版）
>
> **状态**：本版收录《Flow 设计决策登记簿》定稿版中属于 UI 的决策（D3/D8/D11），作为 UI 指南的第一版正式内容。
>
> 面向：普通开发者（交付物 14）。
>
> 已收录章节：
>
> - §4 `ui {}` 语义与事件回调（D3）
> - §5 全局状态 `@global`（D8）
> - §6 平台支持（D11）
>
> 其余章节（`ui` / `render` / `@state` / Component / Props / Layout / Window / Dialog / Animation / Hot Reload / State Management）将在 Phase 3/5 实现后填充。

---

## §1 概览（占位）

Flow UI 是第一方声明式 UI（V5.8 §50）。核心运行模型（交付物 4）：

```text
UI Event → go{} → HTTP / SQLite / File / IO → Task Result → ui { ... } → State Update
```

---

## §4 `ui {}` 语义与事件回调

> **来源：登记簿 D3（🟢 已确认）**

### 4.1 `ui {}` 是什么

`ui { ... }` 是 **UI Domain Operation**（V5.8 §36），不是"切回 UI 线程"，也不是普通闭包（V5.8 §37）：它在 HIR/VIR 中 lowering 为 `UIDomainOperation`，校验组件/状态身份，携带合法的 Owned/Send 数据，由 UI runtime 提交执行。

### 4.2 `ui {}` 内部规则

**允许：**

1. **读取 `@state`**：直接读 State Storage（同一 UI Domain 内同步读取，无消息开销）；读取在操作执行时求值：

```flow
ui {
    total = records.iter().map(|r| r.amount).sum()   // 读取 State Storage 中的 records
}
```

2. **写入 `@state`（State Target）**：如 `records = new_records`——`records` 表示 `ComponentId + StateSlot`，`new_records` 作为 Owned+Send payload 传入（V5.8 §38-39）；写入 = Consume/Store，payload 在任务中不再可用。
3. **调用只读操作**：判定口径——不产生新的任务、不发起 IO/外部副作用、不写 State 之外的状态。读取捕获的 Owned/Send payload 与 State 均属允许。

**禁止（编译器检查，违者报错）：**

1. 在 `ui {}` 内发起新的 `go{}` / `go!{}`——错误提示："请在 ui{} 外部创建后台任务"；
2. 捕获普通外部借用（V5.8 §38）：只允许 State Target 与 Owned/Send payload；
3. 读取已 move 的变量（V5.8 §39）：payload 写入 State 后在任务中不再可用。

### 4.3 事件回调签名与 `?`

- 回调签名：`(Event) -> ()` 或 `(Event) -> Result<(), UiError>`（无返回值时默认 `()`）；
- `? ` 在回调内传播到回调返回值，由 UI runtime 捕获并显示错误提示（不崩溃、不 panic）：

```flow
Button("添加") {
    let amount = amount_input.to_f64()?   // 失败 → 回调返回 Err → UI 显示错误提示
    ...
}
```

### 4.4 后台任务不得持有 `@state` 借用（V5.8 §34 硬规则）

任何 `go{}` / `go!{}` 不得持有 `@state` 的普通借用；需要数据时用 `ui.snapshot(user)` 或只提取所需字段（V5.8 §35）。

### 4.5 组件身份

后台任务不保存 UI State 裸指针：运行时采用 `ComponentId + Generation` 构成的可失效身份；组件销毁后 generation invalidated，后台操作到达时 UI runtime 验证身份，失效则忽略/取消/报告，**不产生悬垂引用**（V5.8 §41）。

---

## §5 全局状态 `@global`

> **来源：登记簿 D8（🟢 已确认）**

### 5.1 定义

`@global` = **全局 State Slot**（组件无关的 State Storage）：

- 只允许存 **Send** 数据；
- 生命周期 = 进程；
- 热重载时按类型匹配保留。

### 5.2 声明位置

模块顶层，不隶属组件：

```flow
@global
var settings: Settings
```

### 5.3 写入与读取规则

| 场景 | 规则 |
|---|---|
| 写入 | 只能经 UI Domain Operation（`ui {}`）或受控消息写入 |
| 后台任务写入 | **禁止**（同 `@state` 硬规则，V5.8 §34） |
| UI 内读取 | 直接读 State Storage |
| 后台任务读取 | 需 `snapshot`（同 V5.8 §35） |

### 5.4 禁止项（V5.8 §44）

禁止利用 `@global` 绕过 Send / Sync / 任务边界 / State Domain；共享可变数据仍使用消息、同步原语、受控状态域、任务模型。

---

## §6 平台支持

> **来源：登记簿 D11（🟢 已确认）**

### 6.1 1.0 范围

- **桌面（Skia 渲染后端）优先**：Windows / Linux / macOS（V5.8 §51）；
- UI Domain 在桌面绑定主线程。

### 6.2 1.1 范围

- **移动端 UI 后端列为 1.1**：第一阶段支持"纯逻辑应用 + 最小 UI 桥"（窗口/基本控件经平台原生桥），完整声明式 UI 后置；
- UI Domain 在移动端绑定主线程（平台惯例）。

### 6.3 平台特定逻辑

普通业务代码不应大量出现平台判断；平台特定逻辑使用 `#[platform(...)]`（V5.8 §64）。

---

## 附录 A：本版迁移来源

| 章节 | 来源决策 | 状态 |
|---|---|---|
| §4 | D3 | 🟢 已确认 |
| §5 | D8 | 🟢 已确认 |
| §6 | D11 | 🟢 已确认 |

## 附录 B：待填充（Phase 3/5 期间）

`ui` / `render` 语法、`@state` 教程、Component/Props、事件系统、布局、Window/Dialog、动画、Hot Reload、State Management、异步 UI 完整指南、移动端 UI 指南（1.1）。
