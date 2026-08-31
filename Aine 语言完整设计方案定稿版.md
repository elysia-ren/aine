# Aine 语言完整设计方案 V5.8
## 实现前核心定稿版

> **版本性质：产品方向、用户体验原则、核心语言模型、所有权/借用策略、并发与 UI 边界、编译器总体架构正式定稿。**
>
> V5.8 不再继续增加核心语言功能，而是对 V5.7 进行最后一轮语义收口。重点解决决定 Aine 能否真正实现的关键问题：
>
> - 跨函数、跨模块的所有权与值流分析；
> - Semantic Summary 的可组合性与精度控制；
> - Owned 语义与 Borrow/Compiler View 优化的严格分离；
> - 大对象物化与隐式复制控制；
> - `String` / `str_view` 的稳定语义；
> - `go{}` 与 UI State 的安全边界；
> - `ui {}` 的 Message/Domain Operation 语义；
> - `Send` / `Sync` 在任务边界中的编译器约束；
> - Value Aine Graph / VIR；
> - 确定性诊断与优化解释；
> - AI 完全退出语言核心、编译器和默认工具链。
>
> V5.8 的核心目标不是“让编译器拥有无限分析能力”，而是：
>
> > **让编译器在合理的分析成本内承担尽可能多的复杂度，在无法证明时安全、透明、可解释地回退，而不是把复杂度转嫁给普通开发者。**

---

# 一、语言定位

> **Aine 是面向个人开发者和小团队的原生全栈应用语言：一套代码完成 UI、业务逻辑和数据访问，并编译为各平台原生应用。**

核心目标：

> **让一个人以较低认知成本，稳定地做出安全、原生、可发布的桌面和移动应用。**

重点场景：

- Windows / Linux / macOS 应用；
- Android / iOS 应用；
- 下载器；
- OCR / 文档 / 校对工具；
- 图片与媒体工具；
- 数据库客户端；
- Markdown / 文本工具；
- 文件管理与批处理工具；
- AI 客户端；
- 工程计算与办公工具；
- 小型业务工具。

不以以下场景为第一竞争目标：

- 操作系统内核；
- 底层驱动；
- 浏览器 Web 前端；
- 大型分布式服务；
- 云原生专用模型；
- 极限科学计算；
- 极端底层系统优化。

---

# 二、Aine 的核心设计命题

Aine 不试图证明：

> “Rust 的复杂语法可以简单化。”

真正要验证的是：

> **编译期所有权安全、确定性析构、零 GC 与高性能，并不必然要求普通应用开发者理解生命周期和借用理论。**

因此：

> **普通开发者表达业务语义，编译器表达所有权语义。**

---

# 三、核心原则

## 3.1 显式边界，隐藏内部复杂度

开发者必须明确：

- 错误传播；
- 后台任务；
- `unsafe`；
- 显式所有权转移；
- 外部运行时；
- 平台专用代码。

开发者默认不需要明确：

- 生命周期；
- 生命周期参数；
- 闭包捕获方式；
- Future 状态机；
- UI 线程；
- 调度器；
- MIR；
- ABI；
- 内部值表示。

---

## 3.2 安全默认

Aine 默认提供：

- 无裸空值；
- 边界检查；
- 所有权检查；
- 借用冲突检查；
- 安全任务边界；
- 确定性资源释放；
- 显式 `unsafe`；
- 明确 FFI 边界。

---

## 3.3 复杂性藏在编译器，而不是运行时

正式原则：

> **复杂性优先由编译器承担，而不是运行时承担，也不应转嫁给普通开发者。**

因此 Aine 不以：

- tracing GC；
- 隐式 ARC；
- 隐式共享 RC；
- 手工智能指针；
- AI 安全判断；

作为核心机制。

---

## 3.4 确定性优先

Aine 的核心工具链必须是确定性的。

相同：

```text
Source
Compiler
Edition
Dependencies
Target
Build configuration
```

必须产生确定、可解释的编译结果。

---

## 3.5 新语法必须有高频收益

若一种能力能够通过：

- 编译器推导；
- 标准库；
- IDE；
- 发布工具；

完成，则优先不增加新的核心语法。

---

# 四、复杂度分层

| 层级 | 普通开发者 | 内容 |
|---|---|---|
| 日常层 | 必须掌握 | `fn`、`let`、控制流、结构体、`Option/Result`、UI、SQLite、HTTP |
| 进阶层 | 按需 | `move`、`&T`、`&mut T`、trait、泛型约束、任务句柄、FFI |
| 底层层 | 默认不需要 | 生命周期、VIR、MIR、调度器、ABI、裸指针、内部优化 |

第一版不要求普通用户理解：

```text
lifetime
borrow region
outlives
capture lifetime
Future state machine
```

---

# 五、内存模型总原则

V5.8 正式冻结：

> ## **语义拥有优先，编译器借用优化，显式借用高级化。**

