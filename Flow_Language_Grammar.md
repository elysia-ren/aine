# Flow 语言语法规范（Grammar Specification）

> **版本**：G1.0（M1 定稿，与 flowc 0.1.0 实现同步）
>
> **地位**：本规范是 Flow 语言的**正式语法根基**。词法分析、语法分析、HIR、类型检查、格式化器均以本规范为准。
> 对应设计稿：§54 核心语法、§55 基本语法；交付物：12 Language Reference、21 Specifications（Language Specification）。
>
> **记号约定**（EBNF 扩展）：
>
> - `A ::= B C` 产生式；`A | B` 选择；`[ A ]` 可选；`{ A }` 零次或多次；`A ,` 表示逗号分隔列表
> - 终结符用小写字母/符号（`'fn'`、`'('`）；非终结符用 PascalCase
> - `⟨换行⟩` 是软语句终结符（见 §3 消歧规则）；`{ A }` 内换行通常可忽略
> - 每条产生式标注【实现】：对应 flowc 源码位置与设计稿章节
>
> 本规范 vG1.0 覆盖 flowc 当前已实现构造；未实现的语法（如完整的 where 子句语义）标注为 ⏳。

---

## 1. 词法文法（Lexical Grammar）

> 【实现】src/lexer.rs、src/token.rs

### 1.1 字符与空白

```ebnf
Whitespace ::= (' ' | '\t' | '\r' | ⟨换行⟩)+
⟨换行⟩      ::= '\n'          (* 语句软终结符，见 §3 *)
```

### 1.2 注释

```ebnf
Comment      ::= LineComment | BlockComment
LineComment  ::= '//' { ⟨除换行外任意字符⟩ }
BlockComment ::= '/*' { CommentBody } '*/'    (* 可嵌套 *)
CommentBody  ::= BlockComment | ⟨任意字符⟩
```

### 1.3 标识符与关键字

```ebnf
Ident ::= (Letter | '_') { Letter | Digit | '_' }
Letter ::= 'a'..'z' | 'A'..'Z'
Digit  ::= '0'..'9'

Keyword ::= 'fn' | 'let' | 'mut' | 'const' | 'if' | 'else' | 'for' | 'in'
          | 'while' | 'return' | 'break' | 'continue' | 'match'
          | 'struct' | 'enum' | 'type' | 'trait' | 'impl' | 'where'
          | 'export' | 'module' | 'import' | 'move' | 'unsafe' | 'extern'
          | 'try' | 'catch' | 'ui' | 'render' | 'go' | 'defer'
          | 'true' | 'false'
```

> 关键字列表与设计稿 §54 一致（+ go/defer/import/true/false，均为设计稿正文使用）。关键字不可作标识符。

### 1.4 字面量

```ebnf
IntLiteral    ::= DecInt | HexInt | BinInt | OctInt
DecInt        ::= Digit { Digit | '_' }
HexInt        ::= '0x' HexDigit { HexDigit | '_' }
BinInt        ::= '0b' ('0'|'1') { ('0'|'1') | '_' }
OctInt        ::= '0o' '0'..'7' { '0'..'7' | '_' }
HexDigit      ::= '0'..'9' | 'a'..'f' | 'A'..'F'

FloatLiteral  ::= Digit { Digit | '_' } '.' Digit { Digit | '_' } [ Exponent ]
                | Digit { Digit | '_' } Exponent
Exponent      ::= ('e'|'E') ['+'|'-'] Digit { Digit }

StrLiteral    ::= '"' { StrChar | Escape } '"'
FmtStrLiteral ::= 'f' '"' { StrChar | Escape } '"'    (* 插值 '{}' 在求值阶段解析 *)
StrChar       ::= ⟨除 '"'、'\\'、换行外任意字符⟩
Escape        ::= '\\' ('n' | 't' | 'r' | '0' | '\\' | '"' | '\'' | 'x' HexDigit HexDigit)
```

> 词法错误码：F1001 未闭合字符串、F1002 未闭合块注释、F1003 非法字符、F1004 非法/溢出数字。

### 1.5 运算符与标点

```ebnf
Punct ::= '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' | '::' | '.'
        | '..' | '->' | '=>' | '@' | '?' | '!'
        | '+' | '-' | '*' | '/' | '%' | '=' | '==' | '!=' | '<' | '<=' | '>' | '>='
        | '&&' | '||' | '&' | '|' | '^' | '~' | '<<' | '>>'
        | '+=' | '-=' | '*=' | '/=' | '%='

Decorator ::= '@' Ident    (* @state / @global，设计稿 §33/§44 *)
Attribute ::= '@' Path [ '(' AttributeArg { ',' AttributeArg } ')' ]    (* G2.0-② 已实施：#[...] 形态移除 *)
```

