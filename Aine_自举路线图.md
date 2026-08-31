# Aine 自举路线图（Self-Hosting Roadmap）

> **版本**：B1.0（2026-02-14）
>
> 目标：**最终用 Aine 语言实现 Aine 编译器本身**；中途尽量用 Aine 开发，
> 在过程中修复错误、优化语法。
>
> 当前基础：aine（Rust）已具备完整前端（词法→语法→HIR→解析→类型→值流）
> + 树遍历解释器（`aine run` 可执行 Aine 程序，134 测试）。

---

## 1. 自举的分层路径

```text
A. 参考实现（现在）        Rust aine：前端 + 分析 + 解释器
        │
B. Aine 表达力验证（本轮起）  用 Aine 写纯逻辑组件（lexer/fmt/json/...），
        │                  由 Rust 解释器执行 → 暴露语法缺口 → 优化语法
        │
C. Aine 写编译器前端         用 Aine 写 lexer/parser/HIR/类型/分析，
        │                  输出 Aine 可处理的数据（结构体/列表），由解释器运行
        │
D. Aine 写后端              用 Aine 写 codegen：
        │                  ① Aine → C 转译器（自举经典路径，C 编译器作后端）
        │                  ② 最终：Aine → LLVM IR（设计稿 D10）
        │
E. 自举闭环                 Aine 编译器（用 Aine 写）转译自身为 C →
        │                  C 编译器产出原生编译器 → 原生编译器编译自身 →
        │                  Rust aine 退化为引导工具
        │
F. 最终形态                 完整 Aine 编译器 100% Aine 实现，编译为原生，
                            能编译自身；语法在自举过程中持续打磨
```

## 2. 中途用 Aine 开发的内容（按价值排序）

| 顺序 | 组件 | 用途 | 暴露的语法需求 |
|---|---|---|---|
| 1 | lexer.aine | 词法分析器 | 字符串/字符操作、结构体列表、错误传播 |
| 2 | json.aine | JSON 解析器 | 递归、Option/Result 传播、Map 需求 |
| 3 | fmt.aine | 格式化器 | AST 遍历、递归、大型结构体 |
| 4 | symbols.aine | 符号表 | **Map/Dict 类型**（编译器刚需） |
| 5 | parser.aine | 语法分析 | **enum 变体 + match 模式**、递归下降 | B4-M1 表达式级已落地（parser_expr.aine） |
| 6 | 值流/摘要 | 静态分析 | 复杂数据流、缓存 |

## 3. 语法优化清单（自举驱动，按优先级）

| 优先级 | 缺口 | 动机 | 现状 |
|---|---|---|---|
| P1 | 字符串比较与字符操作 | lexer 需要 `c >= "0"`、字符类判断 | 解释器 compare 仅数值；需补 Str 比较 |
| P1 | 用户 enum 变体构造与 match 模式 | AST 需 `Expr::Binary` 式表示 | 解释器仅内置 Ok/Err/Some/None；自定义 enum 构造不可用 |
| P1 | Map/Dict 类型 | 符号表、作用域 | 无字典类型；先以 Vec<(String,V)> 过渡 |
| P2 | impl 方法执行 | 代码组织（结构体方法） | impl 已解析未执行 |
| P2 | 模块（mod）执行 | 大规模代码组织 | mod 已解析未执行 |
| P2 | 更多标准库（substring/split/replace...） | 编译器需要文本处理 | 方法表待扩 |

## 4. 执行纪律

- **每个 Aine 组件必须能 `aine run` 运行并有断言测试**——先证明能跑，再谈性能。
- **语法缺口先记入本清单，再最小化修改**（Rust 解释器/前端 + Aine 语法规范 G1.0 同步更新）。
- **Rust 实现是真相来源（参考编译器）**：Aine 组件行为必须与 Rust 参考一致（差分测试）。
- 最终形态前，**Rust aine 保留**：即使自举完成，也保留参考实现用于测试与回退。

## 5. 关键风险

- **性能**：解释器执行 Aine 编译器会很慢（lexer 自举时可为原始速度）。缓解：先保证正确，性能优化（字节码 VM/原生）后置。
- **语言表达力**：Aine 必须能表达指针/数组/字典等编译器数据结构。缓解：Map 类型 P1 立项。
- **范围蔓延**：自举期间不做 UI/并发 runtime（设计稿 Phase 3+ 保持 Rust/后续阶段）。