这句话具有严格含义：

### 用户语义层

普通开发者主要操作：

```text
String
Vec<T>
User
Image
Data
```

### 编译器实现层

编译器内部可以选择：

```text
Owned
Borrowed
Compiler View
Copied
Materialized
```

### 高级控制层

高级开发者可以明确：

```text
&T
&mut T
move
unsafe
```

---

# 六、Owned 语义

## 6.1 `String`

V5.8 正式规定：

> **`String` 永远表示拥有的字符串语义。**

例如：

```flow
fn load_name(user: User) -> String {
    return user.name
}
```

调用方获得的是：

> 独立拥有的字符串值。

编译器可以内部消除不必要分配，但不能改变 `String` 在用户层面的拥有语义。

---

## 6.2 `str`

`str` 不再承担模糊的“有时拥有、有时借用”含义。

V5.8 普通语言不再把 `str` 作为默认拥有字符串类型。

标准写法：

```text
String
```

高级借用：

```text
&str
```

内部视图：

```text
str_view
```

只有语义明确时才允许出现。

---

# 七、Compiler View

## 7.1 定义

> **Compiler View 是编译器内部产生的临时、零成本、受验证的借用视图。**

它：

- 不是普通用户必须理解的类型；
- 不需要生命周期参数；
- 不产生 GC/RC；
- 不拥有数据；
- 不允许非法逃逸；
- 不改变程序的安全语义。

例如：

```flow
fn print_name(user: User) {
    print(user.name)
}
```

内部可以：

```text
user.name
↓
Compiler View
↓
直接读取
```

而无需创建新的字符串对象。

---

# 八、Owned 语义与优化必须完全分离

Aine 编译器分成两个逻辑层：

```text
Source
 ↓
Semantic Validation
 ↓
Valid Program
 ↓
Representation Optimization
 ↓
Native Code
```

## 8.1 语义检查

负责判断：

> 程序是否合法。

包括：

- 类型；
- 所有权；
- 借用；
- 任务边界；
- UI 边界；
- FFI 安全。

## 8.2 优化

负责判断：

> 合法程序能否更便宜地实现。

包括：

- Borrow；
- Compiler View；
- Allocation Elimination；
- Copy Elision；
- Materialization 消除。

---

## 8.3 优化失败不是语义错误

如果：

```text
Borrow optimization failed
```

不意味着：

```text
Program invalid
```

而是：

```text
Borrow unavailable
↓
Owned implementation
```

必须严格区分：

> **“程序不合法”与“程序合法但没有被优化得足够好”。**

---

# 九、Materialization

当编译器无法证明视图能够安全存在时：

```text
Borrow/View
↓
Materialize
↓
Owned value
```

这是合法、安全的回退路径。

---

## 9.1 Materialization 的三档成本

```text
Cheap
Material
Large
```

### Cheap

可以无提示完成。

### Material

允许完成，并产生非阻塞性能信息。

### Large

例如：

- 100 MB；
- 1 GB；
- 大型图像；
- 大型 `Vec`。

不得悄悄产生高成本复制。

必须产生性能诊断。

---

# 十、禁止隐式大型 Clone

为了修复一般 Ownership Conflict：

> 编译器不得偷偷生成大型 `clone()`。

允许：

```flow
let copy = data.clone()
```

因为这是明确行为。

不允许：

```text
用户没有写 clone
↓
编译器偷偷复制 500 MB
```

---

# 十一、性能诊断

编译器/IDE 必须能够解释：

```text
这里发生了一次 103 MB 的数据物化。

原因：
源数据无法安全地在当前返回范围内继续借用。

分析：
A → B → C → dynamic indexing

可选方案：
1. 接受独立拥有值；
2. 拆分函数以帮助分析；
3. 使用显式高级借用 API `&T`。
```

诊断使用：

> **业务语言 + 数据流事实**

而不是强迫用户理解：

```text
lifetime region
outlives
borrow checker
```

---

# 十二、Value Aine Graph

V5.8 正式加入：

> **Value Aine Graph（VFG）**

VFG 是所有权、借用、逃逸、闭包、任务和 UI 传输分析的统一中间抽象。

它描述：

```text
Origin
Projection
Transformation
Copy
Move
Consume
Borrow
Escape
Capture
Boundary
```

例如：

```text
user
 ↓
profile
 ↓
label
 ↓
return
```

或者：

```text
new_records
 ↓ move
UI Operation
 ↓
State.records
```

---

# 十三、VIR：Value Aine IR

VFG 的编译器实现载体定义为：

> **VIR（Value Aine Intermediate Representation）**

架构：

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
VIR
 ↓
Ownership / Escape / Capture / Summary
 ↓
MIR
 ↓
Optimization
 ↓