---

## 2. 语法文法（Syntactic Grammar）

> **定位**：采用 Rust 风格现代语法，但由编译器自动承担所有权复杂度、
> 面向原生应用开发的独立语言（Rust-like syntax, compiler-derived ownership,
> application-first semantics）。独立性由 Value Flow / Compiler View /
> Semantic Summary / Task·UI Domain 的语义层证明，不依赖表面语法差异。
> ## 语法版本标注（消除双规范歧义）
>
> - **G1.0（本文件正式 EBNF）= 当前实现语法**，已实施项直接落入产生式
>   （② 属性统一 `@` 形态已落地，`#[...]` 产生式已删除）；
> - **G2.0 = 迁移中变更**，仅存在于下方决议表，逐项标注状态
>   （①③④⑤ 未实施，正文 EBNF 维持 G1.0 旧形态直至迁移落地）；
> - 每项落地时同步改写产生式并移除本标注对应行——**任何时刻本文件只描述一套可执行语法**。

> ## G2.0 语法演进决议（定稿，分阶段迁移）

> 目标：**保留 Rust 的熟悉感，消除 Rust 的语言指纹堆叠**；常见代码表面相似度
> 预期 85% → **70～75%**（含 trait/impl 改名则约 68～75%）。独立性由语义层
> 证明，不靠"长得不像 Rust"。以下 5 项为定稿变更，按阶段迁移（每项均需
> Rust 侧与 flparse 自宿主双侧同步 + stdlib/examples/tests 全库迁移）：
>
> | # | 变更 | 旧 | 新 | 推荐度 |
> |---|------|----|----|--------|
> | 1 | 可变绑定 | `let mut x = 0` | `var x = 0`（`let` = 不可变） | ★★★★★ **已实施 ✓** |
> | 2 | 属性统一 | `#[display(...)]` / `@state` | 全部 `@` 形态：`@display(...)` `@test` `@unsafe` | ★★★★★ **已实施 ✓** |
> | 3 | 模块三元组 | `pub` `use` `mod` | `export` `import` `module` | ★★★★★ **已实施 ✓** |
> | 4 | 闭包 | `\|x: i32\| x * 2` | `x => x * 2`（多参 `(a, b) => a + b`；与尾部块联动重设计） | ★★★★☆ || 4 | 闭包 | `\|x: i32\| x * 2` | `x => x * 2`（多参 `(a, b) => a + b`；与尾部块联动重设计） | ★★★★☆ |<br>**联动设计**：`=>` 与 match 臂共用，消歧靠位置（模式位 vs 表达式位）；
建议与 ⑤ 同批落地或 ⑤ 先行，避免 `=>` 双义过渡期；参数类型注解形态
`(x: i32) => x * 2` 保留；零参 `() => ...`。
> | 5 | match 臂 | `Pat => Expr` | 块式 `Pat { body }`（与 `Button(...) { }` 视觉统一） | ★★★★☆ |
>
> **保留清单（明确不改）**：`?`（错误传播体验成熟，不与 try/catch 混淆）、
> `&T/&mut T`（高级借用层，普通代码低频）、`fn/struct/enum`（现代静态语言
> 共同语法）、enum 变体无 `case` 前缀（保持现状）、Pratt 优先级 + 软换行。
> **择机候选**：`trait → interface`、`impl → extend User: Trait`（★★★★☆，
> 需与类型系统术语一起评估，暂缓）。
> **视觉身份证（继续强化）**：`go` / `go!` / `ui` / `render` / `@state` /
> `@global` / `import` / `export` / `var` / 块式 match / 尾部块。
> 【实现策略】单特性特性开关式迁移：keywords 双侧兼容期 → 全库重写
> （stdlib → examples → conformance → tests）→ fmt/diagnostics 同步 →
> 兼容期结束后移除旧形态。每步保持 27+29 用例与 fixpoint 全绿。

> 【实现】src/parser.rs。语句默认以换行终结（§3），块内语句无需分号。

### 2.1 程序与项目

> 内建全局（无需声明）：`print` `read_file` `write_file` `append_file`
> `file_exists` `now` `assert(cond[, msg])`（断言内建，失败即确定性 panic
> 且消息含函数栈）`cli_args`（命令行实参，flowc run/build 注入）等。