## 6. 当前里程碑

- [x] B0：Rust 参考实现（前端+分析+解释器，134 测试）
- [x] **B1：lexer.aine 用 Aine 写出并运行**（140 测试）—— Aine 成功表达词法分析器；
  暴露并修复 5 个实现缺陷（见下）
- [ ] B2：json.aine / fmt.aine
- [x] **B3 第一阶段：Map 类型 + 用户 enum 变体（145 测试）**—— 自举两大结构性能力落地：
  Map<String, V>（new/set/get/contains/len/keys/remove/values，set 可变重绑定）、
  用户 enum 变体构造（Number(5)）与 match 变体模式（Num(n) => ...，含嵌套递归）
- [x] **B3 第二阶段：impl 方法执行 + 模块执行（149 测试）**：
  impl Type { fn ... } 实例方法（self 作为首参绑定、字段访问、f-string 插值）、
  mod 扁平注册（函数/结构体跨模块可见）、限定名结构体字面量 g.P { ... }；
  修复 parse_impl 丢弃 self 类型的解析 bug
- [x] **B2：json.aine（Aine 写 JSON 解析器，155 测试）**：
  对象/数组/字符串（转义）/数字/布尔/null 递归下降解析 + 序列化 + 往返验证；
  写 Aine 过程中修复 8 个实现缺陷（? 传播、match 语句臂、Option/Result 模式、
  字符串拼接类型、发散臂、单元变体值/模式、None 值）——详见日志
- [x] **B4-M1：parser_expr.aine（表达式 Pratt 解析器，157 测试）**
- [x] **B4-M2：parser_stmt.aine（语句级解析器，159 测试）**：
  let(mut/类型注解)/if-else/while/for..in/return/赋值/块 + fn 定义，
  4 组自断言全过；暴露并修复表达式块内 return 不传播（RtError::Return）；
  Aine 无元组解构 → TyResult 结构体；新增 += 等复合赋值
- [x] **B4-M3：parser_stmt.aine 全量 items（162 测试）**：
  struct/enum/impl/mod + match 语句 + 空元组 + 泛型类型解析；
  修复解释器栈溢出（Rc<FnDef> + 512MB 栈线程）
- [x] **B4-M4：parser_stmt.aine 解析真实 Aine 程序（自举验证）**：
  注释/f-string/结构体字面量/?/for-in 集合迭代；
  用例 9 解析 lexer.aine 源码；修复一元运算符 trailing 消费 bug
- [x] **B4-M5：parser_stmt.aine 终极自解析（11 用例）**：
  Aine 解析器解析自身类型定义与 parse_let 函数；
  补齐闭包/mut 形参/索引切片解析
- [x] **B5-M1：transpiler.aine（Aine 写 Aine→C 转译器）**：
  纯函数子集转译（fn/let/if/while/算术/递归/print→printf）；
  4 用例全过含自转译；enum/match/Vec→C 待 M2
- [x] **B5-M2：struct/enum/match → C 映射**：
  enum → tagged union；match → switch；模式绑定 → union 字段访问
- [x] **B5-M3：类型符号表（用户类型感知 codegen）**：
  area(struct Shape s)；String→char*；变体构造辅助函数生成
- [x] **B5-M4：符号表 + let 类型推断**：
  Circle(2.0) → Shape_Circle(2.0)；let s → struct Shape s
- [x] **B5-M5：转译器自转译（转译自身函数）**：
  字符串迭代/字符比较/let 推断/strcat 标记；用例 6 自转译 first_ident
- [x] **B5-M6：完整闭环验证**：
  Aine→C→zig cc 编译→运行输出 55（与解释器一致）；修复 match 块值语义
- [x] **B5-M7：字符串拼接（al_strcat）+ 函数返回类型表**：
  hello Aine 闭环验证；print 格式 fn 感知（%d/%s）
- [x] **B5-M8：Vec → C（al_vec）+ 方法调用**：
  al_vec 类型/EVec 初始化/len()→.len；闭环输出 3
- [x] **B5-M9：数组迭代（for x in Vec）**：
  变量类型表贯穿；sum 闭环输出 6；用例 10
- [x] **B5-M10：Vec.push + 函数返回类型推断**：
  al_push/空数组/闭环输出 2；用例 11