Codegen
```

VIR 不属于用户可见语法。

---

# 十四、VIR 的基本节点

至少支持：

```text
Source
Copy
Move
Consume
FieldProjection
ElementProjection
Borrow
Materialize
Call
Return
Capture
TaskTransfer
UITransfer
StateRead
StateWrite
```

---

# 十五、跨函数/跨模块语义摘要

Aine 正式采用：

> **Function Semantic Summary（函数语义摘要）**

摘要不是生命周期表。

它描述的是：

> **函数的值流、所有权影响和关键副作用。**

---

## 15.1 摘要示例

```text
get_label(user)

input[0]:
    User

return:
    DerivedFrom(
        input[0],
        ["profile", "label"]
    )

mutation:
    none

effects:
    none

escape:
    caller

allocation:
    not-required
```

---

# 十六、Summary 使用 Value Provenance

摘要的核心不是：

```text
'a
'b
```

而是：

```text
Input(i)
Field(base, field)
Element(base, index)
Transform(base)
Owned
Copy
Unknown
```

例如：

```text
Field(
    Field(
        Input(0),
        "profile"
    ),
    "label"
)
```

最终可以规范化为：

```text
DerivedFrom(
    Input(0),
    ["profile", "label"]
)
```

---

# 十七、Summary Composition

跨函数必须支持：

> **摘要组合与参数代入。**

例如：

```text
C:
get_label
    return = Field(Input(0), ["profile", "label"])

B:
get_display_name
    return = get_label(Input(0))
```

编译器将其组合为：

```text
B.return
=
Field(Input(0), ["profile", "label"])
```

A 再调用 B：

```text
A.user
↓
B.summary
↓
C.summary
↓
user.profile.label
```

无需重新展开整个调用图。

---

# 十八、Summary 精度等级

不要求无限精确。

定义至少五级：

```text
Level 0
Unknown

Level 1
Owned / Copy / Basic Effect

Level 2
Input-derived

Level 3
Field / Element Projection

Level 4
Limited path-sensitive information

Level 5
Complex summary
```

编译器根据：

- 函数复杂度；
- 模块边界；
- 编译时间预算；
- 缓存情况；

决定摘要精度。

---

# 十九、跨函数分析策略

采用：

> **局部优先、摘要优先、有限扩展、超预算回退。**

## 第一层

必须支持：

- 字段投影；
- 条件；
- 普通循环；
- 简单集合；
- 简单闭包；
- 简单函数调用。

## 第二层

有限支持：

- 跨函数；
- 跨模块；
- 容器传播；
- 闭包逃逸；
- 简单递归。

## 第三层

复杂情形：

- 深层递归；
- 大调用图；
- FFI；
- 复杂高阶组合；
- 复杂异步数据流。

超过预算：

> **禁止无限分析。**

---

# 二十、跨模块语义摘要缓存

每个已编译模块产生：

```text
.ainei
semantic summary
debug/source mapping
```

语义摘要：

- 编译器自动生成；
- 内容寻址；
- 版本化；
- 可持久化；
- 与源码 hash 绑定。

修改 A 模块：

> 仅使依赖它的摘要失效。

无需重新分析无关模块。

---

# 二十一、公开 API 与内部摘要分离

`.ainei` 面向：

- 公共符号；
- 类型；
- ABI/API 契约；
- 泛型边界。

Semantic Summary 面向：

- 编译器优化；
- 跨模块值流；
- Ownership；
- Escape；
- Task；
- UI 边界。

二者不能要求开发者手工维护同步。

---

# 二十二、函数参数

## 22.1 普通模式

```flow
fn show(user: User) {
    print(user.name)
}
```

编译器可在内部选择 Borrow。

---

## 22.2 `move`

```flow
fn store(move user: User) {
    users.push(user)
}
```

表示：

> 明确要求所有权转移。

---

## 22.3 显式借用

```flow
fn inspect(user: &User)
fn modify(user: &mut User)
```

属于高级精确 API。

---

# 二十三、生命周期

V5.8 正式冻结：

> **用户可见语法不提供生命周期参数。**

不提供：

```text
'a
'b
'a: 'b
```

但编译器内部完整维护：

```text
Ownership
Borrow
Escape
Region
Drop
Capture
```

---

# 二十四、多来源返回

删除：

```text
#[return_from]
```

普通代码：

```flow
fn choose(a: User, b: User) -> String {
    if a.age > b.age {
        return a.name
    }

    return b.name
}
```

普通用户不需要知道：

> 返回来源来自哪个生命周期。

编译器：

- 进行 Value Aine 分析；
- 能借用则优化；
- 不能安全证明则 Materialize；
- 成本过高则建议高级 API。

---

# 二十五、闭包捕获

固定推导优先级：

```text
Borrow
↓
Copy
↓
Move
```

但具体策略受到：

- 是否逃逸；
- 是否长期保存；
- 是否进入任务；
- 是否进入 UI Operation；

约束。

---

## 25.1 当前作用域

```flow
let name = "张三"

