# Awen → Aine 移植记录

> 移植日期:2026-09-04
> Aine 工具链:0.1.0(解释器模式)
> 测试结果:30/30 全绿;corpus 136 块 0 词法错误

## Aine 解释器 Bug / 已知限制

### 1. match 臂内 continue/break 不生效(已修复)

**状态**:已由用户修复(commit 6b95c13),文档已统一(GRAMMAR v1.1)。
**影响**:词法器主循环和 lw_atomize 中依赖 match 臂内 continue 的控制流静默失效。
**workaround**(旧工具链):全部 continue/break 退出 match 臂,改为旗标 + if 层 continue。
**建议**:新工具链构建后,可将旗标模式改回 idiomatic 写法(当前代码仍保留旗标注释,便于对照)。

### 2. `Option.unwrap()` 对所有类型返回 Nil

**状态**:未修复(截至 0.1.0 dist 版本)。
**复现**:
```aine
let a: Option<i32> = Some(42)
print(a.unwrap())  // → <nil>
```
**workaround**:全部 `.unwrap()` 替换为 `match opt { Some(v) { v } None { ... } }`。

### 3. `Vec.push()` 在新工具链(含 continue 修复)中返回 `()`

**状态**:新构建的 `target/release/aine.exe` 和 `aine-studio.exe` 均受影响。
**复现**:
```aine
var v: Vec<i32> = []
v = v.push(42)
print(v)  // → ()
```
**影响**:新工具链完全不可用,必须使用 `dist/aine-0.1.0-single/aine.exe`(Aug 31 版本)。
**根因**:疑似 continue 修复引入的回归,影响所有类型(内建/自定义枚举/结构体)。

### 4. `Text` 是 Aine UI 内建组件名,枚举变体名冲突

**状态**:Aine 设计如此,非 bug。
**复现**:定义 `enum Foo { Text(String) }` 后 `Text("x")` 返回 Nil。
**影响**:Inline 枚举不能使用 `Text` 作为变体名;已重命名为 `AwTxt`。
**注意**:函数内定义的同名枚举变体也会被吞掉。

### 5. `IText` 等含 "Text" 子串的变体名在模块函数内部返回 Nil

**状态**:未确认是否为 bug 或设计限制。
**复现**:`enum Inline { IText(String) }` 在模块 `lightweight.aine` 内的 `lw_resolve` 函数中,`IText("x")` 返回 Nil;但从外部调用 `IText("x")` 正常。
**影响**:`lw_resolve` 中所有构造 `AwTxt` 的位置都必须通过包装函数 `mk_text(s)` 间接构造。
**workaround**:
```aine
fn mk_text(s: String) -> Inline {
    return AwTxt(s)
}
// lw_resolve 内部:
children.push(mk_text(m.literal))  // 而非直接 AwTxt(m.literal)
```

### 6. 枚举变体的字段访问在 `for` 迭代器上下文中返回 Nil

**状态**:未确认。
**复现**:`for item in vec_of_inline { match item { Command(u) { u.cmd } } }` — `u.cmd` 返回 Nil。
**workaround**:通过 `extract_cu(b)` 辅助函数从 `LexedBlock` 中提取 `CommandUse`,或对 `Block` 值做二次 match:
```aine
fn extract_cmd_from_block(blk: Block) -> ExplicitCommand {
    match blk { Object(u) { return u.cmd } _ { return Code } }
}
```

### 7. `for` 迭代器在含 Nil 元素的 Vec 上 match 报 "match 未匹配任何分支"

**状态**:未确认。
**复现**:Vec 中含 Nil 元素时,`for x in vec { match x { ... } }` 即使有 `_` catch-all 也报错。
**workaround**:改用 `while i < vec.len()` + 显式索引访问。

## 设计决策(移植期裁定)

### 行内标记不跨物理行

Awen 规范未规定行内标记是否跨行;移植时裁定为**不跨行**。理由:
- Aine 解释器逐行处理更简单
- 跨行需求可走显式语法 `@[bold]多行内容@[/bold]`

### `collect_inline_cmds` 添加 `_` catch-all

原始实现只匹配已知 Inline 变体;因 lw_resolve 遗留 Nil 值,添加 `_` arm 兜底。不影响正确性(Nil 值来自 UI 内建冲突的残留,新工具链修复后可移除)。

## 文件结构

```text
E:\Flow\awen-proto\
├── src/
│   ├── main.aine          演示入口:加载 corpus 并输出统计
│   ├── tests.aine         30 个验收测试(运行:aine test src/tests.aine)
│   ├── diag.aine          诊断(规范 §6.10 引导提示)
│   ├── registry.aine      显式命令白名单(§11,含中文别名)
│   ├── fullwidth.aine     全角归一化(§6.9–6.10,上下文敏感封闭表)
│   ├── escape.aine        转义(§6.8,@@[ 优先级最高)
│   ├── explicit.aine      命令头解析(§6.1/§11:参数/属性/原文内容)
│   ├── lightweight.aine   轻量层分类+行内扫描(§6.2–6.7/§6.11–6.13)
│   ├── lexer.aine         词法编排:逐行分类+Raw/表格状态机(§6/§9/§11)
│   └── seed.aine          基础类型(§2.2 Label/§3.3 Span/§4 Buffer/Patch)
├── corpus/
│   ├── all_syntax.awen    全冻结语法正样本
│   └── edge_cases.awen    消歧/字面/预期错误样本
└── docs/
    └── PORT_NOTES.md      本文
```

## 运行

```bash
cd E:\Flow\awen-proto

# 测试(30 个)
E:\Flow\flowc\dist\aine-0.1.0-single\aine.exe test src/tests.aine

# 演示
E:\Flow\flowc\dist\aine-0.1.0-single\aine.exe run src/main.aine
```

⚠ **必须使用 `dist/aine-0.1.0-single/aine.exe`**(Aug 31 版本)。新构建的 `target/release/aine.exe` 因 `Vec.push()` 回归不可用。