- [x] **B5-M11：if 表达式 + 字符串切片**：
  C 三元 + al_substr；闭环输出 1/AB；用例 12
- [x] **B5-M12：转译器重建 + mini lexer 转译**：
  read 缓存事故后重建（ASCII）；struct/Vec<Token>/al_push 结构完整
- [x] **B5-M13：String 深度语义**：
  strlen/strcmp/tail-return/bool→%d；闭环输出 5/1；用例 6-7
- [x] **B5-M14：转译 lexer 核心函数**：
  read_op 闭环输出 ==；if 表达式/切片/strcmp 全支持；用例 8
- [x] **B5-M15：转译完整 tokenize**：
  al_push sizeof/未知类型 strcmp；用例 9；read_op 闭环输出 ==
- [x] **B5-M16：enum+match 完整闭环**：
  变体构造/match-return/tag 逗号修复；12.56 输出；用例 10
- [x] **B5-M17：内置 Option/Result 类型映射（al_opt）**：
  al_opt 类型化载荷联合体 + 分型构造宏 + match 绑定类型化提取；
  SBlock/c_print_stmt 补全；describe/got:hi + Option<i32>/five 闭环；
  用例 11-12；验证：transpiler 12 用例全过 + aine 159+3 全绿
- [x] **B5-M18：transpiler 自转译闭环（fixpoint）**：
  self_host.exe（Aine 生成 C 编译）13 用例全过 + 自生成输出与 aine 逐行全同
  （5606 行 diff=0）。修复链：char 比较代码生成反逻辑、push 字面量取址
  堆损坏、循环变量/EInt-EFloat/EInt-EIdent 同名冲突（扁平表 first-match）、
  SStruct 缺 return、al_fnum %g 丢小数、var_info 的 EField 迭代器与
  EIndex-EField 类型推断、expr_is_string 单索引、char vs String 变量比较
  （String 侧取 [0]）。
- [x] **B5-M19：通用编译验证（端到端自举闭环）**：
  f-string 函数调用插值（按 fn_return 返回类型转 al_num/al_fnum）；
  case14/15 编译真实 Aine 程序（fib.aine / hello.aine：递归、for 区间、
  中文串、if-else、尾返回、f-string 变量+调用插值）→ zig cc → 原生运行
  输出与 aine run 逐行一致（diff 0）；fixpoint 保持 diff 0。
- [x] **B5-M20：字符串方法 split/trim/replace + 字面量后缀解析 + 方法调用类型推断**：
  解释器三层（interp.rs/typeck/transpiler）补齐文本处理；parse_prefix 对
  字面量/括号补 parse_postfix（"a,b,c".split(..) 可解析）；method_ret 返回
  类型表接入 var_info/c_stmt/c_print_stmt；case16 strings.aine 端到端
  （trim/split().len()/Vec 索引/replace）→ 原生运行 diff 0；fixpoint diff 0。
- [x] **B5-M21：数组字面量换行分隔 + Err 载荷绑定 + Vec<struct> 类型推断**：
  parse_prefix `[` 跳过前导/项间换行；pat_bind_lines 对 Err(e) 按 Result 错误
  类型选联合体成员（second_type_arg + err_mem）；EVec 字面量推断 Vec<首元素
  类型>（空数组保持 infer 保 push 回退）；case17 records.aine（Vec<struct>
  数组 + 索引 + 字段访问 + Err 绑定）端到端 diff 0；fixpoint diff 0。
- [x] **B5-M22：方法链 iter/map/sum（闭包内联）**：
  c_expr 用 GNU 语句表达式实现 .map(closure)（构建 Vec）与折叠
  X.iter().map(f).sum()（单循环求和）；闭包参数临时入变量表推断体类型；
  SLet 标识符初始化补用户类型（struct T）；print 的 f64 用 %s+al_fnum
  （4.0 而非 4）；case18 mapsum + case19 todo.aine（方法链+Result+? 传播）
  端到端 diff 0；fixpoint diff 0。剩余缺口：已全部闭环（闭包独立值、
  import/module 均已实现，G2.0-③ 后为 module 形态）。