button("你好") {
    print(name)
}
```

默认优先借用。

---

## 25.2 跨任务

```flow
go! {
    print(name)
}
```

普通借用不得逃逸。

如果不能安全借用：

> 自动/诊断要求所有权转移。

必要时使用：

```flow
move name
```

---

# 二十六、`Send` 与 `Sync`

`Send` / `Sync` 成为并发安全的内部能力。

普通代码不要求手写：

```text
T: Send
T: Sync
```

编译器自动检查：

> 跨任务 / 跨 UI Domain 的值是否允许安全移动或共享。

失败时优先输出：

> “这个值不能安全地发送到后台任务。”

而不是只显示 trait 术语。

高级开发者可查看完整约束。

---

# 二十七、`defer` 与析构

```flow
fn read_file(path: String) -> Result<Vec<u8>, Error> {
    let file = File.open(path)?
    defer file.close()

    file.read_all()
}
```

退出：

```text
函数退出
↓
defer 后进先出
↓
局部资源析构
```

适用于：

- 正常返回；
- early return；
- panic；
- 可捕获异常。

---

# 二十八、并发模型

## 28.1 Future

```text
Future<T>
```

表示异步计算抽象。

## 28.2 Task

```text
Task<T>
```

表示已经提交调度运行的异步任务。

---

# 二十九、`go{}`

```flow
let task = go {
    fetch_data()
}

let result = task?
```

正式语义：

> `go{}` 创建并立即提交一个结构化 `Task<T>`。

父作用域必须：

- 等待；
- 取消；
- 或根据显式规则处理该任务。

---

# 三十、`go!{}`

```flow
go! {
    background_work()
}
```

表示：

> detached background task。

必须：

- 独立处理错误；
- 满足所有权要求；
- 不持有普通外部借用；
- 不假设一定完成。

---

# 三十一、Task 边界

Task 是所有权安全边界。

普通借用不得跨越：

```text
Task boundary
```

任务传输的数据必须满足：

```text
Owned
或
安全 Copy
或
满足 Send
```

---

# 三十二、取消

标准异步 API 必须明确：

- cancellation；
- cancellation points；
- side effects；
- rollback guarantees。

取消任务：

> 不自动撤销已经发生的外部副作用。

---

# 三十三、UI State

```flow
@state
let mut user: User
```

不是普通变量。

它是：

> **Runtime-managed State Storage**

包含：

- Component ID；
- State Slot；
- 生命周期；
- 观察关系；
- 热重载信息。

---

# 三十四、后台任务禁止直接借用 `@state`

V5.8 硬规则：

> **任何 `go{}` / `go!{}` 均不得持有 `@state` 的普通借用。**

禁止：

```flow
go {
    print(user.name)
}
```

这种代码如果其语义要求后台任务继续持有 UI State 引用，则必须被编译器拒绝或重写为合法数据传输形式。

---

# 三十五、UI Snapshot

需要把状态数据带入后台：

```flow
let snapshot = ui.snapshot(user)