```ebnf
Program ::= { Item }

Item ::= [ Attribute ] [ 'export' ] [ 'unsafe' ] ( ExternBlock | ItemKind )
ItemKind ::= FnDef | StructDef | EnumDef | TypeAlias | TraitDef | ImplDef
           | UiDef | ModDef | Import | GlobalDef

Import   ::= 'import' Path                    (* 声明标记：名字已存在时引用之；
                                                   自举期扁平注册下无命名空间效果 *)
ModDef    ::= 'module' Ident '{' { Item } '}'   (* 内联模块 *)
            | 'module' Ident ';'               (* 文件模块：读入 Ident.flow（搜索路径
                                                  examples/ → stdlib/），项原名扁平注册 *)
StructDef ::= 'struct' Ident [ '<' Ident { ',' Ident } '>' ]
              '{' { StructField } '}'
StructField ::= Ident ':' Type            (* 换行或逗号分隔 *)
EnumDef   ::= 'enum' Ident [ '<' Ident { ',' Ident } '>' ]
              '{' { EnumVariant } '}'
EnumVariant ::= Ident [ '(' Type { ',' Type } ')' ]
TypeAlias ::= 'type' Ident '=' Type
TraitDef  ::= 'trait' Ident [ '<' Ident { ',' Ident } '>' ] '{' { FnSig } '}'
ImplDef   ::= 'impl' [ Path 'for' ] Type '{' { Item } '}'
ExternBlock ::= 'extern' StrLiteral? '{' { FnSig } '}'
GlobalDef ::= '@' 'global' ( 'var' | 'let' ) Ident [ ':' Type ] [ '=' Expr ]

FnDef ::= 'fn' Ident [ '<' Ident { ',' Ident } '>' ]
          '(' { Param ',' } ')' [ '->' Type ] [ 'where' ... ⏳ ] Block
FnSig ::= 'fn' Ident '(' { Param ',' } ')' [ '->' Type ]    (* trait / extern 内 *)
Param  ::= [ 'move' ] Ident [ ':' Type ]                    (* 设计稿 §22 *)

UiDef ::= 'ui' Ident '{' { UiMember } '}'
UiMember ::= UiProp | StateDecl | InitDecl | RenderDecl
InitDecl ::= '@' 'on_init' FnDef                       (* V5.8 UI 生命周期 *)
RenderDecl ::= 'render' Block
UiProp ::= Ident '=' Expr                                    (* title = "..." *)
StateDecl ::= '@' 'state' 'var' Ident [ ':' Type ] [ '=' Expr ]
```

### 2.2 类型

```ebnf
Type ::= PathType | RefType | TupleType | FnType | 'Infer'
FnType ::= 'fn' '(' [ Type { ',' Type } ] ')' [ '->' Type ]   (* 统一函数类型 D14：
    具名函数/闭包同型；捕获即快照；作为形参时发射为 C 函数指针 *)
PathType ::= Ident [ '::' Ident ] [ '<' Type { ',' Type } '>' ]
             (* Vec<Record>, Result<(), E>, Map<String, Vec<T>> —— 嵌套泛型的 '>>'
                由词法/解析层拆分，用户按字面书写即可 *)
             (* 内建类型：i32 i64 u8..u64 usize f32 f64 bool char String
                Vec<T> Map<K,V> Option<T> Result<T,E>
                —— str 的类型地位（D5/G2.0 定稿）：String = 拥有字符串（默认）；
                str 仅作为 '&str' 的 pointee（高级借用字符串视图），
                不得单独作为变量/字段/参数/返回的拥有类型；
                '&str' = 高级借用视图（逃逸检查约束同 RefType）。*)
RefType  ::= '&' [ 'mut' ] Type          (* 设计稿 §22.3 高级借用层；返回需过逃逸检查，见下 *)
TupleType ::= '(' { Type ',' } ')'
```

> 语义约束（登记簿 D5 修订，两档模型）：
> - **普通模式**：所有权自动推导，无需书写返回借用；
> - **高级精确借用层**：`&T` 允许出现在高级函数签名与返回位置
>   （如 `fn get_name(user: &User) -> &str`），但必须通过编译器
>   完整的逃逸与借用检查；生命周期参数仍不可见（Language Veil 不破例）。

### 2.3 语句