- [x] **B5-M23：Map 类型 C 后端 + json.aine 端到端编译**：
  al_map（keys/vals）+ c_type 映射 + set/get/contains/keys/values/len/new
  语句表达式代码生成；修复遮蔽参数自引用（临时变量+函数体嵌套块）、
  无载荷变体作值（构造器调用）、EMatch 三元逆序嵌套 + Some 绑定、bool
  插值 al_bool；case20 json.aine（Map<String,Json> + 枚举 Map 载荷）端到端
  编译 diff 0（解析/序列化/往返 OK）；fixpoint diff 0。
  剩余缺口：char→String 实参转换已落地（B5-M24）；闭包独立值与
  import/module 已闭环；捕获闭包调用点补传已闭环（A2）。
- [x] **B5-M24：char→String 实参自动转换（函数形参类型表）**：
  collect_fn_ptypes/fn_param_ty + fptypes 穿表（80+ 调用点）；c_expr ECall
  对 String 形参的 char 实参生成 al_char_to_str（is_digit(src[i]) 直接可用）；
  json.aine 的 is_digit 恢复 String 仍端到端 diff 0；case21 char-arg；
  fixpoint diff 0。剩余缺口：闭包独立值、import/module、Map 嵌套类型。
- [x] **B5-M25：嵌套泛型类型（Map<String, Vec<T>>）+ case13 稳定性定论**：
  Rust 前端 `>>` 闭合拆分（pending_gt）；转译器 second_type_arg 重写
  （嵌套保留内层 <>）、keys/values 类型表特判、set 复合值临时变量、
  f-string 索引插值 C 化；case25 mapnest（set/get/match 绑定/遍历/keys/
  values）端到端 diff 0；fixpoint 9118 行全同（25 用例）；aine 159+3
  测试全绿。case13「间歇性崩溃」经 12 次干净长运行零复现，定论为调试期
  读-写竞争 + 探针二进制伪象（详见 IMPLEMENTATION_LOG B5-M25）。
  剩余缺口：f-string 方法链插值、闭包独立值、import/module。
- [x] **B5-M26：import / module（文件模块 + 扁平注册）+ 上帝文件首次拆分**：
  `mod name;` 文件模块（Rust 加载器递归读入+防环+缺文件诊断，接入 7 个命令；
  Aine 转译器 SMod/SImport 解析 + flatten_stmts 打平 + c_program 薄壳重算
  类型表）；`import`/`use` 为声明标记（引用已有名字或占位）；case26
  modtest（modtest_main + modtest_util）解释器/原生 diff=0；狗粮拆分
  altype.aine（first/second_type_arg、method_ret、payload_member 四工具），
  Rust 加载与 Aine 自转译双路径验证。工程期决策：MVP 后按 §71.2 再演进
  命名空间。剩余缺口：f-string 方法链插值、闭包独立值。
- [x] **B5-M26.1：transpiler.aine 上帝文件拆分完成（六模块）**：
  根文件 4649→382 行（类型 + 26 用例 main）；allex/alparse/alstr/alcollect/
  altype/alee 各司其职；修复跨模块前向引用（resolve hoist/bodies 分离）、
  跨文件 span 碰撞（加载器重构为文本级内联合并源）、read_file 标记误判、
  String.len() 表推断缺口；26 用例 + fixpoint 9270 行全同 + 全量回归通过。
  上帝文件问题终结——后续新增功能一律进模块文件。
- [x] **M6-STD-1：标准库启动（M6 首个交付物破土）**：
  aine/stdlib/{math,strutil,collections}.aine（纯 Aine）；mod 搜索路径
  同目录 → stdlib/（Rust 与 Aine 两侧加载器）；stdtest.aine 端到端 diff=0；
  fixpoint 9461 行全同（自宿主经自身 C flatten 加载标准库）；修复
  C read_file 失败语义不一致（fail-visible）与 String.len 字段发射缺口。
  已知问题：var_info EIdent 初始化推断臂启用会触发 json 发射越界
  （idx=0 len=0），已禁用待查。
- [x] **B5-M27：闭包独立值**：
  `let f = |x: i32| body` 独立闭包（非捕获、参数注解）；解释器原生；
  C 发射经 desugar_closures 提升为顶层函数（调用点零改动）；case28
  端到端 diff=0；fixpoint 9735 行全同。连带：parse_ty `|` 终止符、
  变体构造 push 取址临时变量化、越界错误带 idx/len。