go {
    analyze(snapshot)
}
```

`snapshot` 是普通拥有值。

但推荐程序员只提取真正需要的数据：

```flow
let user_id = user.id
let query = user.query
```

然后：

```flow
go {
    let data = db.query(user_id, query)?

    ui {
        update(data)
    }
}
```

避免无意义地复制整个大型状态。

---

# 三十六、`ui {}` 的正式语义

V5.8 不再把：

```flow
ui { ... }
```

解释为：

> “切回 UI 线程。”

正式定义：

> **`ui { ... }` 是一个 UI Domain Operation。**

它：

- 创建 UI 操作；
- 校验组件/状态身份；
- 携带合法的 Owned/Send 数据；
- 由 UI runtime 提交执行。

---

# 三十七、`ui {}` 不是普通闭包

在 HIR/VIR 中：

```text
ui { ... }
```

必须 lowering 成：

```text
UIDomainOperation
```

而不是普通：

```text
Closure
```

---

# 三十八、UI Operation Capture

`ui {}` 捕获分为：

## State Target

例如：

```flow
ui {
    records = new_records
}
```

`records` 表示：

```text
ComponentId
+
StateSlot
```

不是普通变量借用。

---

## Task-owned Payload

```text
new_records
```

如果它是后台任务拥有的数据，则作为：

```text
Owned + Send
```

消息体传入 UI Domain。

---

# 三十九、UI State Write = Consume / Store

例如：

```flow
ui {
    records = new_records
}
```

语义：

```text
new_records
↓
Move
↓
UI State Storage
```

`new_records` 在任务中不再可用。

随后：

```flow
ui {
    total = records.iter()
        .map(|r| r.amount)
        .sum()
}
```

读取的是：

```text
State Storage
```

而不是已经 moved 的：

```text
new_records
```

---

# 四十、UI Operation 中的 Send 要求

跨越：

```text
Task
↓
UI Domain
```

的普通数据必须满足：

```text
Owned + Send
```

或：

```text
Copy
```

禁止传递：

- 普通任务局部借用；
- 非 Send 对象；
- 非法可变引用；
- 裸指针。

---

# 四十一、UI 组件身份

后台任务不保存 UI State 的裸指针。

运行时采用：

```text
ComponentId
Generation
```

构成的可失效身份。

组件销毁：

```text
generation invalidated
```

后台操作到达：

```text
UI runtime validates identity
```

若已失效：

- 忽略；
- 取消；
- 报告；
- 或按照任务策略处理。

不得产生悬垂引用。

---

# 四十二、典型 UI + Task 模式

```flow
Button("查询") {
    go {
        let data = db.query(
            "SELECT * FROM records"
        )?

        ui {
            records = data
        }
    }
}
```

必须满足：

```text
Task owns data
↓
Move into UI Operation
↓
Move into State Storage
↓
No copy
↓
No dangling reference
```

---

# 四十三、UI + Task 联合语义是核心验收项

Phase 3 与 Phase 5 不再完全独立。

必须共同验证：

```text
UI Event
↓
Task
↓
SQLite / HTTP
↓
Task-owned Result
↓
UI Operation
↓
State Update
```

这是 Aine 的一级核心场景。

---

# 四十四、`@global`

`@global` 是运行时托管全局状态。

禁止利用它绕过：

- Send；
- Sync；
- 任务边界；
- State Domain。

共享可变数据仍然使用：

- 消息；
- 同步原语；
- 受控状态域；
- 任务模型。

---

# 四十五、错误处理

```text
Option<T>
```

表示：

> 有 / 无。

```text
Result<T, E>
```

表示：

> 成功 / 失败。

```text
?
```

表示：

> Try 传播。

---

# 四十六、Task 与 `?`

`Task<Result<T,E>>` 可以参与统一 Try 语义：

```flow
let result = task?
```

执行：

```text
wait
↓
extract
↓
propagate error
```

不定义特殊的第二套 async-await error 语义。

---

# 四十七、`try/catch`

仅用于：

- FFI exception；
- 平台 exception；
- 明确允许捕获的 panic。

业务失败：

> 优先 `Result`。

---

# 四十八、FFI

安全 FFI 优先：

```text
CStr
CString
CArray
ABI-safe struct
```

例如：

```flow
extern "C" {
    fn strlen(s: CStr) -> usize
}
```

裸指针：

```text
*const T
*mut T
```

只进入：

```text
unsafe
```

高级边界。

---

# 四十九、Python FFI

如果启用：

必须明确：

- Python version；
- runtime；
- third-party packages；
- native extensions；
- platform compatibility；
- package size；
- license。

Python 不属于 Aine 默认运行时。

---

# 五十、UI Framework

第一方声明式 UI：

```flow
ui Counter {
    title = "计数器"
    size = (400, 300)

    @state
    let mut count = 0

    render {
        Column {
            Text(f"当前计数：{count}")

            Row {
                Button("+1") {
                    count += 1
                }

                Button("-1") {
                    count -= 1
                }

                Button("重置") {
                    count = 0
                }
            }
        }
    }
}
```

UI runtime 负责：

- 状态；
- 渲染；
- 事件；
- 布局；
- 生命周期；
- 平台差异。

---

# 五十一、渲染后端

第一阶段：

```text
Skia
```

默认桌面渲染后端。

系统 WebView：

```text
可选
```

原生控件：

```text
后续
```

---

# 五十二、Language Veil

Canonical Source 是唯一真相：

```text
.aine
```

Language Veil 是：

> **IDE projection**

而不是第二份源码。

核心编译器不接受任意本地化关键字作为新的核心语法。

---

# 五十三、Source Map

必须维护：

```text
Display Token
↓
Canonical Token
↓
AST/HIR Span
```

报错位置永远能够映射回 Canonical Source。

默认复制：

> Canonical Source。

---

# 五十四、核心语法

控制在约 40 个核心语义关键字以内：

```text
fn let mut const
if else for in while return break continue match
struct enum type trait impl where
pub use mod
move
unsafe extern
try catch
ui render
```

V5.8 不新增：

```text
view
lifetime
return_from
advanced
```

等专门语法。

---

# 五十五、基本语法

```flow
let name = "张三"
let mut count = 0

fn add(a: i32, b: i32) -> i32 {
    a + b
}

if count > 10 {
    print("超过上限")
} else {
    print("正常")
}

for i in 0..10 {
    print(i)
}

struct User {
    name: String
    age: i32
}

enum Result<T, E> {
    Ok(T)
    Err(E)
}
```

默认换行结束语句。

续行由正式 grammar/token context 决定。

---

# 五十六、编译器架构

```text
Source
  ↓
Lexer
  ↓
Parser
  ↓
AST + Source Span
  ↓
