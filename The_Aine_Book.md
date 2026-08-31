# The Aine Book

> Aine 语言入门到进阶(第 1-15 章,至 SQLite/HTTP)。
> 配套文档:《Aine Language Reference》(语言参考)、《Aine_UI_Guide》(UI 指南)、
> 《Aine_Concurrency_Guide》(并发指南)。
> 所有示例均来自 examples/ 目录的真实可运行代码(可复制运行验证)。

---

## 第 1 章 欢迎与安装

Aine 是一门面向应用开发的新语言:静态类型、所有权安全(无悬垂/无数据竞争)、
结构化并发、第一方声明式 UI,编译为原生可执行(Windows 优先)。

**安装**:解压发布包,`aine.exe` 加入 PATH。验证:

```text
$ aine --version
aine 0.1.0

$ aine run examples/fib.aine
fib(0) = 0
fib(1) = 1
...
```

**命令行**(完整工具链):

| 命令 | 用途 |
|---|---|
| `aine run <file>` | 解释执行 |
| `aine check <file>` | 词法+语法+名称解析检查 |
| `aine build <file>` | 编译为原生可执行(C 后端 + zig cc) |
| `aine test <file>` | 运行 `@test` 测试 |
| `aine fmt <file>` | 规范格式化 |
| `aine vf <file>` | 值流分析(VIR + 摘要 + §72 验收指标) |
| `aine profile <file>` | 性能分析报告 |
| `aine debug <file> <行号...>` | 断点执行 + 局部变量 |
| `aine lsp` | 语言服务器(vscode/neovim) |

---

## 第 2 章 你好,Aine

```aine
fn main() {
    print("你好,Aine")
}
```

- 程序入口是 `fn main()`。
- `print(...)` 输出一行。
- 源码后缀 `.aine`,注释 `//`。

**格式化**:`aine fmt hello.aine` 输出规范格式(4 空格缩进、最小括号)。

---

## 第 3 章 基本语法

### 变量

```aine
fn main() {
    let x = 5          // 不可变绑定
    var total = 0      // 可变绑定
    total = total + x
    let name: String = "flow"   // 显式类型注解
    print(f"total={total} name={name}")
}
```

### 函数

```aine
fn fib(n: i32) -> i32 {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}
```

函数尾表达式自动返回(`return` 可省略)。

### 控制流

```aine
fn classify(n: i32) -> String {
    if n > 0 {
        "positive"
    } else {
        "non-positive"
    }
}
```

`if` 是表达式。循环:`while` / `for`:

```aine
var i = 0
while i < 3 {
    print(i)
    i += 1
}
for x in [10, 20, 30] {
    print(x)
}
```

---

## 第 4 章 类型系统

基础类型:`i32`、`i64`、`f64`、`bool`、`char`、`String`。

复合:`Vec<T>`、`Map<String, V>`、`Option<T>`、`Result<T, E>`、`fn(A) -> R`。

```aine
struct User {
    name: String
    age: i32
}

fn describe(u: User) -> String {
    return u.name + f"({u.age})"
}

fn main() {
    let u = User { name: "aina" age: 3 }
    print(describe(u))
}
```

`i64` 映射 `long long`;`String` 映射 `char*`(所有权由编译器管理)。

---

## 第 5 章 字符串与文本

```aine
fn main() {
    let s = "hello, aine"
    print(s.len())              // 13
    print(s[0])                 // 'h'（字符）
    print(s[7..11])             // "aine"（切片）
    print(s.to_upper())         // 大写（stdlib strutil）
    print(f"len={s.len()}")     // f-string 插值
}
```

f-string 支持方法链插值:

```aine
let nums = [1, 2, 3]
print(f"sum={nums.iter().map(x => x).sum()}")   // sum=6
```

---

## 第 6 章 集合

```aine
fn main() {
    var v: Vec<i32> = []
    v.push(1)
    v.push(2)
    print(v.len())        // 2
    print(v[0] + v[1])    // 3

    let m = Map.new()
    m.set("a", 1)
    m.set("b", 2)
    print(m.get("a"))     // 1
    print(m.contains("c")) // false
    print(m.keys().len())  // 2
}
```

---

## 第 7 章 结构体与枚举

```aine
enum Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn area(s: Shape) -> f64 {
    match s {
        Circle(r) { 3.14 * r * r }
        Rect(w, h) { w * h }
    }
}

fn main() {
    let c = Circle(2.0)
    print(area(c))     // 12.56
}
```

枚举是带标签联合;match 模式匹配(支持数字字面量模式与嵌套)。

---

## 第 8 章 模式匹配