```ebnf
Block ::= '{' { Stmt } [ Expr ] '}'      (* 末表达式为块尾值 *)
Stmt  ::= LetStmt | ReturnStmt | IfStmt | WhileStmt | ForStmt
        | BreakStmt | ContinueStmt | DeferStmt | TryCatchStmt | ExprStmt

LetStmt   ::= ( 'let' | 'var' ) Pattern [ ':' Type ] [ '=' Expr ]   (* let=不可变, var=可变；'@state' 仅限 ui 域 *)
ReturnStmt ::= 'return' [ Expr ]          (* 当前实现：仅同行取 Expr，换行视为空 return；
   改进待办（用户导向）：return 后下一行若以表达式起始 token 且非语句关键字
   （let/if/match/for/while/return），应续读为返回值；空行/块结束才视为空 return。
   需 Rust parser 与 flparse 双侧同步实现，防止既有早退 return 语义漂移 *)
IfStmt    ::= 'if' Expr Block [ 'else' ( IfStmt | Block ) ]
WhileStmt ::= 'while' Expr Block
ForStmt   ::= 'for' Pattern 'in' Expr Block
BreakStmt ::= 'break'
ContinueStmt ::= 'continue'
DeferStmt ::= 'defer' Expr
TryCatchStmt ::= 'try' Block [ 'catch' [ '(' Ident ':' Type ')' ] Block ]
ExprStmt  ::= Expr
```

### 2.4 模式

```ebnf
Pattern ::= Ident | '_' | IntLiteral | FloatLiteral | StrLiteral | 'true' | 'false' | Path
                        (* G1.0 边界：结构体/元组/数组/绑定模式为后续扩展（G2 候选）；
                           当前 match 的表面形态与 Rust 一致，模式能力弱于 Rust *)
```

### 2.5 表达式（按优先级从低到高）

```ebnf
Expr        ::= AssignExpr

AssignExpr  ::= RangeExpr ( AssignOp AssignExpr )?     (* 右结合 *)
AssignOp    ::= '=' | '+=' | '-=' | '*=' | '/=' | '%='

RangeExpr   ::= OrExpr [ '..' [ OrExpr ] ] | '..' OrExpr   (* 区间接入优先级链 *)
OrExpr      ::= AndExpr { '||' AndExpr }
AndExpr     ::= EqualityExpr { '&&' EqualityExpr }
EqualityExpr ::= RelExpr { ('==' | '!=') RelExpr }
RelExpr     ::= BitOrExpr { ('<' | '<=' | '>' | '>=') BitOrExpr }
BitOrExpr   ::= BitAndExpr { '|' BitAndExpr }
BitAndExpr  ::= BitXorExpr { '&' BitAndExpr }
BitXorExpr  ::= ShiftExpr { '^' ShiftExpr }
ShiftExpr   ::= AddExpr { ('<<' | '>>') AddExpr }
AddExpr     ::= MulExpr { ('+' | '-') MulExpr }
MulExpr     ::= UnaryExpr { ('*' | '/' | '%') UnaryExpr }
UnaryExpr   ::= ('-' | '!' | '&' | '*') UnaryExpr | PostfixExpr
PostfixExpr ::= Primary { PostfixOp }
PostfixOp   ::= '(' { CallArg ',' } ')'          (* 调用 *)
              | '.' Ident                        (* 字段 / 方法 *)
              | '[' Expr ']'                     (* 索引；Expr 为 Range 时为切片 s[a..b]，G1.1 增补 *)
              | '?'                              (* Try 传播，设计稿 §45 *)
              | '|' Param { ',' Param } '|' Expr (* 尾部闭包参数 *)
              | '{' Block '}'                    (* 尾部块（组件体）*)
CallArg     ::= [ Ident '=' ] Expr               (* 命名参数：placeholder = "..." *)
```

### 2.6 原子表达式

```ebnf
Primary ::= IntLiteral | FloatLiteral | StrLiteral | FmtStrLiteral | 'true' | 'false'
          | Ident
          | Path                              (* a::b *)
          | '(' Expr ')'                      (* 括号 *)
          | '(' Expr { ',' Expr } ')'         (* 元组 *)
          | '[' { Expr ',' } ']'              (* 数组 *)
          | Block
          | StructLit
          | Closure
          | IfExpr | MatchExpr
          | GoExpr | UiOpExpr
          | '..' Expr                         (* 半开范围：..n *)

StructLit ::= Ident '{' Ident ':' Expr { ',' Ident ':' Expr } '}'
Closure   ::= '|' [ ClosureParam { ',' ClosureParam } ] '|' Expr
            | '||' Expr                        (* 空参闭包 *)
ClosureParam ::= Ident [ ':' Type ]            (* 独立闭包值（let f = |x: i32| ...）
                                                  的参数必须带类型注解；方法链内联闭包可省 *)
(* 独立闭包值：let f = |x: i32| body 将 f 提升为可调用值（非捕获时编译器
   直接脱糖为顶层函数）；捕获外部变量时为 env 闭包（M6 原生发射计划中，
   解释器已完整支持）。*)
IfExpr    ::= 'if' Expr Block [ 'else' ( IfExpr | Block ) ]
MatchExpr ::= 'match' Expr '{' { MatchArm } '}'
MatchArm  ::= Pattern '=>' Expr
GoExpr    ::= 'go' [ '!' ] Block                (* 设计稿 §29/§30 *)
UiOpExpr  ::= 'ui' Block                        (* 设计稿 §36-41 *)
```