HIR
  ↓
Name Resolution
  ↓
Type Check
  ↓
VIR
  ↓
Ownership Analysis
  ↓
Escape Analysis
  ↓
Capture Analysis
  ↓
Function Semantic Summary
  ↓
MIR
  ↓
Borrow / Compiler View Lowering
  ↓
Optimization
  ↓
Codegen
  ↓
Native Object / Link
  ↓
Package / Sign / Bundle
```

---

# 五十七、Compiler Diagnostics

诊断必须是：

> **确定性、结构化、可解释、可执行。**

至少包含：

- Error Code；
- 标准术语；
- 人话解释；
- 文件/行/列；
- 源代码片段；
- 原因；
- 建议；
- Quick Fix。

---

# 五十八、Optimization Explain

V5.8 正式要求编译器能够解释：

```text
为何 Borrow
为何 Copy
为何 Materialize
为何不能 Borrow
为何发生 Allocation
```

例如：

```text
性能提示 F2104

这里物化了一个约 103 MB 的 String。

原因：
返回值必须在源对象离开作用域后继续存在。

无法使用零拷贝实现的原因：
当前函数存在动态索引，编译器无法证明返回值来源。

建议：
1. 使用显式高级借用 API；
2. 将逻辑拆分为更容易分析的函数；
3. 接受当前拥有值实现。
```

---

# 五十九、分析预算

所有高级分析必须具有明确预算：

```text
时间预算
内存预算
调用深度预算
摘要规模预算
```

超过预算：

> **保守回退，不无限分析。**

不能因为用户代码复杂就让编译器进入不可控分析状态。

---

# 六十、缓存与 Semantic Summary

缓存键至少包括：

```text
source content hash
compiler version
language edition
stdlib version
package lock
target triple
optimization profile
feature flags
environment configuration
VIR version
semantic summary version
```

缓存不改变程序语义。

---

# 六十一、缓存与摘要失效

函数体改变：

> 当前函数摘要失效。

其摘要被下游依赖：

> 相关调用方重新分析。

无关模块：

> 不重新分析。

这保证大型项目仍具有可接受的增量性能。

---

# 六十二、标准库

第一阶段重点：

```text
File
Path
HTTP
JSON
SQLite
CSV
ZIP
Time
Crypto
Clipboard
Process
Logging
Collections
Configuration
```

API 规范必须包含：

- 类型；
- 错误；
- 阻塞性；
- 异步性；
- 取消；
- 并发；
- 生命周期；
- 分配特征；
- 平台差异。

---

# 六十三、标准库的简单 API 与高级 API

普通 API：

```text
file.read_all()
```

性能 API：

```text
file.read_view()
```

高级 API：

```text
&T
&mut T
```

但：

> 性能 API 不得成为普通开发前置知识。

---

# 六十四、跨平台

目标：

```text
Windows x64/ARM64
Linux x64/ARM64
macOS ARM64/x64
Android
iOS
```

普通业务代码不应大量出现平台判断。

平台特定逻辑使用：

```flow
#[platform(windows, linux)]
fn open_native_menu() { ... }
```

---

# 六十五、发布系统

支持：

```text
single
split
bundle
portable
installer
```

原则：

> **编译决定程序是什么，打包决定程序怎样交付。**

---

# 六十六、发布中心

IDE 提供：

```text
目标平台
发布模式
常用选项
```

显示：

- 预计大小；
- 外部依赖；
- 是否安装；
- 是否签名；
- 更新能力；
- 推荐模式。

GUI 与 CLI 必须共享同一发布语义。

---

# 六十七、AI 正式退出核心工具链

V5.8 正式规定：

> **Aine 编译器、语言服务器、构建系统、运行时、类型系统、Ownership Checker、Codegen 均不得依赖 AI。**

Aine 默认不内置：

- 模型权重；
- LLM runtime；
- 本地 7B/14B 模型；
- 模型推理服务。

---

# 六十八、AI 的可选定位

未来 IDE 可以接入：

- Qwen；
- DeepSeek；
- GLM；
- 用户自己的模型；
- 用户自己的 API。

用途：

- 代码生成；
- 文档问答；
- 迁移；
- 复杂解释；
- 高级重构建议。

但流程必须是：

```text
AI Suggestion
↓
Compiler
↓
Type Check
↓
Ownership Check
↓
Tests
↓
Accepted / Rejected
```

AI：

> **永远不是安全信任根。**

---

# 六十九、完整应用示例

```flow
import sqlite
import ui

struct Record {
    id: i32
    desc: String
    amount: f64
    time: i64
}