- [x] **B5-M28：特性收尾冲刺（部分）**：嵌套 Map 原生 ✅（payload 宏/
  绑定类型）；闭包捕获解释器 ✅/原生 fail-visible ✅；诊断体系增强
  （越界 idx/len/vec 内容/调用标签/回溯）✅；Vec<String>/f64 字面量 +
  int-sum 类型 ✅。f-string 链插值与值流子集 v0.1 实现完成但被发射层
  两短板阻塞（pat_bind_lines 位置信息、if-expr 分支异型），WIP 存档。
- [x] **B5-M28.1：发射层修复 + 值流子集激活**：
  pat_bind_lines 迭代边界改按绑定路径层数（根因修复）；var_info
  idxs[nl2-1] → 路径末段；vf 模式扁平化 + 构造 push 临时变量化 +
  al_strcat_own/al_strdup_lit 双实现（Aine 语义 + C 内建）；27 用例 +
  fixpoint 10156 行全同（值流子集激活：14 处原地追加）。永久资产：
  解释器 fn 栈跟踪 + 越界 idx/len/内容诊断。
  遗留（有精确线索）：fstr AST 自编译表对齐、EIdent 推断臂 names/types
  失配、闭包捕获原生发射。
- [x] **B5-M28.2：架构级发现**——C 后端 al_vec 浅拷贝+realloc 值语义
  不健全（自宿主执行 fstr AST/值流子集段错误；解释器因 push 重绑定免疫）。
  立项 B5-M29：深拷贝安全基线 + move/末用消除（真正的值流工程），完成后
  解锁两个 WIP（fstr AST、值流子集 v0.1，代码均已存档）。
- [x] **B5-M29：C 后端值语义（move/clone）**：
  六内建（解释器/C 双侧）+ value_semantics 趟（var-from-var：源未再用
  → MOVE+置空，否则 CLONE 深拷贝基线）；值流子集 v0.1 在其上激活
  （自宿主执行安全）；fixpoint 10789 行全同。实现要点：sem 仅处理
  复合类型、切片需显式 Vec 构建、块构造 push 临时变量化。
- [x] **B5-M30：闭包捕获原生（隐藏参数提升）**：捕获分析 + 隐藏前缀
  参数 + 程序级调用点改写（含 f-string 文本改写）；captest 端到端 diff=0。
  🔶 自宿主 case1 出现 "unsupported stmt"（3 次，分支齐全）→ fixpoint
  暂断，现场已插桩，下一轮专项修复（B5-M31）。
- [x] **B5-M31：自宿主 fixpoint 修复**——根因=match 尾构造调用被当语句
  发射(丢返回值,返回未初始化 struct);rewrite_lift_calls 源层规避
  (显式 return+res);fixpoint 11088 行全同恢复。
- [x] **B5-M32：自举线定稿**——match 尾构造:rewrite_lift_calls 采用语句式
  改写(稳定),转译器层完整修复列后续;some_member 补 EMatch/EClosure 载荷;
  fstr AST 登记为已知限制(let 中转等价),B5-M33 候选。
  **自举线 100% 基线达成**:语言特性层全部自举(嵌套类型/模块/stdlib/独立
  与捕获闭包/值语义/值流子集),六模块+三趟全 Aine,fixpoint 11110 行全同。
- [x] **里程碑 A：Aine 工具链自给**——
  1.1 诊断质量（5 类高频错误全带位置+白话建议；did-you-mean；片段单行钳制；
      语义+类型错误一次全部输出）；
  1.2 fmt_tool（幂等 + 可重解析，fib/records/mapnest/strings/mapsum 全过；
      修复 stringify 丢 f-string 前缀）；
  1.4 pkg_tool（aine.toml 极简解析 + myapp 脚手架 + build 产物校验）。
- [x] B5-M33(后续):fstr AST 段错误根因(C 后端分析)、match 尾构造转译器层
  修复、typeck/formatter 组件迁移(M6 分期)——fstr AST/match 尾构造已完成;
  迁移见下"全 Aine 迁移"。
- [ ] **全 Aine 迁移(向全 Aine 转移)**:
  - [x] Phase 1:fmt 命令改用 Aine 实现(examples/fmt_tool.aine,10 文件 diff=0)
  - [ ] Phase 2:typeck 诊断分期迁移 Aine(前置:Index 节点携带 span)
  - [ ] Phase 3:valueal(所有权/值流)迁移 Aine
  - [ ] Phase 4:解释器自举(Aine 解释器解释 Aine)
  - [ ] Phase 5:工具(LSP/Profiler/Debugger)接入 Aine 流水线