---

## 3. 消歧与软终结规则（Parser 实现约束）

> 【实现】src/parser.rs 中 `on_new_line` / `starts_continuation` / `allow_trailing`

1. **换行是软语句终结符**：语句在换行处结束，除非下一个 token 是续接运算符。
2. **续接运算符**：行首的 `. `、`(`、`[`、`?`、二进制运算符、`..`、尾部 `{`/`|` 继续上一表达式（支持 §69 的 `.iter()\n.map()\n.sum()` 链）。
3. **if/while/for/match 头部**：条件/迭代器/被匹配表达式之后紧跟的 `{` 属于块，**不得**被解析为尾部块（`allow_trailing=false`）。
4. **结构体字面量 vs 组件尾部块**：`Ident '{' Ident ':' ... ` 判为结构体字面量；否则判为组件调用尾部块。
5. **`return` 值（G2.0-⑥ 已实施 ✓，双侧落地）**：`return` 后若下一行以表达式起始 token 开头
   （字面量/Ident/`(`/`[`/`-`/`!`）且非语句关键字（let/var/if/match/for/while/return/break/continue），
   则续读为返回值；仅空行或块结束时视为空 return。早期早退 + 死代码模式不受影响。
6. **`go!`**：`go` 后紧跟 `!` 为 detached 任务（设计稿 §30）。
7. **`@state` 是两个 token**（`@` + 标识符），在 ui 定义内解析为状态声明。

---

## 4. 与实现的对应性（Conformance 表）

| 文法规则 | 实现位置 | 状态 |
|---|---|---|
| §1.1-1.2 空白/注释 | lexer.rs skip_trivia | ✅ |
| §1.3 关键字表 | token.rs keyword_from_ident | ✅ |
| §1.4 数字/字符串/f-string | lexer.rs scan_number / scan_string_body / decode_string | ✅ |
| §1.5 运算符/装饰器/属性 | lexer.rs next_token_kind；parser.rs parse_attribute | ✅ |
| §2.1 Program/Item | parser.rs parse_program / parse_item | ✅ |
| §2.1 Import/Struct/Enum/TypeAlias | parser.rs parse_import/struct/enum/type_alias | ✅ |
| §2.1 Trait/Impl/Extern/Mod | parser.rs parse_trait/impl/mod、parse_fn_sig | ✅（外部块⏳验证） |
| §2.1 FnDef/Param（含 move） | parser.rs parse_fn / parse_params | ✅ |
| §2.1 UiDef（props/state/render） | parser.rs parse_ui | ✅ |
| §2.1 GlobalDef | parser.rs parse_global_state | ✅ |
| §2.2 Type（泛型/Ref/元组） | parser.rs parse_type / parse_path_with_generics | ✅ |
| §2.3 Block/Stmt 全套 | parser.rs parse_block / parse_stmt | ✅ |
| §2.4 Pattern | parser.rs parse_pattern | ✅ |
| §2.5-2.6 表达式优先级表 | parser.rs parse_expr_mode（Pratt bp 表） | ✅ |
| §2.6 StructLit/Closure/Match/Go/UiOp | parser.rs（结构体字面量/parse_closure/parse_prefix） | ✅ |
| §3 消歧规则 1-7 | parser.rs on_new_line / starts_continuation / allow_trailing | ✅ |
| 名称解析 | resolve.rs（§56 Name Resolution） | ✅（M1.3） |

> ⏳ 待补：完整 where 子句语义（M1.4 类型检查）、f-string 插值求值（求值阶段）、完整 trait/impl 方法解析（Phase 2）。

---

## 5. 设计说明

1. **为什么换行是软终结符**：设计稿 §55「默认换行结束语句」，但 §69 示例要求跨行表达式（`.iter()` 链、多行调用）。软终结符 + 续接运算符同时满足两者。
2. **为什么尾部块/闭包**：§69 的 `Button("添加") { ... }` 与 `List(records) |r| { ... }` 是 UI 组件体的核心形态，归属调用参数（闭包），为 Phase 3 UI Domain Operation 做准备。
3. **为什么 if 头部禁用尾部块**：`if cond { }` 的 `{` 必须是块；若允许尾部块，条件表达式会吞掉整个分支体。
4. **结构体字面量判据**：`Ident { field: ... }` 中 `field:` 模式是唯一可靠消歧点（组件体不含 `ident :`）。