ui AccountBook {
    title = "记账本"
    size = (700, 500)

    @state
    let mut records: Vec<Record> = []

    @state
    let mut total: f64 = 0.0

    @state
    let mut desc_input: String = ""

    @state
    let mut amount_input: String = ""

    render {
        Column {
            Text(f"总余额: {total}").font_size(28)

            Row {
                TextInput(
                    placeholder = "描述",
                    bind = desc_input
                )

                TextInput(
                    placeholder = "金额",
                    bind = amount_input,
                    type = number
                )

                Button("添加") {
                    let description = desc_input.clone()
                    let amount = amount_input.to_f64()?

                    go {
                        let record = Record {
                            id: 0
                            desc: description
                            amount: amount
                            time: now()
                        }

                        db.insert("records", record)?

                        let new_records =
                            db.query(
                                "SELECT * FROM records ORDER BY time DESC"
                            )?

                        ui {
                            records = new_records

                            total = records
                                .iter()
                                .map(|r| r.amount)
                                .sum()
                        }
                    }
                }
            }

            List(records) |r| {
                ListItem {
                    Text(f"{r.desc} - {r.amount}")

                    Button("删除") {
                        let id = r.id

                        go {
                            db.delete("records", id)?

                            let new_records =
                                db.query(
                                    "SELECT * FROM records ORDER BY time DESC"
                                )?

                            ui {
                                records = new_records

                                total = records
                                    .iter()
                                    .map(|r| r.amount)
                                    .sum()
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), AppError> {
    db.init("account.db")?
    run_ui(AccountBook)
}
```

这个示例体现：

```text
UI Event
↓
Task
↓
Owned Result
↓
UI Domain Operation
↓
Move into State
↓
State Read
```

全过程不要求用户理解生命周期。

---

# 七十、Roadmap

## Phase 0：核心语言

实现：

- Lexer；
- Parser；
- AST；
- Source Span；
- HIR；
- Name Resolution；
- 类型系统基础；
- `fn/let/if/for`；
- `struct/enum`；
- `Option/Result`；
- Diagnostics；
- Formatter。

---

## Phase 1A：Value Aine Core

实现：

- Owned；
- Copy；
- Move；
- Projection；
- Consume；
- Borrow；
- Escape；
- Capture；
- Materialize。

---

## Phase 1B：VIR

实现：

- Value Aine IR；
- Value Provenance；
- Function-local Aine Graph；
- Basic Dataflow；
- Task/UI boundary nodes。

---

## Phase 1C：Semantic Summary

实现：

- Summary；
- Summary Composition；
- Parameter Substitution；
- Precision Levels；
- Cross-module summary；
- Summary Cache；
- Invalidation。

---

## Phase 1D：Optimization Explain

实现：

- Borrow explanation；
- Copy explanation；
- Materialization explanation；
- Allocation explanation；
- Analysis-budget explanation；
- Quick Fix。

---

## Phase 1E：真实大型数据验证

至少测试：

```text
10 MB
100 MB
1 GB
```

测试：

- 字符串；
- Vec；
- 图像；
- 二进制；
- 数据库结果；
- HTTP response。

---

## Phase 2：类型与模块

- 泛型；
- trait；
- impl；
- where；
- module/use；
- FFI；
- 基础标准库。

---

## Phase 3：Concurrency + UI Joint Prototype

必须同时实现：

- Future；
- Task；
- `go{}`；
- `go!{}`；
- cancellation；
- channel；
- Send/Sync；
- UI Domain Operation；
- Component ID；
- State Slot；
- UI operation payload。

---

## Phase 4：编译器成熟度

- 增量编译；
- Semantic Summary Cache；
- debugger；
- LSP；
- Source Map；
- 性能分析；
- linker/toolchain integration。

---

## Phase 5：完整 UI

- `@state`；
- props；
- callback；
- render；
- layout；
- animation；
- hot reload；
- platform backend。

---

## Phase 6：平台与发布

- Windows；
- Linux；
- macOS；
- Android；
- iOS；
- single；
- split；
- bundle；
- portable；
- installer；
- signing；
- update。

---

## Phase 7：Language Veil 与生态

- Registry；
- mirror；
- offline build；
- localized projection；
- localized diagnostics；
- package ecosystem。

---

# 七十一、Phase 1 必须通过的测试

## 71.1 跨函数

```text
A → B → C → Field
```

必须能够正确组合 Semantic Summary。

---

## 71.2 跨模块

不同模块：

```text
A.aine
B.aine
C.aine
```

不得因模块边界自动导致所有大型值 Materialize。

---

## 71.3 动态索引

复杂：

```text
Vec
loop
dynamic index
conditional selection
```

不能错误通过，也不能无限分析。

---

## 71.4 大对象

对于：

```text
100 MB
1 GB
```

必须明确：

- 是否借用；
- 是否物化；
- 为什么；
- 成本多少；
- 如何进入高级优化路径。

---

## 71.5 闭包

验证：

- Borrow；
- Copy；
- Move；
- 逃逸；
- Task capture。

---

## 71.6 Task

验证：

- Send；
- Capture；
- Cancellation；
- Detached；
- Error propagation。

---

## 71.7 UI

验证：

- State lifetime；
- State write；
- Task → UI；
- Component destruction；
- Stale UI Operation；
- Hot reload。

---

# 七十二、Phase 3/5 联合验收标准

核心案例：

```flow
Button("查询") {
    go {
        let data = db.query(...)?

        ui {
            records = data
        }
    }
}
```

必须保证：

```text
0 dangling reference
0 data race
0 illegal state access
1 ownership transfer
0 unnecessary full-data copy
```

---

# 七十三、认知负担验收标准

测试对象：

> 没有 Rust 经验的普通开发者。

必须能够完成：

- 字段访问；
- 返回值；
- 多来源选择；
- 集合操作；
- map/filter；
- 闭包；
- UI 回调；
- 文件访问；
- SQLite；
- HTTP；
- 后台任务。

目标：

> **90% 以上的普通应用开发任务不要求用户主动理解生命周期、借用规则或生命周期参数。**

---

# 七十四、性能验收标准

必须统计：

```text
Borrow Success
Materialization Count
Large Copy Count
Allocation Count
Summary Composition Cost
Ownership Analysis Time
Incremental Rebuild Time
```

特别关注：

```text
10 MB
100 MB
1 GB
```

数据。

---

# 七十五、编译时间原则

分析必须：

- 可增量；
- 可缓存；
- 有预算；
- 有摘要；
- 可失效；
- 超预算可回退。

不能出现：

> 为证明一处零拷贝而让整个项目重新做全程序生命周期分析。

---

# 七十六、失败模式

如果某项能力无法在合理成本内实现：

### 不允许

- 随机放宽安全规则；
- 运行时偷偷 GC；
- 偷偷大规模 Clone；
- 让 AI 猜测；
- 要求普通用户学习生命周期参数。

### 允许

- Materialize；
- 给性能诊断；
- 建议拆分函数；
- 建议显式 `&T`；
- 建议进入高级 API；
- 暂时放弃该优化。

---

# 七十七、V5.8 最终五条核心原则

> **第一：普通开发者面对值，不面对生命周期。**

> **第二：Owned 是稳定的用户语义，Borrow/View 是编译器可以选择的实现优化。**

> **第三：跨函数、跨模块、任务和 UI 的数据流统一由 Value Aine Graph / VIR 分析。**

> **第四：后台任务不得持有 UI State 普通借用；`ui {}` 是携带 Owned/Send 数据的 UI Domain Operation。**

> **第五：编译器安全性必须由确定性算法保证，AI 永远不是语言或编译器的信任根。**

---

# 七十八、V5.8 状态声明

Aine V5.8 定义为：

> **实现前核心定稿版。**

本版正式冻结：

- 产品定位；
- 用户复杂度边界；
- Owned 语义；
- `String` 语义；
- Compiler View；
- Materialization；
- 大对象复制策略；
- `#[return_from]` 删除；
- 生命周期不进入普通语法；
- Function Semantic Summary；
- Summary Composition；
- Value Aine Graph；
- VIR；
- 跨函数/跨模块分析预算；
- `Future/Task/go/go!`；
- Send/Sync 边界；
- `@state`；
- `ui {}`；
- UI Domain Operation；
- Component Identity；
- Task → UI 数据传输；
- 确定性诊断；
- Optimization Explain；
- Language Veil 的投影性质；
- AI 不进入编译器和默认工具链。

本版本之后：

> **原则上不再增加新的核心语言概念，除非 Phase 1/3 原型证明现有模型无法解决真实任务。**

后续工作的判断标准从：

> “Aine 还能不能更强？”

改为：

> **“这个修改能不能让真实应用更容易写，同时不把复杂度重新转嫁给普通开发者？”**

---

# 七十九、Aine 的最终技术假设

Aine 最终要验证的不是：

> “能不能设计出一门看起来比 Rust 简单的语言。”

而是：

> ### **安全的拥有值语义能否成为普通开发者稳定的心智模型，而编译器通过 Value Aine、Semantic Summary、Borrow/View、Escape Analysis 和确定性优化，在不依赖 GC/ARC/AI 的情况下，将其中大量操作实现为零拷贝、零额外分配或低成本原生代码。**

如果这一假设成立：

> **Aine 就不是“简单版 Rust”，而是一种不同的语言设计路线。**

如果这一假设经过原型验证不成立：

> Aine 也应明确记录失败原因，而不是通过越来越复杂的隐式机制掩盖问题。

因此 V5.8 的下一步不是继续添加语法，而是：

```text
Value Aine Core
        ↓
VIR
        ↓
Semantic Summary
        ↓
Cross-module Composition
        ↓
Materialization Cost Model
        ↓
Task / UI Domain
        ↓
真实应用 Benchmark
```

**至此，Aine 的设计阶段基本收口，正式进入“编译器能否证明这套设计成立”的工程验证阶段。**