- [ ] B5-M30：闭包捕获原生发射（env 结构体）
- [x] **M6-STD-2：stdlib 扩容完成（MVP 8/11 模块）**——
  io（write/append/exists，C 内建 + 解释器双侧对称）、json（B2 升级入库）、
  collections（sort_ints/sum_ints/contains_int）、time（now_secs/elapsed/fmt_duration）、
  path（join/dir/base/ext）。全部端到端验证（iotest/coltest2/timetest diff=0）。
  连带修复：var_info EBin 比较运算→bool、f-string bool 插值（al_bool）、
  内建函数重名跳过清单机制。SQLite/HTTP 按原计划评估（MVP 可后置）。
- [ ] M6-STD-3：SQLite/HTTP（MVP 后置候选；需 C 内建扩展）
- [ ] M6-FMT-1：fmt_tool.aine——Aine 原生格式化工具（幂等 + 可重解析）
- [ ] M6-FMT-2：fmt_tool 输出对齐 aine fmt（diff=0）
- [ ] M6-TCK：typeck 诊断分期迁移 Aine（前置：Index 节点携带 span）
- [x] M6-TEST-1：测试框架（M23）—— `@test` 属性 + `aine test` 命令
  （源码顺序发现、每测独立解释器+大栈线程隔离、Err/运行时错误/崩溃三类失败判定、
  汇总报告+退出码）；验收集 examples/testdemo.aine（5/5 过）与
  examples/testfail.aine（失败演示）；回归测试 3 项入 cargo test（162 全绿）
- [x] M2：Conformance 子集 —— 正向行为集 conformance/（类型 10/值语义 6/
  闭包 6/错误处理 5/模块 4 = 31 用例，aine test 运行全过）；
  负向诊断集 + VF/Summary 契约 11 项入 cargo test（错误码稳定性与
  did-you-mean 断言）；连带修复一元负号优先级缺陷（缺陷清单 #8），
  自举 fixpoint 复验成立（27 用例，仅宿主运行器尾行 Ok(()) 口径差）
- [x] M4：语言面补全（用户导向缺口全修复）—— assert 内建、
  索引赋值 v[i]=x 与 m[k]=v（双路径 + 值语义）、统一函数类型
  fn(A,B)->R（D14 捕获即快照，C 函数指针发射）、f-string 嵌套引号
  （双 lexer）、字符索引模型（D15，解释器权威实现 + C 侧备妥待 M6-TCK
  切换）；Conformance C6 九用例；自宿主 fixpoint 成立（29 用例，
  新增 case29/case30）；设计决策 D14/D15 入登记簿
- [ ] 其余 M6 交付物：UI Framework/Runtime/IDE-LSP/语言面纱/文档/发布
- [x] B5：Aine → C 转译器（自举闭环起点）— 前端组件（lexer/json/parser/transpiler）
  已全部由 Aine 实现并自举替换 Rust 对应组件

## 7. B1 暴露并修复的缺陷（写 Aine 的价值证明）

1. **typeck 禁止 String 比较**（`c >= "0"`）→ 允许 Str-Str 字典序比较
2. **parser：`a || b` 中 `||` 被误解析为尾部空闭包**（变成第 2 个参数）→ 尾部闭包仅限单 `|`
3. **typeck：嵌套块尾部被错误地按函数返回类型检查** → 仅函数体块执行 tail 检查
4. **解释器：Vec 方法不可变语义**（push 返回新 Vec 被丢弃）→ 变量接收者自动重绑定（Aine 可变用法）
5. **语法增强：切片语法 `s[a..b]` / `v[a..b]`**（自举刚需，记入 Grammar G1.0）
6. **Rust 前端：嵌套泛型 `>>` 被词法为移位符**（`Map<String, Vec<i32>>`）→
   解析器拆分闭合括号（M25）
7. **转译器：second_type_arg 丢弃嵌套 `<>`**（`Option<Vec<i32>>` 推断成
   `Option<Veci32>`）→ 深度计数重写（M25，与正确的 first_type_arg 对齐）
8. **Rust 前端：一元负号优先级低于加减**（`-5 + 3` 实为 `-(5+3) = -8`，
   违反语法 §2.5 UnaryExpr 层级）→ parse_prefix 一元操作数 min_bp 13→14
   （M2 Conformance 首战暴露；修复后自举 fixpoint 复验成立）