```aine
fn describe(o: Option<i32>) -> String {
    match o {
        Some(x) {
            let s = "got"
            s + f" {x}"
        }
        None { "none" }
    }
}
```

- 块式臂 `Pat { body }`;函数尾 match 的臂体尾值自动返回。
- 整数字面量模式:`match n { 0 { "zero" } _ { "many" } }`。

---

## 第 9 章 错误处理

```aine
fn parse_num(s: String) -> Result<i32, String> {
    let n = s.to_i32()
    if n > 0 {
        Ok(n)
    } else {
        Err("需要正数")
    }
}

fn main() -> Result<(), String> {
    let n = parse_num("42")?    // 失败则提前返回 Err
    print(n)
    return Ok(())
}
```

`?` 传播错误;main 返回 `Result` 时 `Err` 映射退出码 1。

---

## 第 10 章 模块与包

```aine
// math.aine（同目录模块）
fn double(x: i32) -> i32 {
    return x * 2
}

// main.aine
module math;

fn main() {
    print(math.double(21))     // 42
}
```

- `module name;` 引入同目录(或 stdlib/)模块,扁平注册。
- 包配置 `aine.toml`:`[package] name / version / entry`。
- 标准库:`math`、`strutil`、`collections`、`json`、`io`、`time`、`path`。

---

## 第 11 章 闭包与高阶函数

```aine
fn main() {
    let f = (x: i32) => x * 2
    print(f(21))               // 42

    let nums = [1, 2, 3]
    print(nums.iter().map(x => x * 10).sum())   // 60
}
```

- `x => expr` 闭包(捕获独立值经隐藏参数提升)。
- 方法链 `iter().map().sum()` 折叠为单循环。

---

## 第 12 章 所有权与值语义

Aine 值语义:复合值(Vec/Map/用户结构)移动或克隆由编译器判定,
无悬垂引用、无数据竞争。

```aine
fn main() {
    var v = [1, 2, 3]
    let w = v          // v 不再使用 → move（零拷贝）
    print(w.len())
}
```

诊断工具:`aine vf` 输出所有权错误、物化提示与 §72 联合验收指标。

---

## 第 13 章 并发

```aine
fn main() {
    go {
        print("后台任务")
    }
    print("主流程继续")
}
```

- `go {}`:结构化任务(所有权安全边界)。
- `go! {}`:分离任务。
- 跨任务值自动 Send 检查(字段组合推断;`@not_send` 显式标记)。
- UI 规则:任务内计算结果经 `ui {}` 写回状态。

```text
$ aine vf app.aine
== §72 联合验收 ==
  dangling reference:        0（目标 0）
  data race:                 0（目标 0）
  illegal state access:      0（目标 0）
  ownership transfer:        1
  unnecessary full copy:     0（目标 0）
```

---

## 第 14 章 UI

```aine
ui Counter {
    title = "计数器"

    @state
    var count: i32 = 0

    render {
        Column {
            Text(f"计数: {count}")
            Button("加一") {
                go {
                    // 后台计算...
                    ui {
                        count = count + 1
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), String> {
    run_ui(Counter)
    return Ok(())
}
```

- `ui Name { @state ... render {...} }`:声明式组件。
- `@state`:组件状态(读写只能在 UI Domain 内)。
- 事件回调:按钮点击执行回调;回调内 `go{}` 做后台工作,`ui{}` 写回状态。
- 规则由编译器检查:`ui{}` 内禁止 `go{}`、`@global` 只在 `ui{}` 内写、跨任务值必须 Send。
- 渲染(终端运行时):`aine run` 输出组件树与状态摘要。

---

## 第 15 章 工具链与下一步

**测试**(`@test` 属性):

```aine
@test
fn add_works() -> Result<i32, String> {
    let r = 1 + 2
    if r == 3 { Ok(r) } else { Err("bad") }
}
```

```text
$ aine test examples/testdemo.aine
```

**格式化**:`aine fmt` 幂等且与 Flow 原生格式化工具一致。

**IDE**:`aine lsp` 提供诊断/悬停/补全/跳转(vscode/neovim 扩展接入)。

**调试**:`aine debug app.aine 5 12` 断点执行并报告局部变量。

**下一步(SQLite/HTTP)**:标准库将扩展数据库与网络模块
(经 C 内建扩展),应用形态:UI 事件 → go{} → SQLite/HTTP → ui{} → 状态更新。

**发布**:`tools/package.sh` 生成 single / portable 发布物。

---

> 全书完。练习:把第 14 章的计数器扩展为记账本(参见 examples/account_book.aine)。
