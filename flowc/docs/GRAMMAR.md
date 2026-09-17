# Aine 语言文法快照(Grammar Snapshot)v1

> 2026-09-04。以**实现为准**冻结(双前端:Rust 宿主 `src/parser.rs` 与自举 `examples/alparse.aine` 已对齐)。
> 本文档用于消除历史"语法书"与现行文法的版本出入。文法变更必须同时修改本文档与两套前端。

## 1. 闭包(Closure)— 唯一文法:箭头形态

```text
closure   ::=  ident "=>" expr
            | "(" ")" "=>" expr
            | "(" param ("," param)* ")" "=>" expr
param     ::=  ident (":" type)?
```

合法示例:
```aine
let f1 = x => x + 1
let f2 = (a, b) => a + b
let f3 = (x: i32) => x * 2
let f4 = () => print("hi")
```

**竖线形态 `|x| expr` / `|x: T| expr` 永久排除**(2026-09-04 拍板):
`|` 已是按位或、`||` 是逻辑或,引入竖线闭包会产生表达式歧义成本,且箭头
形态表达力完全覆盖。宿主 parser.rs 中的旧注释残留已修正;若旧语法书仍
描述竖线闭包,以本快照为准。

## 2. match 臂 — 双形式并存(均可,可混用)

```text
match_expr  ::=  "match" expr "{" arm* "}"
arm         ::=  pattern "=>" expr
              |  pattern "=>" "{" stmt* "}"
              |  pattern "{" stmt* "}"          # 块式臂(G2.0 ⑤)
pattern     ::=  字面量 | ident | 通配 "_" | 变体模式(如 Some(x) / Circle(r))
```

合法示例(三种臂形):
```aine
match s {
    Circle(r)  => 3.14 * r * r          // 箭头 + 表达式
    Rect(w, h) => { return w * h }      // 箭头 + 块
    Line       { print("line") }        // 块式臂
    _          { print("other") }
}
```

- 块式臂内 `break` / `continue` / `return` 向上传播(宿主 2026-09-04 修复:
  语句形/块尾 match 此前会静默吞掉 break/continue,见 commit 6b95c13)。
- 臂体若以语句起始关键字(return/let/…)开头,箭头形式自动包块。

## 3. 其他易混点(与旧语法书的差异)

| 主题 | 现行文法 | 旧/错误写法 |
|---|---|---|
| 可变绑定 | `var x = …`(let mut 已移除) | `let mut x` |
| 顶层全局 | `@global var name: T = init` | 裸顶层 `var`(不允许) |
| UI 块 | `ui { Window(...) { … } }` / `go { … }` | — |
| f-string | `f"{expr}"`(整体一个字符串 token,插值走完整表达式) | — |
| 区间 | `a..b`(for 与切片共用) | — |

## 4. 双前端一致性纪律

任何文法改动必须:① 同时改 `src/parser.rs` 与 `examples/alparse.aine`;
② 跑双引擎对照(mini 套件 + transpiler 基线);③ 更新本文档版本号与日期。
