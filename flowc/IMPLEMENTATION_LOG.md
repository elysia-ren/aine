# aine 实现日志

> Aine 语言编译器工程实现记录。
> 遵循《Aine 1.0 执行计划与完成定义》里程碑 M1-M3 + 自举路线图 B0-B5。

## 方法论纪律（重要）

> **语法规范先行，实现验证在后。**
> 正式语法根基 = 《Flow_Language_Grammar.md》（G1.0），类型规则 = 《Flow_Type_System.md》（T1.0）。
> 任何变更必须先改规范、再改实现、再补测试。

## M1/M2/M3 + 最小可执行闭环 ✅
## 自举 B1（lexer.aine）✅ / B3 一/二阶段（Map+enum+impl+mod）✅

## 自举 B2：json.aine（Aine 写 JSON 解析器）— 完成 ✅（本轮，155 测试）

- **examples/json.aine**（~180 行 Aine）：JSON 递归下降解析器 + 序列化器 +
  往返验证。覆盖对象/数组/字符串（\" \\ \n 转义）/数字（整数/浮点/指数）/
  true/false/null；用 Json enum（JNull/JBool/JNum/JStr/JArr/JObj）+ Map + Result? 表达。
- **写 Aine 过程暴露并修复 8 个实现缺陷**：
  1. **? 传播语义错误**：嵌套函数的 Err 冒泡到顶层而非转为自身 Result 返回
     → call_fn 捕获 Propagate 返回 Result(Err)；main 返回 Err → 程序失败（退出码 1）
  2. **match 臂不支持语句**：`=> return x` 解析失败 → 语句臂包装为块
  3. **Some/None/Ok/Err 模式不匹配 Option/Result 值** → 模式匹配补充
  4. **typeck 拒绝 String + String**（字符串拼接）→ 允许并返回 Str
  5. **match 发散臂（return 块）污染结果类型**（T3004 误报）→ 发散臂跳过合并
  6. **单元变体作为值**（JNull 表达式）→ eval Ident 支持零参构造
  7. **单元变体模式**（裸 JNull 被当变量绑定，永远匹配）→ 模式匹配消歧（最关键）
  8. **None 裸值** → eval Ident 支持
- **测试**：152 单元 + 3 集成 = 155 全绿（新增 json 往返、? 传播、match 语句臂、
  单元变体、字符串拼接、Some/None 模式 6 项）
- **教训**：测试模块再次被截断 → 确立纪律：**每次重建后用测试计数校验完整性**


## 自举 B4-M1：parser_expr.aine（Aine 写表达式 Pratt 解析器）— 完成 ✅（本轮）

- **examples/parser_expr.aine**（~120 行 Aine）：Aine 表达式级语法分析器，
  含 mini 词法器 + Pratt 优先级解析（bp_of 3-8）+ enum Expr AST
  （EInt/EFloat/EStr/EIdent/EBin/EUnary/ECall/EField）+ stringify 回显。
- **测试用例全过**：优先级（1+2*3 → (1+(2*3))）、括号（(1+2)*3 → ((1+2)*3)）、
  后置调用/比较/逻辑/一元（a.b(2)-5<10&&!ok）、浮点（3.14+x）。
- **写 Aine 过程暴露并修复 2 个实现缺陷**：
  1. **parse_expr_bp 误消费右括号/逗号**（lbp==0 时 break 条件写反）→
     修正为 lbp==0 或 lbp<min_bp 时 break
  2. **逻辑或 || 被误解析为尾随空闭包**（is_ident_start(c) || ... 变双参调用）→
     尾随闭包只匹配单个 Pipe
- **配套：mut 形参**。parser.rs 吃 KwMut，Param 携带 is_mut（ast/hir/resolve 贯穿），
  运行期允许重赋值（fn sum_to(mut n: i32)）。
- **测试**：154 单元 + 3 集成全绿（新增 parser_expr_flow_runs、mut_params_can_be_reassigned）。

## 测试模块截断事故（第三次）与恢复

- 追加 B4 测试时 findIndex+slice 再次截断 interp.rs 测试模块（只剩 3 条）。
- 恢复：核对核心（前 1273 行）完整且括号平衡（434/434）→ 单次全量写入
  （核心 + 37 条测试）→ 重建后立即用测试计数校验（37 ✅）。
- 纪律强化：**测试只能整文件重写，绝不 findIndex+slice 追加；写后必验计数**。


## 自举 B4-M2：parser_stmt.aine（Aine 写语句级解析器）— 完成 ✅（本轮，159 测试）

- **examples/parser_stmt.aine**（~500 行 Aine）：在 B4-M1 表达式核心之上扩展：
  语句（let/mut/类型注解/if-else/while/for..in/return/赋值 op=/块/表达式语句）、
  项（fn 定义含参数与返回类型）、字符串与数组字面量、换行→TSemi 语句边界。
  完整 AST 的 stringify 回显 + main 内 4 组自断言（let 优先级、if-else 缩进、
  fn+while+for+赋值、字符串拼接返回值）。
- **写 Aine 过程暴露并修复解释器重大缺陷**：
  1. **表达式块内的 return 不传播**（最关键）：eval 的 Block/If/Match 表达式把
     ControlFlow::Return 吞成普通块值 → if 块内的 match 臂 return 失效、
     表达式块内 return 也不生效。修复：RtError 新增 Return(Value) 变体，
     eval 表达式块遇到 Return 向上抛，由 Stmt::Expr/exec_block 尾部/call_fn/
     call_closure 捕获转为正常返回。
  2. **Aine 无元组 .0/.1 与 is 关键字**：parse_ty 返回元组 (String,i32) →
     改 TyResult 结构体；ty is Some → match 模式。
  3. **let mut 顺序**：parse_let 需自处理 mut 前缀（原把 mut 当变量名）。
  4. **+= 等复合赋值未纳入两字符运算符** → tokenize 补充；新增 SAssign 语句。
- **测试**：156 单元 + 3 集成全绿。新增回归：
  return_in_if_block_match_propagates（if 内 match 臂 return）、
  return_in_expression_block_propagates（表达式块内 return）。
- **教训（第三次截断根因修正）**：read 工具默认 2000 行 limit，interp.rs 超过
  2000 行后任何 read→write 全量回写都会截断文件！纪律升级：**超长文件禁止
  read 后整写；只用 write 一次性写全量内容或 edit 定点修改**。


## 文件删除纪律（用户反馈修正）

- **绝不用 Remove-Item -Force 删除文件**（不进回收站，误删无法恢复——曾误删
  parser_stmt.aine，靠完整重建才恢复）。
- **删除一律进回收站**（PowerShell，无弹框）：
  Add-Type -AssemblyName Microsoft.VisualBasic;
  [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($path, 'OnlyErrorDialogs', 'SendToRecycleBin')
- Shell.Application 的 InvokeVerb('Restore'/'Delete') 会弹确认框且同名冲突时不可靠，
  自动化脚本不用；需要还原时在资源管理器回收站里手动操作。
- 临时调试文件（dbg_*.aine）同理进回收站，不彻底删除。


## 自举 B4-M3：parser_stmt.aine 全量 items 解析 — 完成 ✅（本轮，162 测试）

- **examples/parser_stmt.aine** 扩展为全量解析器：新增 struct/enum/impl/mod 项
  解析 + match 语句（模式收集 + 臂体块/表达式）+ 空元组 () + 泛型类型
  （Map<String, i32> 的逗号不再截断类型）+ 复合赋值 += 等。
  AST 可编程遍历验证（统计项/字段/变体/方法数量）。
  7 组自断言全过，包括解析完整真实风格程序（fn+enum+match+Map+Result）。

## 重大缺陷修复：解释器栈溢出（写 parser 暴露）

- **现象**：parse_program(tokenize(src))? 在用户函数内调用时栈溢出崩溃
  （deep(20) 就溢出），而分开调用正常。
- **根因**（三层）：
  1. **每层调用克隆整个 FnDef AST**（fns.get(name).cloned()），大对象栈分配；
  2. 树遍历解释器每层递归创建多个大局部变量（Env/defers）；
  3. 默认线程栈 8MB 不足以支撑深层递归。
- **修复**：
  1. **fns/impls 改存 Rc<ast::FnDef>**——Rc 克隆廉价且解借用（4 处调用点）；
  2. **解释器跑在 512MB 栈线程上**（main.rs + 测试 run_src 均如此）；
  3. 保留 eval/call/exec 三级递归深度 guard（防死循环误伤）。
- **验证**：deep(500) 正确输出 500；互递归 a↔b 阈值 100 正常；
  question_mark_in_deep_call_chain 通过。新增 2 个回归测试。
- **写 Aine 过程继续暴露语法缺口并补齐**：
  - Aine 无元组解构 .0/.1 与 is 关键字 → 用 struct + match
  - match 语句缺失 → SMatch + MatchArm
  - 空元组 () 无法解析 → EUnit
  - 泛型类型里的逗号截断 → parse_ty 深度感知


## 自举 B4-M4：parser_stmt.aine 解析真实 Aine 程序（自举验证）— 完成 ✅（本轮）

- **目标**：Aine 写的解析器解析 Aine 写的编译器组件源码（自举关键验证）。
- **补齐的语法能力**（parser_stmt.aine，Aine 侧）：
  1. **注释支持**：// 行注释、/* */ 块注释（tokenize 跳过）
  2. **结构体字面量**：Point { x: 1 y: 2 }（EStruct，仅当 { 后是 ident : 才触发，
     避免与 match c { 冲突）
  3. **f-string**：f"..." 整体作为字符串 token（不再拆成 f + 字符串）
  4. **? 后缀运算符**：ETry(expr)
  5. **for-in 迭代集合**：for o in ops（SFor 改为存迭代表达式，区间 a..b 用 ERange）
- **新增正式用例 8/9**：
  - 用例 8：解析 Aine 类型声明（enum 载荷 + 泛型 Vec<Expr> + 结构体字面量 + 变体构造）
  - 用例 9：解析 lexer.aine 真实代码（struct + 函数 + 字符串迭代 + 运算符判断）
  → **Aine 解析器成功解析 Aine 编译器组件源码**，自举闭环验证通过
- **修复 aine Rust parser bug**：一元运算符（! - & *）操作数用 parse_expr(13)
  允许 trailing 消费，导致 if !x { 的 { 被错误消费（if -x { 同样出错）。
  修复：4 处一元分支改用 parse_expr_mode(13, false)。
  parser_stmt.aine 之前 ast 阶段失败（322 行 match base）即因此 bug。
- **测试**：parser_stmt.aine 9 组用例全过；aine 159 单测 + 3 集成全绿；
  parser_stmt.aine 可被 aine ast 正常解析。


## 语法评估 + 上帝文件拆分（本轮）

- **语法评估**（用户要求：整体看语法可优化处，不快速向下）：
  整理《Flow_语法优化清单.md》：23 个实战问题（15 实现缺口 + 8 语法增补）已解决并回写；
  评估 8 个新提议——char 类型/元组/类型推断/单字符索引均已有或不做（保持简洁），
  泛型/trait/标准库/并发属于 M4/M5 范畴后置。**结论：语法无需大改，G1.0 经受住实战**。
- **上帝文件拆分**（用户要求，解决维护难/编译慢）：
  1. parser.rs 1675→1497 行：测试模块独立为 parser_tests.rs（22 测试）；
  2. interp.rs 1566→1285 行：测试模块独立为 interp_tests.rs（42 测试）；
  3. 增量构建 0.05s（测试模块拆分后编译单元变小）；
  4. 方法级拆分（call_method 等）评估后**不做**：跨 impl 可见性改动风险高、收益有限，
     且自举 B5 将用 Aine 重写这些组件，拆分是重复劳动。
- **测试**：159 单测 + 3 集成全绿（拆分零回归）。


## 拆分后回归验证（设计导向确认，本轮）

- **测试**：159 单测 + 3 集成全绿（拆分零回归）。
- **示例程序**：8 个正常程序 exit=0（hello/fib/lexer/json/parser_expr/
  parser_stmt/account_book/reduced）；todo.aine exit=1 是设计预期
  （? 错误传播演示）；bad/type_errors 预期失败。
- **CLI 全阶段**：lex/ast/hir/typeck/fmt/vf/opt/check 全部 exit=0；
  bench 输出 M3 大对象报告（0.99ms/1200 函数）。
- **设计导向验收**：
  - M1 核心语言全链路正常；M2 VIR/Summary/Ownership 正常；M3 bench 正常
  - §71.1 跨函数 Summary 组合（add → Owned）
  - 零拷贝优先：opt 报告 5 Copy / 0 Materialize（符合设计目标）
  - 诊断体系：T3001/T3010/T3003 结构化类型错误输出
  - fmt 幂等（两次输出哈希一致）
- **结论**：拆分是纯重构，设计导向的全部验收点保持满足。


## 自举 B4-M5：parser_stmt.aine 终极自解析 — 完成 ✅（本轮）

- **目标**：Aine 写的解析器解析 Aine 解析器自身的代码（自举决定性证据）。
- **补齐的语法能力**（自解析暴露的真实缺口）：
  1. **闭包解析**：|x, y| expr（EClosure）——parser 自身大量使用
  2. **mut 形参**：parse_fn 支持 mut 前缀（之前把 mut 当参数名）
  3. **索引表达式**：base[i] 与切片 base[a..b]（EIndex）——parser 自身
     最常用语法（tokens[i]、src[a..b]），之前完全缺失
- **新增正式用例 10/11**：
  - 用例 10：解析 parser_stmt.aine 自身的类型定义（递归 enum Expr↔Stmt、
    泛型 Vec<StructField>、Option<String>）
  - 用例 11：解析 parser_stmt.aine 自身的 parse_let 函数（mut 形参、索引、
    字段访问、Option/Some、? 传播、f-string、结构体字面量、复合赋值）
  → **Aine 解析器成功解析 Aine 解析器自己的代码，自举闭环达成**
- **测试**：parser_stmt.aine 11 组用例全过；aine 159 单测 + 3 集成全绿。


## 自举 B5-M1：transpiler.aine（Aine 写 Aine→C 转译器）— 启动 ✅（本轮）

- **examples/transpiler.aine**（~1300 行 Aine）：复用 parser_stmt.aine 的
  完整解析核心（tokenize/parse_program/AST），新增 C codegen：
  - c_expr：算术/比较/逻辑/一元/调用/字段 → C 表达式
  - c_stmt：let/if/while/return/赋值/函数定义 → C 语句
  - print(x) → printf；类型映射 i32→int、f64→double、String/Vec→void*
- **4 组用例全过**：
  1. 函数+算术+调用：int add(int a, int b) { int c = (a + b); return c; }
  2. if/while/复合赋值：while ((n > 0)) { total += n; n -= 1; }
  3. 递归：return (fib((n - 1)) + fib((n - 2)));（递归转译正确）
  4. 自转译：转译自身 c_type 函数（void* c_type(void* ty)）
- **B5 真实边界**（诚实记录）：当前支持纯函数子集（fn/let/if/while/算术/
  递归调用）；enum/match/Option/Vec/字符串操作 → C 映射未覆盖（unsupported）。
  Option<String> 映射为 void* 是占位。后续 M2 目标：覆盖 struct/enum/match → C。
- **验证**：aine 159 单测 + 3 集成全绿；transpiler.aine 4 用例全过；
  调试文件进回收站。


## 自举 B5-M2：struct/enum/match → C 映射 — 完成 ✅（本轮）

- **transpiler.aine 扩展**（Aine 写 codegen）：
  1. struct → C struct（字段类型映射）
  2. enum → C tagged union（enum <T>_tag + struct { tag; union { ... } }）
  3. match → C switch（case <变体>；default 为 _）
  4. **模式绑定 → C 字段访问**：Circle(r) → auto r = s.Circle._0;
     Rect(w, h) → auto w = s.Rect._0; auto h = s.Rect._1;
  5. EStruct 字面量 → ((struct T){ .f = ... })；变体构造 → var_<Name>(...)
- **用例 5 全过**：enum Shape { Circle(f64) Rect(f64,f64) } + struct Point +
  match area() 转译成功，模式绑定断言（r/w/h 的 union 字段路径）验证通过。
- **B5-M2 剩余边界**（诚实记录，B5-M3 目标）：
  - 类型映射需感知用户类型：area(void* s) 应为 area(struct Shape s)
  - 变体构造需枚举类型名（Circle 属于 Shape）→ 需要符号表/类型环境
  - 字符串/Vec 操作 → C 未覆盖
- **验证**：transpiler.aine 5 用例全过；aine 159+3 全绿。


## 自举 B5-M3：类型符号表（用户类型感知 codegen）— 完成 ✅（本轮）

- **transpiler.aine 扩展**：引入 user_types 参数（collect_user_types 扫描
  struct/enum 定义），贯穿 c_program → c_stmt → c_expr → c_type。
- **B5-M2 边界修复**：
  1. ✅ 用户类型感知：area(void* s) → area(struct Shape s)
  2. ✅ String 映射 void* → char*（更准确）
  3. ✅ 变体构造辅助函数：enum 定义处生成
     Shape_Circle(double _0) / Shape_Rect(double _0, double _1)
     （返回 { .tag = Circle, .Circle = { _0 } }）
- **实现过程中踩坑**（诚实记录）：
  - @global Map/Vec 的运行时 set/push 不生效（解释器全局可变性 bug），
    但初始值+读取正常 → 改用参数传递而非全局状态
  - 大量 c_expr/c_stmt 调用点需要逐一加 user_types 参数（机械改动易错）
- **B5-M4 剩余边界**：
  - 变体→枚举归属映射（Circle 属于 Shape）→ 需符号表；当前 Circle(2.0)
    生成无前缀调用，与 Shape_Circle 构造函数不一致
  - let 类型推断（let s = Circle(2.0) 生成 int s 而非 struct Shape s）
- **验证**：transpiler.aine 5 用例全过；aine 159+3 全绿。


## 自举 B5-M4：符号表（变体归属）+ let 类型推断 — 完成 ✅（本轮）

- **transpiler.aine 扩展**：collect_variant_names/collect_variant_owners
  收集变体→枚举归属表（平行 Vec），variant_owner() 查找；
  贯穿 c_program → c_stmt → c_expr（+2 参数）。
- **B5-M3 边界全部解决**：
  1. ✅ 变体构造归属：Circle(2.0) → Shape_Circle(2.0)
  2. ✅ let 类型推断：let s = Circle(2.0) → struct Shape s = Shape_Circle(2.0)
- **生成的 C 代码现在是完整一致的 tagged union 实现**：
  enum → tag+union；变体构造 → Shape_Circle(double) 返回正确初始化；
  match → switch + 模式绑定 auto r = s.Circle._0；let 推断 struct Shape s。
- **踩坑教训**：pwsh 行号手术多次漂移破坏文件结构，最终用 edit 工具
  （内容匹配）修复；维护纪律：**行号操作不可靠，优先内容匹配 edit**。
- **验证**：transpiler.aine 5 用例全过（含增强断言）；aine 159+3 全绿。


## 自举 B5-M5：转译器自转译 — 完成 ✅（本轮）

- **目标**：transpiler.aine 转译自身核心函数（自举闭环第一步）。
- **新增转译能力**（写自转译暴露的真实缺口）：
  1. **for-in 字符串迭代 → C for 循环**：for c in pat →
     for (size_t _i = 0; _i < strlen(pat); _i++) { char c = pat[_i]; ... }
  2. **单字符字符串 → C 字符字面量**：c >= "a" → c >= 'a'
  3. **let 类型推断扩展**：空字符串 → char*、整数 → int、浮点 → double
  4. **字符串拼接标记**：+ 两侧含 EStr 字面量 → /* strcat */ 注释
    （C 动态内存拼接是 B5-M6 目标）
- **用例 6 全过**：转译器转译自身 first_ident 函数（字符串迭代、字符比较、
  let 推断、if/else/break 全部转译正确）。
  → **Aine 转译器成功转译 Aine 转译器自己的代码**
- **踩坑教训**：pwsh 引号替换两次破坏文件（''' 非法字符）→ 修复后确认
  纪律：**代码注入用 edit 内容匹配，pwsh 仅做只读操作**。
- **验证**：transpiler.aine 6 用例全过；aine 159+3 全绿。


## 自举 B5-M6：完整闭环验证（Aine→C→原生编译→运行）— 完成 ✅（本轮）

- **闭环实证**（自举路线决定性验证）：
  Aine 源码 (fib) → transpiler.aine（Aine 写转译器）→ C 代码 →
  zig cc（clang 22，本机发现）→ 原生可执行文件 → 运行输出 55
  （与 aine run 结果完全一致）。
  → **Aine 写的转译器 + C 编译器 = 完整自举后端路径实证**
- **转译器修复**（闭环暴露的真实 bug）：
  1. **match 臂块尾部不返回值**：ct = match ret { None => { if... } }
     返回 Unit（()）→ 改为显式赋值 ct = "int"（Aine match 语义细节）
  2. main 处理：int main() + return 0 放函数末尾；print 格式 %d/%g/%s
     按字面量推断（非字面量参数用 %d）
- **用例 7 全过**（fib→C 完整断言：int fib/int main/printf %d/return 0）。
- **编码事故**（诚实记录）：pwsh Set-Content 多次操作导致 transpiler.aine
  中文注释/字符串编码损坏（显示乱码），但 ASCII 逻辑完好、功能正常；
  后续新用例用 ASCII 编写。待修复：整体重写文件恢复中文。
- **验证**：transpiler.aine 7 用例全过；aine 159+3 全绿；
  zig cc 编译运行输出 55。


## 自举 B5-M7：字符串拼接（fl_strcat）+ 函数返回类型表 — 完成 ✅（本轮）

- **transpiler.aine 新增**：
  1. **fl_strcat 辅助函数**：c_program 头部生成动态分配字符串拼接
     （malloc + memcpy），EBin + 字符串 → fl_strcat(a, b)
  2. **函数返回类型表**：collect_fn_names/collect_fn_returns 收集
     fn name -> 返回类型；fn_return() 查表
  3. **print 格式 fn 感知**：print 参数是 ECall 时查函数返回类型
     （fib → %d；greet → %s）
- **闭环验证**：greet 字符串拼接程序 → C → zig cc 编译 → 运行输出
  hello Aine（与解释器一致）。
- **踩坑（重复教训）**：
  - pwsh 行号/正则手术 3 次破坏 transpiler.aine（误替换 fl_strcat 定义行、
    fn 表代码插进 variant_owner 函数体、for fn 用关键字 fn 作变量）
  - 修复纪律再次确认：**内容匹配 edit 优先，行号操作高风险**
- **验证**：transpiler.aine 8 用例全过；aine 159+3 全绿；
  zig cc 编译运行 hello Aine。


## 自举 B5-M8：Vec → C（fl_vec）+ 方法调用转译 — 完成 ✅（本轮）

- **transpiler.aine 新增**：
  1. **fl_vec 类型**：typedef struct { size_t len; int* data; } fl_vec
  2. **EVec 字面量 → fl_vec 初始化**：((fl_vec){ 3, (int[]){ 1, 2, 3 } })
  3. **let 推断 EVec → fl_vec**；Vec<...> 参数映射 → fl_vec
  4. **方法调用**：nums.len() → nums.len（字段访问）+ %zu 格式
  5. **变量类型表**：collect_var_names/collect_var_types 收集（B5-M9 用）
- **闭环验证**：Vec 程序 → C → zig cc 编译 → 运行输出 3
  （[1,2,3].len() = 3，与解释器一致）。
- **关键发现**（Aine 语义细节）：match 臂块尾部表达式不返回值
  （f1 = match a {...} 得 Unit）→ 全部改显式赋值；嵌套 match 内 return
  会提前返回整个函数 → 严禁在 match 臂内 return 表达式值。
- **B5-M9 剩余**：变量类型表贯穿 SFor（数组迭代 for x in v）；
  Vec.push/索引 v[i] 转译。
- **验证**：transpiler.aine 9 用例全过；aine 159+3 全绿。


## 自举 B5-M9：数组迭代（for x in Vec）— 完成 ✅（本轮）

- **transpiler.aine 新增**：变量类型表（collect_var_names/collect_var_types）
  贯穿 codegen；SFor 对 Vec 类型变量 → fl_vec 数组迭代：
  for (size_t _i = 0; _i < v.len; _i++) { int x = v.data[_i]; }
- **闭环验证**：sum(v: Vec<i32>) 程序 → C → zig cc 编译 → 运行输出 6
  （[1,2,3] 求和，与解释器一致）。
- **用例 10 全过**（数组迭代断言：v.len / v.data[_i]）。
- **方案对照**（用户要求时常对照方案检查）：
  - 编译器界面多语言 = Language Veil（§52）：Canonical Source 唯一真相 +
    IDE 投影，本地化是显示层非第二份源码（Phase 7，架构上已留接口）
  - 编译渲染 = UI Framework ui{}（§36-41）：aine 已解析 UiDef/UiProp，
    但解释器/转译器不执行 UiOp（编译渲染缺口，M6 范畴）
  - 简洁/高效/用户导向 = §73 认知负担 + §75 编译时间原则
  - **当前状态**：R1（编译器分析器）已基本攻克（自举闭环实证），
    M6 MVP 的其他 8 项（标准库/UI/Runtime/IDE/Veil/测试/文档/Build）未启动
- **验证**：transpiler.aine 10 用例全过；aine 159+3 全绿。


## 自举 B5-M10：Vec.push + 函数返回类型推断 — 完成 ✅（本轮）

- **transpiler.aine 新增**：
  1. **fl_push 辅助函数**（realloc 动态增长）：v.push(x) → fl_push(&v, x)
  2. **空数组初始化**：let mut v = [] → ((fl_vec){ 0, NULL })
  3. **SLet 函数返回类型推断**：let r = build() → fl_vec r（查 freturns，
     支持 Vec<...>/String/i32/f64）
- **闭环验证**：build+push 程序 → C → zig cc 编译 → 运行输出 2
  （两次 push 后 len=2，与解释器一致）。
- **用例 11 全过**（fl_push(&v, 1) / fl_vec r = build() 断言）。
- **B5-M11 缺口**（转译 lexer.aine 级程序暴露）：
  1. **if 表达式**：let two = if cond { ... } else { ... }（当前只支持
     if 语句，不支持 if 表达式赋值）
  2. **字符串切片**：src[i..i+2]（当前 unsupported）
- **验证**：transpiler.aine 11 用例全过；aine 159+3 全绿。


## 自举 B5-M11：if 表达式 + 字符串切片 — 完成 ✅（本轮）

- **transpiler.aine 新增**：
  1. **if 表达式**：AST 加 EIf；parse_if_expr 解析（let two = if cond {...}
     else {...}）；codegen → C 三元 (cond ? a : b)（block_tail_expr 取块尾值）
  2. **字符串切片**：EIndex + ERange → fl_substr(src, a, b) 辅助函数
    （动态分配子串，越界裁剪）
- **闭环验证**：
  - if 表达式：f(5) → C 三元 → 运行输出 1
  - 切片：read_op("ABC", 0) = src[0..2] → 运行输出 AB（与解释器一致）
- **用例 12 全过**（(x > 0) ? 1 : 2 / fl_substr(s, i, (i + 2)) 断言）。
- **验证**：transpiler.aine 12 用例全过；aine 159+3 全绿。


## 自举 B5-M12：转译器重建 + mini lexer 转译 — 部分完成 ✅（本轮）

- **事故与恢复**：read 工具缓存错误导致 transpiler.aine 损坏（1492 行不可用），
  无备份。恢复：从 parser_stmt.aine 提取核心（pwsh，1146 行）+ 重建 codegen
  （ASCII 注释版，避免中文乱码）→ transpiler.aine 恢复（1775 行，5 用例）。
- **转译 mini lexer 成功**：transpiler 转译精简版 lexer（struct Token +
  Vec<Token> + fl_push 结构体 + 切片 + while）→ C 结构完整：
  struct Token / fl_vec tokenize / fl_push(&tokens, ((struct Token){...}))
- **重建后恢复的能力**：fib/strcat/vec/if 表达式(三元)/切片(fl_substr)/数组迭代
  ——5 用例全过；if 表达式闭环编译运行输出 1。
- **B5-M13 缺口**（转译 String 深度语义）：
  - String.len() → strlen（当前 src.len 错误）
  - String 比较 → strcmp（C 字符串比较）
  - 切片返回类型 char*（let c = fl_substr 推断）
- **纪律强化**：read 工具缓存不可靠 → **文件修改后立即用 pwsh 验证**；
  **transpiler.aine 重建用 ASCII 注释**（中文乱码教训）。
- **验证**：transpiler.aine 5 用例全过；aine 159+3 全绿。


## 自举 B5-M13：String 深度语义（strlen/strcmp）— 完成 ✅（本轮）

- **transpiler.aine 新增**（类型感知的 String 语义）：
  1. **String.len() → strlen(s)**（base_type 查 var 类型，Vec → .len）
  2. **String 比较 → strcmp(a, b) op 0**（expr_is_string 检测 String 操作数）
  3. **函数体尾表达式 → return**（SDef 尾部 SExpr 生成 return）
  4. **bool 返回 → %d**（print 格式 fn 感知）
- **闭环验证**：
  - String.len：f("hello") → strlen → 运行输出 5
  - String 比较：is_digit("5") → strcmp → 运行输出 1（与解释器一致）
- **用例 6-7 全过**（strlen(s) / strcmp(c, "0") >= 0 断言）。
- **验证**：transpiler.aine 7 用例全过；aine 159+3 全绿。


## 自举 B5-M14：转译 lexer 核心（if 表达式+切片+strcmp）— 完成 ✅（本轮）

- **transpiler.aine 修复**（转译 lexer 核心暴露）：
  1. **EIf → char* 推断**：let two = if ... { src[i..i+2] } else {...}
     的 then 块尾是切片 → 推断 char*
  2. **expr_is_string 对 infer 类型保守视为 String**：let 推断变量参与
     比较时用 strcmp（否则 C 指针比较错误）
  3. **EIndex → char* 推断**：切片赋值推断 char*
- **闭环验证**：transpiler 转译 lexer 的 read_op（多字符运算符识别，含
  if 表达式 + 切片 + String.len + strcmp）→ C 编译无警告 → 运行输出 ==
  （与解释器一致）。
- **用例 8 全过**（strcmp(two, "==") == 0 / fl_substr(src, i, (i + 2)) 断言）。
  → **Aine 转译器转译 Aine 写的词法分析器核心函数，编译运行正确**
- **验证**：transpiler.aine 8 用例全过；aine 159+3 全绿。


## 自举 B5-M15：转译完整 tokenize — 完成 ✅（本轮）

- **transpiler.aine 修复**（转译 tokenize 暴露）：
  1. **fl_push 加 elem_size 参数**：首次 push 时设置元素大小
     （结构体字面量 → sizeof(struct T)；其他 → sizeof(int)）
  2. **expr_is_string 未知类型回退**：循环内 let 变量（不在 var 表）
     参与比较且对方是 EStr → 用 strcmp
  3. **空 fl_vec 初始化**：{ 0, 0, NULL }（3 字段）
- **转译完整 tokenize**（struct + while + 切片 + strcmp + fl_push sizeof）
  → C 结构完整：fl_vec tokenize(char* src) + strcmp(c, " ") == 0 +
  sizeof(struct Token)。用例 9 全过。
- **完整 lexer.aine 嵌入**（143 行）的转义工程（PowerShell 数组陷阱 +
  read 缓存）多次失败——记录为工程噪音，核心验证由 read_op/tokenize
  闭环达成（输出 == 与解释器一致）。
- **验证**：transpiler.aine 9 用例全过；aine 159+3 全绿。


## 自举 B5-M16：enum+match 完整闭环 — 完成 ✅（本轮）

- **transpiler.aine 修复**（编译 enum 程序暴露 4 个 bug）：
  1. **enum tag 逗号**：C enum 需逗号分隔（Circle, Rect）
  2. **match 臂 return**：SExpr 臂 → return（函数有返回值时）
  3. **变体构造恢复**：Shape_Circle(...) 构造函数（重建时丢失）+ SLet
     变体推断（let s = Circle(2.0) → struct Shape s）
  4. **SEnum return out 位置**：构造函数后返回（之前提前 return 成死代码）
- **闭环验证**：enum Shape + match area → C → zig cc -std=c23 编译
  → 运行输出 12.56（Circle(2.0) 面积，与解释器一致）。
  注：match 模式绑定用 auto，需 -std=c23（zig cc 支持）。
- **用例 10 全过**（Shape_Circle 构造 / struct Shape s 推断 /
  return ((3.14 * r) * r) 断言）。
- **验证**：transpiler.aine 10 用例全过；aine 159+3 全绿。


## 自举 B5-M17：内置 Option/Result 类型映射（fl_opt）— 完成 ✅（本轮）

- **transpiler.aine 新增**（内置类型 → C 的类型化映射）：
  1. **fl_opt 类型化载荷联合体**：union { int i; double f; char* s;
     fl_vec v; void* p; + 用户类型 t_<T> }，替代原 void* 单槽
     （int 载荷转 void* 会编译失败）
  2. **构造宏分型**：fl_some_i/f/s/v/p/t_<T>（按 some_member 静态解析
     载荷类型），fl_none()；Ok 与 Some 同路径
  3. **match 绑定类型化提取**：按 scrut 静态类型（base_type → Option<T>
     泛型参数）选联合体成员 → auto x = o.data.s; / o.data.i
  4. **裸 None → fl_none()**：无括号 None 是 EIdent，此前生成未定义标识符
  5. **c_program 重排**：用户 struct/enum 定义先行（fl_opt 联合体需完整
     类型），函数定义随后——顺带修复类型前向引用
  6. **c_stmt 补 SBlock 分支**（match 臂块体此前生成 /* unsupported */）
  7. **print 代码生成提取为 c_print_stmt 助手**（match 臂里的 print 以
     语句输出 + break，不再生成 return print(...)）
- **闭环验证**（与解释器输出一致）：
  - Option<String>：describe(Some("hi")) → got:hi；describe(None) → none
  - Option<i32>：Some(5) match 提取 x → five（zig cc -std=c23 零错误编译）
- **用例 11-12 全过**（fl_opt sig / fl_some_s / fl_none / auto x = o.data.s /
  fl_some_i / auto x = r.data.i 断言）。
- **验证**：transpiler.aine 12 用例全过；aine 159+3 全绿。
- **已知缺口（下轮 B5-M18）**：nested 模式（SReturn(Some(e))）、match
  scrut 为函数调用（base_type 需扩展 fn_return）、literal 模式
  （Some("i32")）、struct 字段类型表。

## 自举 B5-M18：transpiler 自转译闭环（fixpoint）— 完成 ✅（本轮）

- **目标**：self_host.exe（由 Aine 生成的 C 编译而来）运行全部 13 用例，
  输出与 `aine run examples/transpiler.aine` 完全一致（自举不动点证明）。
- **修复链（从 case2 崩溃到 13 用例全过）**：
  1. **char 比较代码生成反逻辑**：c_operand 的 is_char_side 判定写反
     （false 时才转 'x'）→ 修正为 char 侧转单字符字面量；
  2. **is_char_typed 不识别 String 单索引**（src[i]）→ 补 EIndex 分支；
  3. **push 字符串字面量取址**：`&"infer"` 把字面量字节当指针拷贝
     （堆损坏源头）→ 统一 `&(char*){ ... }` 复合字面量；
  4. **循环变量名冲突**（for st in seed_types / for st in stmts）→
     扁平变量表 first-match 使 st 类型错成 String → 重命名 stp；
  5. **SStruct 臂缺 return out**（其余臂都有）→ 生成落到 unsupported → 补上；
  6. **EFloat/EInt 同绑 n** → f-string 把 f64 当 i32 → fl_num 截断 3.14→3 →
     重命名 EFloat 绑定为 nf（stringify/c_expr 两处）；
  7. **fl_fnum %g 丢小数**：2.0 → "2" → 整数值用 %.1f 补 ".0"；
  8. **var_info 循环变量/索引类型推断缺口**：for x in v.fields（EField 迭代器）、
     v.fields[i]（EIndex EField 基）→ 类型落成 char/String 错误 → 扩展推断；
  9. **expr_is_string EIndex 臂**：单索引 Vec<String>[i] 也视为 String；
  10. **EIdent(n)/EInt(n) 同绑 n** → `n == "true"` 指针比较失效 → 重命名 nn；
  11. **char vs String 变量比较**（c == sep）：生成指针比较永远 false →
      String 表达式侧取 [0]（char 即单字符 Str 语义）。
- **fixpoint 结果**：aine 生成 self_host.c（5606 行）与 self_host.exe 自生成
  输出逐行全同（diff = 0）——自转译闭环完全收敛。
- **验证**：aine exit 0；zig cc -std=c23 零错误；self_host.exe exit 0
  （case1-13 全过，含自转译 case13）。

## 自举 B5-M19：通用编译验证（f-string 调用插值 + 真实程序端到端）— 完成 ✅（本轮）

- **f-string 函数调用插值**：fstr_piece 对含 ( 的非 .len 表达式此前返回裸文本
  （int 传给 char* 参数编译失败）→ 提取调用名按 fn_return 返回类型转
  fl_num/fl_fnum（fstr_piece/fstr_c 签名扩展 fnames/freturns）。
- **case14（fib.aine）**：递归 + for 区间 + `f"fib({i}) = {fib(i)}"` 调用插值 →
  `fl_num(fib(i))`；断言 for (int i = 0; i < 10; i++) / printf %s。
- **case15（hello.aine）**：尾返回 + 中文串 + if-else + for 区间 + f-string 变量插值。
- **端到端验证（自举闭环可用性）**：
  1. aine run transpiler.aine → 15 用例全过（exit 0）
  2. self_host.c（5656 行）→ zig cc -std=c23 零错误
  3. self_host.exe → exit 0，15 用例全过
  4. **Aine 编译器（Aine 实现，原生运行）编译真实程序**：fib.aine → fib_gen.c
     → zig cc → 运行输出与 aine run 逐行一致（diff 0）；hello.aine 同理（diff 0）
  5. fixpoint 保持：self_host 自生成 == aine 生成（diff 0）

## 自举 B5-M20：字符串方法 split/trim/replace + 字面量后缀解析 + 方法调用类型推断 — 完成 ✅（本轮）

- **解释器新增方法**（interp.rs，编译器刚需的文本处理）：
  `trim()`（去首尾空白）、`split(sep)`（→ Vec<String>，空分隔符按字符切）、
  `replace(from, to)`（全部替换）。typeck 未知方法不报错，无需改动。
- **transpiler 字面量后缀解析修复**：parse_prefix 对 TInt/TFloat/TStr/括号表达式
  不调用 parse_postfix → `"a,b,c".split(..)`、`5.to_f64()`、`(x).len()` 全部
  「无法解析前缀: '.'」→ 补 parse_postfix 链。
- **方法调用类型推断**：新增 method_ret(mname, argc) 返回类型表
  （len→i32 / trim→String / split→Vec<String> / replace→String / to_i32→i32 /
  to_f64→f64 / starts_with·contains·ends_with→bool / push→()）；接入
  var_info_of_stmts SLet、c_stmt SLet 的 ct 推断、c_print_stmt 的格式选择
  （修 `let t = s.trim()` 生成 `int t` 与 `print(n)` 用 %s 的错误）。
- **case16（strings.aine）**：读取真实程序（trim + split().len() 链 +
   Vec 索引 + replace）→ 断言 fl_trim/fl_split(...).len/fl_replace。
- **端到端验证**：aine 16 用例全过；self_host 16 用例全过；strings_gen.c
  → zig cc 零错误 → 运行输出与解释器逐行一致（diff 0）；fixpoint 保持 diff 0。

## 自举 B5-M21：数组字面量换行分隔 + Err 载荷绑定 + Vec<struct> 类型推断 — 完成 ✅（本轮）

- **数组字面量换行分隔解析**：parse_prefix 的 `[` 分支跳过前导/项间 TSemi
  （`[1\n2]`、`[Record{...}\nRecord{...}]`），此前直接对换行调 parse_expr 报
  「无法解析前缀」；项间无逗号时按换行继续解析。
- **Err(e) 载荷绑定**：pat_bind_lines 此前把 Err 当 None（无载荷），
  `Err(e)` 生成 `auto e = <scrut>`（整个 fl_opt）→ 按 Result<T,E> 的 E 类型
  选联合体成员（新增 second_type_arg + SMatch 计算 err_mem，Err 分支与
  Some/Ok 同样支持嵌套）。
- **EVec 数组字面量类型推断**：非空数组 → Vec<首元素类型>（新增
  elem_expr_type：EStruct→类型名/EInt/EFloat/EStr/EIdent/ECall/method_ret），
  空数组保持 infer（push 按实参回退，避免 `let mut tokens = []` 后 push 结构体
  被误判元素大小）。修复 records 索引生成 `records[0]` →
  `((struct Record*)records.data)[0]`。
- **case17（records.aine 新示例）**：Vec<struct> 数组字面量 + 索引 + 字段访问
  + match Err 绑定打印错误。
- **端到端验证**：aine 17 用例全过；self_host 17 用例全过；records_gen.c
  → zig cc 零错误 → 运行输出与解释器一致（diff 0）；fixpoint 保持 diff 0。

## 自举 B5-M22：方法链 iter/map/sum（闭包内联）— 完成 ✅（本轮）

- **方法链代码生成**（c_expr）：
  - `.iter()` → 恒等（链起点）
  - `.map(|p| BODY)` → GNU 语句表达式 `({ ... })` 循环构建新 Vec
    （zig cc -std=c23 支持）；闭包参数作循环元素变量，体按元素类型推断
  - `.sum()` → 折叠链 `X.iter().map(|p| BODY).sum()` 为单循环求和
    （累加类型按闭包体返回类型 double/long）；普通 Vec 求和按元素类型
- **配套修复**：
  1. **闭包参数类型**：临时把闭包参数加入变量表（var_info 不收集闭包形参）
     → expr_ctype/c_expr 正确推断体类型（`r.amount` → f64）
  2. **SLet EIdent 初始化缺用户类型**：`let x = structVar` 生成 `int x` →
     补 `contains_str(user_types, t) → struct T`
  3. **print 的 f64 格式**：%g 打出 "4"（应 "4.0"）→ f64 实参用
     `%s` + fl_fnum（与解释器 Display 一致）
  4. 新增 expr_ctype（表达式 → C 类型）辅助
- **case18（mapsum.aine）**：方法链 + f64 求和；**case19（todo.aine 真实程序）**：
  方法链 + Result + ? 传播 + Vec<struct> 数组。
- **端到端验证**：aine 19 用例全过；self_host 19 用例全过；mapsum_gen.c /
  todo_gen.c → zig cc 零错误 → 运行输出与解释器一致（mapsum diff 0；todo
  两行输出一致 + 负数金额正确失败）；fixpoint 保持 diff 0；aine 159 测试全绿。

## 自举 B5-M23：Map 类型 C 后端 + json.aine 端到端编译 — 完成 ✅（本轮）

- **Map<String, T> C 后端**：新增 fl_map（keys/vals 双 fl_vec）+ c_type 映射；
  方法代码生成（语句表达式内联）：`Map.new()` → 空 fl_map；`.set(k,v)`
  （struct 值用临时变量避免复合字面量误初始化首成员）；`.get(k)` →
  Option<T>（按值类型选 fl_some_* 宏）；`.contains(k)`；`.keys()` →
  m.keys；`.values()` → m.vals；`.len()` → m.keys.len。
- **配套修复（6 处）**：
  1. **遮蔽参数**：`let mut i = i` 生成 `int i = i;` 自引用 UB → 临时变量
     捕获外层值 + SDef 函数体包嵌套块（C 参数不可同层重定义）
  2. **无载荷变体作值**：`JNull` 生成裸枚举常量 → `Owner_Name()` 构造器调用
  3. **EMatch 三元嵌套顺序**：`_` 兜底臂包在外层永远命中 → 逆序嵌套
     （先检查臂在外）
  4. **EMatch 绑定**：`Some(x) => x` 在语句表达式内声明 x（按 scrut
     Option<T> 载荷类型）；var_info/c_stmt 的 EMatch 推断 Option 载荷类型
  5. **bool 插值**：`f"{b}"` → fl_bool(b)（"true"/"false"）
  6. **set 复合字面量转义错误**：`((" + vct` 字面量泄漏 → 修正
- **case20（json.aine）**：B2 的 Aine JSON 解析器（Map<String, Json> +
  枚举 Map 载荷 + set/keys/get + Option match）→ 端到端编译。
- **端到端验证**：aine 20 用例全过；self_host 20 用例全过；json_gen.c
  → zig cc 零错误 → 运行输出与解释器一致（JSON 解析 + 序列化 + 往返 OK）；
  fixpoint diff=0；aine 159 测试全绿。
- **已知缺口**：char 实参传 String 形参需 fl_char_to_str 转换（json.aine 的
  is_digit 已改为 char 规避）；闭包独立值；import/module。

## 自举 B5-M24：char→String 实参自动转换（函数形参类型表）— 完成 ✅（本轮）

- **函数形参类型表**：collect_fn_ptypes（与 fnames 对齐，每项逗号连接形参
  类型串）+ fn_param_ty(idx, ai, fptypes) 查询；fptypes 穿表
  c_program → c_stmt → c_expr/c_operand/c_str_operand/c_print_stmt
  （80+ 调用点批量追加）。
- **char 实参转换**：c_expr ECall 定位函数索引，形参为 String 且实参
  char 类型 → `fl_char_to_str(实参)`（`is_digit(src[i])` →
  `is_digit(fl_char_to_str(src[i]))`）。
- **case21（char-arg）**：String 形参函数接收 s[0]/s[1] char 实参 →
  断言 fl_char_to_str(s[0])。
- **json.aine 恢复**：is_digit 从 char 改回 String（无需规避）→ 仍端到端
  编译运行正确（真实验证 char→String 转换）。
- **端到端验证**：aine 21 用例全过；self_host 21 用例全过；json.aine
  （String is_digit）→ zig cc 零错误 → 输出与解释器一致；fixpoint diff=0；
  aine 159 测试全绿。
- **已知缺口**：闭包独立值（非方法链）；import/module；Map 的嵌套类型
  （Map<String, Vec<T>> 值类型递归）。

## 自举 B5-M25：嵌套泛型类型（Map<String, Vec<T>>）+ case13 稳定性定论 — 完成 ✅（本轮）

- **case13「间歇性崩溃」定论（未复现，判定为调试期伪象）**：换用干净重建
  的二进制后连续 15+ 次运行零失败（6 次自转译压测 + 6 次 24 用例全量 +
  多次单测），且历史失败样本的被转译源大小在两次运行间不断变化
  （201163→201508 字节）——表明当时在运行间隙编辑被 read_file 的源文件
  （读-写竞争读入半截文件）并使用探针版二进制。fixpoint 重验：自宿主输出
  与 aine 输出逐行全同（仅存两处已知非差异：C 运行时 \n→\r\n、解释器
  末行多打印一次 Ok(()) 回显）。
- **解释器诊断增强（interp.rs）**：字段访问错误带结构体类型名与现有字段
  （"结构体 'A' 没有字段 'kind'（现有字段: x）"），并新增 Value::type_name。
- **Rust 前端修复（parser.rs）**：嵌套泛型闭合 `>>` 被词法为 Shr token →
  parse_path_with_generics 拆分（pending_gt 单槽机制，`>>>` 亦正确）；
  typeck 的 Ty::Map/Ty::Vec 本就递归，无需改动；159+3 测试全绿。
- **转译器修复（transpiler.aine，4 处）**：
  1. second_type_arg 重写：嵌套泛型保留内层 <>（此前 Option<Vec<i32>>
     被推断成 Option<Veci32>、Map 值类型丢失全部类型信息 → void*/fl_some_i）；
  2. 变量类型表补 keys()/values() 特判：values() → Vec<V>、keys() →
     Vec<String>（此前 values() → 裸 Vec、vs 被发成 int）；
  3. SLet 发射端：values/keys → fl_vec；set 的复合值
     （fl_vec/fl_map/fl_opt/struct）统一走临时变量路径（此前 Vec 值被
     (fl_vec){ fl_vec{...} } 双重包裹 → 非法 C）；
  4. f-string 索引插值：v[0] → ((T*)(base.data))[idx]（按元素类型
     int/double/String/char 选指针类型并包装 fl_num/fl_fnum/原样）。
- **case25（mapnest.aine）**：Map<String, Vec<i32>> 的 set/get
  （Option<Vec<i32>> 的 match 绑定）/for 遍历求和/keys/values 端到端。
- **端到端验证**：aine 25 用例全过；self_host（7317 行自生成 C，zig cc
  零错误）25 用例全过；mapnest_gen.c 原生输出与解释器 diff=0；
  fixpoint：9118/9118 行全同；aine 159+3 测试全绿。
- **已知缺口**：f-string 内方法链插值（v.iter().map(..).sum() 在 f-string
  中原样透传——插值是文本级管道，需先 let 中转）；int 链 .sum() 的
  method_ret 表面类型为 f64；闭包独立值；import/module。

## 自举 B5-M26：import / module（文件模块 + 扁平注册）— 完成 ✅（本轮）

- **工程期决策（非 D 登记簿范围，MVP 后按 §71.2 再演进命名空间）**：
  - `mod name;` 文件模块：加载器读入同目录 name.aine，项以**原名**进入全局
    命名空间（扁平注册）；递归加载 + 环路检测。
  - `import` / `use` Path：声明标记。名字已存在（如 `mod x;`）时引用之，
    不存在时占位注册；扁平语义下无命名空间效果。
  - 内联 `mod x { ... }` 维持既有扁平语义。
- **Rust 侧（aine）**：
  - ast：ModDef 增加 is_file；parser：`mod name;` 文件形态（吃掉分号）。
  - lib：resolve_file_modules（递归 + canonical 路径防环 + 缺文件/词法/语法
    错误诊断）；接入 run/ast/hir/typeck/interp/opt/vf/check 七个命令入口
    （fmt 保留 `mod x;` 原样输出）。
  - resolve：import 不再参与顶层命名 hoist（避免与 mod 重名 N3002），
    Import 臂改为 lookup→intern 占位。
  - 测试：tests/file_modules.rs 3 项（内联+运行 / 缺文件诊断 / 环路检测）。
- **Aine 转译器侧（transpiler.aine）**：
  - Stmt 增加 SImport(String)；parse_mod 支持文件形态（空体标记）；
    parse_import 读取到分号；stringify 支持。
  - flatten_stmts：内联 mod 拼接子项、文件 mod read_file+tokenize+parse
    递归打平（loaded 列表防环）；c_program 变薄壳——打平后**重算 11 张
    类型表**再进入 c_program_flat（原函数体零改动，保证 fixpoint 行为）。
  - 自举期限制：打平根目录固定 examples/。
- **case26（modtest）**：modtest_main.aine（mod + import + main）+
  modtest_util.aine（shout/repeat）→ 解释器运行正确；转译打平产物含模块
  函数前向声明与定义；zig cc 原生输出与解释器 diff=0。
- **自举狗粮拆分**：first_type_arg / second_type_arg / method_ret /
  payload_member 四个类型工具移入 examples/fltype.aine，transpiler.aine
  顶部 `mod fltype;`——上帝文件首次真实拆分；Rust 侧加载（aine run）与
  Aine 侧打平（自转译）双路径验证。
- **验证（串行单跑）**：aine 全套测试 159+3+3 全绿；26 用例全过；
  self_host（7460 行自生成 C，zig cc 零错误）26 用例全过；modtest 原生
  diff=0；**fixpoint 9263/9262 行全同**（唯一差异：解释器末行多打印一次
  Ok(()) 回显）。
- **重要教训（再次证实 case13 定论）**：本轮复现过程中曾出现退出码 127 /
  0xC0000374（堆损坏）/ "结构体 'Token' 没有字段 'kind'（现有字段: , text）"
  ——全部发生在**后台全量运行与并行调试进程/文件改写重叠**时；串行单跑
  全部消失。字段名出现逗号的 Token 正是读入半截文件的字节流垃圾（新诊断
  消息直接解出）。结论：heavy run 必须串行、运行中不得改写其读取的文件。
- **事故记录**：清理时 bash 通配符 `rm examples/s*.aine` 误删了真实示例
  strings.aine（rm 不走回收站）；已从上一会话日志的原始 write 调用中
  完整恢复（315 字节，与 case16 断言吻合）。

## 自举 B5-M26.1：transpiler.aine 上帝文件拆分完成（六模块）— 完成 ✅（本轮）

- **拆分结果**：根文件 4649 → 382 行（仅 AST 类型定义 15 个 + 26 用例 main）；
  fllex（词法，151 行）/ flparse（解析 + 模块打平，897）/ flstr（字符串工具
  与源码回显，309）/ flcollect（顶层收集器，257）/ fltype（类型查询，扩至
  45 函数）/ flee（C 发射，1718）。
- **拆分暴露并修复的三个真问题**：
  1. **resolve 跨模块前向引用**：模块项在 pass2 按源顺序展开，模块 A 引用
     后方模块 B 的函数报 N3001 → pass1 改为 hoist_names 递归提升全部模块
     名字，pass2 只解函数体（resolve_bodies），完整保留前向引用能力；
  2. **跨文件 span 碰撞**：AST 加载器内联保留各文件自身偏移，typeck 的
     HashMap<Span,_> 把模块项与主文件同数值 span 互相覆盖（幽灵 T3010：
     "不能给不可变绑定 'out' 赋值" 实为别的绑定）→ 加载器重构为**文本级
     内联** read_source_with_modules：`mod x;` 行替换为 x.aine 全文，虚拟
     合并源内 span 同一坐标系；load_source 全命令生效（fmt 保留原始源）；
  3. **自举元编程陷阱**：flatten 的 read_file 失败检测 contains("[read_file
     error") 在读取包含同名字面量的转译器自身源码时误判 → 改前缀匹配；
     String.len() 出现在普通表达式位置时类型表推断为 infer →
     var_info_of_stmts 补 read_file → String（发射端 strlen 路径已有）。
- **验证（串行单跑）**：check OK（117 顶层项 / 34186 token）；26 用例全过；
  自宿主 7464 行 C zig cc 零错误；fixpoint 9270 行逐行全同（除末行 Ok(())
  回显）；cargo test 159+3+3 全绿；fib/hello/mapnest/modtest 回归全过。

## M6-STD-1：标准库启动（stdlib/math + strutil + collections）— 完成 ✅（本轮）

- **stdlib 位置与搜索路径**：aine/stdlib/（编译器自带标准库，自举刚需）；
  `mod name;` 解析顺序：同目录 → 同目录../stdlib → CWD stdlib（Rust 加载器
  read_source_with_modules）；Aine 侧 flatten 对应 dir → stdlib/。
- **首批模块（纯 Aine 编写，同时是语言真实用例）**：
  - math：abs_i32/abs_f64/min_i32/max_i32/clamp_i32/gcd/pow_i32
  - strutil：reverse/count_char/pad_left/starts_with_digit（ASCII 范围）
  - collections：range_vec/vec_reverse/vec_max/vec_concat
- **examples/stdtest.aine**：27 号端到端程序（f-string 调用插值 + 字符串
  切片/比较 + Vec 遍历求和 + 显式类型注解）。
- **修复两个真问题**：
  1. **C read_file 与解释器语义不一致**：解释器失败返回 "[read_file error…]"
     标记，C 侧返回 ""——自宿主的 stdlib 回退在 C 里永远不触发，拿着空串
     继续编译导致静默断言失败（exit 1 无任何输出）。发射的 read_file 已
     改为返回同一标记（fail-visible）。
  2. **String.len 字段发射缺口**：`out.len`（字段形式）在 String 上被原样
     发成 `.len`（非法 C）→ c_expr EField 对 String 类型发 strlen
     （Vec/未知类型维持旧行为）。配套发现 var_info_of_stmts 缺 EIdent 初始化
     推断臂——**启用它会触发 json.aine 发射期 索引越界 idx=0 len=0**（新增
     越界消息带 idx/len），暂时禁用并记录为已知问题；stdlib 侧用显式类型
     注解（`let mut out: String = s`）规避。
- **验证（串行单跑）**：27 用例全过；自宿主 7464 行 C zig cc 零错误；
  stdtest 原生输出与解释器 diff=0；**fixpoint 9461 行逐行全同**（自宿主
  首次以自身 C flatten 加载标准库完成自转译）；cargo test 159+3+3 全绿；
  modtest/fib 回归通过。

## 自举 B5-M27：闭包独立值 — 完成 ✅（本轮）

- **语法**：`let f = |x: i32| body`（独立闭包值；参数必须全部带类型注解，
  非捕获 MVP）。解释器原生支持（Value::Closure + 变量调用通路既有）；
  Rust parse_closure 补解析 `: Type` 注解。
- **FLOW 侧**：EClosure 改为携带 Vec<Param>（带类型）；parse_closure 支持
  类型注解；parse_ty 增加 `|` 终止符（此前吞掉闭包体）。
- **C 发射（脱糖提升）**：新增 desugar_closures——`let f = |x: i32| body`
  提升为顶层 `fn f(x: i32) -> <expr_ctype 推断> { return body }`，
  调用点零改动（闭包名即函数名）；仅提升 fn 体顶层非捕获闭包，
  参数未注解时保持原样。约束：闭包变量名提升后需全局唯一。
- **连带修复**：
  1. push 变体构造的取址：`lifted.push(SDef(...))` 发成 `&(struct Stmt){调用}`
     仍非法（构造函数返回整个结构体，复合字面量按成员初始化）——改为
     desugar 源码用临时变量承接再 push（取址合法 lvalue）；
  2. 解释器越界错误带 idx/len 上下文（`索引越界: idx=0 len=0`）。
- **case28（closuretest.aine）**：i32/双参/String 闭包 + f-string 调用插值
  + 条件中的闭包调用；解释器与原生 diff=0。
- **验证（串行单跑）**：27 用例全过；自宿主 C（含 desugar 机制自举）
  zig cc 零错误；fixpoint 9735 行逐行全同；cargo test 159+3+3 全绿。

## typeck/formatter 组件替换：分期计划（M6-FMT / M6-TCK）

- **现状**：fmt.rs（908 行 Rust）服务 `aine fmt`；typeck.rs（1167 行）
  服务 `aine check`。自举编译器的对应职责实际由 transpiler 的类型表
  （fltype/flcollect）+ C 编译验证承担。
- **M6-FMT-1（起点，低风险）**：fmt_tool.aine——用 flparse 的
  lex/parse + flstr 的 stringify 系（已有 300 行格式化代码）组成
  独立原生格式化工具；验收：幂等（format∘format = format）且输出可重解析。
- **M6-FMT-2**：对齐 Rust fmt.rs 的排版规则（缩进/换行/注解宽度），
  fmt_tool 输出与 aine fmt diff=0。
- **M6-TCK**：分期迁移 typeck 诊断（先表达式/let 类型不匹配，后 trait/泛型）；
  前置依赖：Index 节点携带 span（同时解决 EIdent 推断臂越界定位问题）。

## 自举 B5-M28：语言特性收尾冲刺 + 值流子集(WIP)——部分完成 🔶（本轮）

- **✅ ①c 嵌套 Map 值类型**：`Map<String, Map<String, i32>>` 端到端
  （nested.aine）解释器与原生输出一致。修复三处：payload_member 补 Map→p、
  get 的宏选择补 fl_some_p、match 绑定对 p 载荷带类型声明
  （`fl_map c = *(fl_map*)(got.data.p)` 形式，仅 Map/Vec/struct 载荷，
  原始载荷保持 auto 兼容既有断言）。
- **✅ ①d 闭包捕获**：解释器支持验证（captest=15 正确）；原生编译
  fail-visible（zig 报 use of undeclared identifier，不静默）。捕获的
  env 结构体发射列为后续项。
- **✅ 诊断体系增强（永久保留）**：解释器越界错误带 idx/len/向量内容/
  Rust backtrace；eval_call 实参求值错误带调用者表达式标签
  （"[在调用 X 的第 N 个实参中]"）——本轮两个深坑均靠它定位。
- **✅ 附带修复**：Vec<String>/Vec<f64> 字面量发射（char*[]/double[]）、
  int 型 .sum() 链的类型判定（f-string 插值不再变 6.0）、parse_ty 增加
  `|` 终止符、Rust parse_closure 支持 `|x: T|` 注解。
- **🔶 ①a f-string 方法链插值**：AST 路径已实现（piece 级 tokenize+
  parse_expr+c_expr+expr_ctype 包装，解释器与原生均验证 sum=6），但
  禁用实验的三元式混型 hack 使自宿主 C 编译失败 → AST 路径代码移入
  wip_fstr_ast_path.aine.txt（未启用），fstr_piece 回到文本版。
  启用前提：if-表达式分支异型的发射支持。
- **🔶 ② 值流子集 v0.1**：完整实现（vf_scan 递归收集 + 保守白名单判定
  + vf_rewrite 改写为 fl_strcat_own 原地追加，191 行，存
  wip_valueflow_lite.aine.txt）——接线后自举期触发 pat_bind_lines 的
  idxs[k-1] 越界（模式绑定路径与构造器深度失配，vec=[0] 已精确定位）。
  未接线停用，待 pattern/Index 携带位置信息后修复。
- **🔶 ①b EIdent 推断臂**：深挖至 fn_return 的 Some(freturns[idx]) 空表
  越界链路仍未闭环 → 保持禁用（显式注解规避），诊断设施已就位。
- **验证（串行单跑）**：check OK；27 用例全过；自宿主 C zig cc 零错误；
  fixpoint 9783 行逐行全同；cargo test 159+3+3 全绿；
  modtest/nested/captest/stdtest 回归通过。
- **诚实说明**：本轮目标"①②③全部完成"未达成——① 4 项中 2 项完成
  2 项部分（a/b 有完整实现但被发射层前置问题阻塞启用），② 完成实现
  但被同一发射层问题阻塞（两者共用 pat_bind_lines/if-expr 发射短板），
  ③ 未开始。发射层的这两个短板（pat_bind_lines 位置信息、if-expr
  分支异型）已列为最高优先。

## 自举 B5-M28.1：发射层修复 + 值流子集激活 — 完成 ✅（本轮）

- **✅ pat_bind_lines 路径层数修复（根因修复）**：内层循环按模式构造器
  层数（nl）迭代，而每个绑定的访问路径应按自身路径层数（idxs.len()）
  迭代——纯标识符绑定的路径短于 ctor 深度（兄弟变体也占 ctor 层）导致
  idxs[k] 越界（idx=1 len=1 vec=[0]，vec 内容显示直指 str_split 结果）。
- **✅ var_info_of_stmts 同类修复**：SMatch 臂 `idxs[nl2 - 1]` →
  `idxs[idxs.len() - 1]`（绑定位置 = 路径末段）。
- **✅ 值流子集 v0.1 激活**：vf_scan/vf_rewrite 模式扁平化
  （EBin(pop, mid, r) + 内层 match，消除嵌套变体模式）+ 构造 push 全面
  临时变量化 + fl_strcat_own/fl_strdup_lit 定义为真实 Aine 函数
  （解释器语义正确；原生发射按名字跳过 Aine 体、使用 header C 内建
  realloc 原地追加；char 实参经 fptypes 自动转换）。
- **验证（串行单跑）**：27 用例全过（值流子集激活：14 处拼接改写为
  原地追加）；自宿主 C zig cc 零错误；fixpoint 10156 行逐行全同
  （自宿主同样运行值流子集）；cargo test 159+3+3 全绿；
  modtest/nested/captest/stdtest/closuretest 回归全过。
- **✅ 永久诊断资产**：解释器 fn 栈跟踪（运行时错误带
  "于函数 a <- b <- c" 上下文）+ 越界错误带 idx/len/向量内容/回溯 +
  eval_call 调用者标签——本轮全部深坑由此定位。
- **🔶 遗留（有精确线索，未闭环）**：
  1. fstr AST 路径：直接 parse_expr 形式在自编译语境下，部分函数
     （如 enum_field_ty 的 f"{cur}"）片段类型推断失败（表来源问题），
     代码存 wip_fstr_ast_path.aine.txt；解释器侧不受影响。
  2. EIdent 推断臂：var_type 收到空 types 表（names/types 某分支
     失配，fn_context 定位链路 main <- collect_var_names <-
     var_info_of_stmts <- var_type），保持禁用；注解规避。
  3. 闭包捕获原生发射（env 结构体），解释器 ✓/原生 fail-visible ✓。

## 自举 B5-M28.2：架构级发现 —— C 后端 fl_vec 值语义不健全（B5-M29 立项）

- **发现**：fstr AST 路径与值流子集两个 WIP 在自宿主(C)执行时段错误，
  与 vf 接线无关（撤除后仍复现）——根因为 **C 后端 fl_vec 赋值是浅拷贝
  （{len,es,data} 指针共享），任何一方 push 的 realloc 会使共享者悬空**。
  解释器因 Vec push 重绑定新 Arc（copy-on-append）而免疫 —— 两后端语义
  分叉是架构级的。
- **修复方向（B5-M29，真正的值流工程）**：
  1. 安全基线：C 后端 fl_vec 赋值发射深拷贝（fl_vec_clone），语义与
     解释器一致（慢但正确）；
  2. 值流优化：move/末用分析判定可安全省略的深拷贝（正是 Aine 设计的
     copy-by-default + move 优化在自举编译器中的落地）；
  3. 解锁两个 WIP：fstr AST 路径 + 值流子集 v0.1（代码均已存档）。
- **本轮保留的永久修复**：pat_bind_lines 迭代边界（路径层数）、
  var_info idxs 末段、SLet 类型先/名字后的入表顺序（影子惯用法）、
  Vec<String>/f64 字面量发射、int-sum 类型、parse_ty `|` 终止符、
  Rust 闭包注解解析、fn 栈跟踪与越界诊断（idx/len/内容/回溯/调用标签）、
  文件读取重试（消除 AV 竞态类间歇故障）。
- **验证（串行单跑）**：27 用例全过；自宿主 C zig cc 零错误；
  fixpoint 10299 行逐行全同；cargo test 159+3+3 全绿；7 个回归程序全过。
- **自举线状态：≈99%**。剩余：B5-M29（C 后端值语义 + 两个 WIP 启用）、
  闭包捕获原生发射（env 结构体）、typeck/formatter 组件迁移。

## 自举 B5-M29：C 后端值语义（move/clone）+ 值流子集激活 — 完成 ✅（本轮）

- **值语义基建**：解释器新增 fl_vec_clone/fl_zero_vec/fl_zero_map/
  fl_map_clone/fl_strdup_lit/fl_strcat_own 六内建（语义与 Aine 一致；
  interp Vec 为持久化结构，clone 即共享引用）；C 侧发射对应 header
  助手（fl_vec_clone 真深拷贝 / fl_zero_* 置空 / fl_map_clone 键值双拷贝）。
- **value_semantics 趟（flparse）**：sem_scan（保守收集 String 字面量/
  Vec/Map 声明与别名）→ sem_rewrite（var-from-var 的 let：源后续未提及
  → MOVE（绑定 + 源置空）；仍被使用 → CLONE（显式深拷贝））。判定用
  stringify 全文扫描——假阳性只会退化为 clone，方向安全。
  关键实现约束（C 后端）：sem 仅处理复合类型（Vec/Map）——int 变量
  误入会发 fl_vec_clone(i) 类型错误；切片切片 `stmts[a..]` 需改为显式
  Vec push 构建（fl_slice 返回 int 语义不匹配 Vec<Stmt>）；块构造 push
  全部临时变量化。
- **值流子集 v0.1 在值语义基线上激活**：move/clone 显式化后，累积器
  复用（fl_strcat_own 原地追加）不再依赖共享语义，自宿主执行安全。
- **fstr AST 路径**：仍段错误——根因进一步精确：fstr_piece 内 toks
  Vec 的跨语句借用/生存期（C 后端生存期安全缺口，B5-M30，需 Rc 式
  引用计数或缓解发射）。代码存 wip_fstr_ast_path.aine.txt。
- **验证（串行单跑）**：27 用例全过；自宿主 C zig cc 零错误；
  fixpoint 10789 行逐行全同；cargo test 159+3+3 全绿；7 回归程序全过。
- **自举线 ≈99.5%**：值语义与值流子集（Aine 设计的性能语义核心）已在
  自举编译器落地。剩余：B5-M30（生存期安全 → 解锁 fstr AST）、
  闭包捕获原生发射、typeck/formatter 组件迁移。

## 自举 B5-M30/M31：闭包捕获原生 + 生存期定位 — 部分完成 🔶（本轮）

- **✅ 闭包捕获原生（隐藏参数提升）**：desugar_closures 扩展——捕获分析
  （var_info_of_stmts 取外层函数局部变量/参数，body 文本包含即捕获）→
  提升为隐藏前缀参数 → 程序级调用点改写 rewrite_program_calls
  （含嵌套实参递归 + f-string 字面量文本改写 str_replace_all，capture
  文本预渲染 lift_caps_txt 避免 Vec<Vec<Expr>> 嵌套类型问题）。
  captest（i32/String 捕获 + f-string 内嵌捕获调用）解释器与原生
  **端到端 diff=0**（15/hi!!/mk=107/base_still=100）。
- **✅ var_info 字面量类型**：EInt→i32/EFloat→f64（消除隐藏参数 void*）。
- **🔶 自宿主 case13 回归（新发现，未闭环）**：闭包捕获改动合入后，
  自宿主（self_host.exe）在 case1 即失败——其生成的 c1 大量出现
  "/* unsupported stmt */"（c_stmt 默认臂，恰好 3 次，分支匹配齐全）。
  宿主侧 27 用例全过、captest 端到端 diff=0，说明 Aine 语义正确；
  问题特异性地存在于【自宿主二进制执行自己的 c_stmt】路径。
  已插桩确认：unsupported 触发 3 次、var_type 无越界、FAIL@13
  （case1 printf 断言）。怀疑方向：desugar/lift 重写后 stmts 结构在
  自宿主解析路径上产生宿主没有的形态，或自宿主枚举 tag 与分支错位。
  **处理**：时间盒到，记录现场；自宿主 fixpoint 需在下一轮修复
  （当前 fixpoint 断言失败，但宿主侧一切验证绿）。
- **诚实结论**：闭包捕获的语言能力已完整落地并端到端验证；自举线的
  fixpoint 闭环被本次改动打破，需下一轮专项修复（现场已充分插桩）。

## 自举 B5-M31：自宿主 fixpoint 修复（match 尾构造发射缺口）— 完成 ✅（本轮）

- **根因（铁证定位）**：闭包捕获改动合入后自宿主 case1 失败。系统性插桩
  （c_stmt 入口 tag / RPC 入口 / per-fn / per-stmt / t4 返回值）锁定:
  **rewrite_lift_calls 返回的 struct Stmt tag=垃圾值(598827248)** ——
  生成的 C 中 `Stmt_SExpr(e2);` 构造调用后**返回值被丢弃**!
  Aine 源形态:「match 臂尾表达式 = 构造调用」作为函数返回值时,转译器
  把 match 尾表达式当**语句**发射(丢返回值),函数返回未初始化 struct。
  此前所有 match-as-value 均为简单表达式(变量/字面量),构造调用是
  首次出现的形态。
- **修复（源层规避 + 已知缺口登记）**：rewrite_lift_calls 改为显式
  `res = 构造; return res` 形式。自宿主恢复:27 用例全过、fixpoint
  11088 行逐行全同。
- **转译器登记缺口（B5-M32 候选）**：match 尾表达式为构造调用时,臂体
  应发射为 `return 构造(...)`（或对 match-as-tail 统一先存临时再 return）。
  当前以源层规避,宿主语义正确(解释器一直正确)。
- **验证（串行单跑）**：27 用例全过;自宿主 C zig cc 零错误;fixpoint
  11088 行全同;cargo test 159+3+3 全绿;7 回归程序全过;
  captest(闭包捕获)端到端 diff=0 维持。
- **自举线状态:≈99.5%**。语言特性层全部完成(嵌套类型/模块/stdlib/
  独立闭包/捕获闭包/值语义/值流子集)。剩余:fstr AST(生存期,B5-M30
  定位到 c_expr 内部)、match 尾构造发射缺口(B5-M32,已规避)、
  typeck/formatter 组件迁移。

## 自举 B5-M32：match 尾构造发射验证 + 自举线定稿 — 完成 ✅（本轮）

- **B5-M32 收尾**：尝试在转译器层修 match 尾构造(SDef 尾 EMatch → SReturn
  改写),连带发现 Some(EMatch) 的载荷成员判定缺口(some_member 补
  EMatch→t_Stmt、EClosure→t_Expr)。综合评估后:
  rewrite_lift_calls 保留**语句式改写**(if/else + res 累积,不依赖尾 match
  语义),SDef 尾改写回退——转译器层完整修复列为后续(需尾部检测上下文),
  源层规避已稳定承载全部现用代码。
- **fstr AST**:重新启用后在自宿主仍段错误(崩于 c_expr 处理
  ECall(EField(len))),正式登记为**已知限制**——等价写法(f-string 内
  先 let 中转)完全覆盖功能;解释器侧完整支持。B5-M33 候选(需 C 后端
  调试器级分析)。
- **最终状态(自举线 100% 基线定义达成)**:
  - 语言特性:嵌套泛型/文件模块/stdlib/独立闭包/捕获闭包(隐藏参数提升)/
    值语义(MOVE+CLONE)/值流子集(原地追加)全部在自举编译器中实现并自举;
  - 管线组件:fllex/flparse/flstr/flcollect/fltype/flee 六模块 +
    desugar/value_semantics/valueflow_lite 三趟,全部 Aine 编写;
  - 已知限制(均登记,均有等价 workaround):f-string 内方法链直插
    (let 中转替代)、fstr AST 段错误(B5-M33)、typeck/formatter 组件迁移
    (M6 分期计划)。
- **验证(串行单跑)**:27 用例全过;自宿主 C zig cc 零错误;fixpoint
  11110 行逐行全同;cargo test 159+3+3 全绿;7 回归程序全过。

## M6-TEST / M23：测试框架 — 完成 ✅（本轮）

- **目标**：`能用 Aine 写的用 Aine 写`推进计划之 M23。Aine 程序的测试
  用 Aine 语法书写（`#[test]` 属性），由 `aine test` 一键运行汇总。
- **实现**：
  - `#[test]` 属性解析已存在（parser `parse_attribute`，属性串 `"test"`
    存入 FnDef.attributes），本轮补齐**传播与消费**：
    `Interp::load` 收集带 `test` 属性的函数名入 `test_fns`（源码顺序、
    去重），新增 `Interp::test_names()` 访问器；
  - `aine test <file>` 子命令（main.rs `run_test`）：
    共享前端流水线（新增 `front_pipeline`：读源含模块内联 → 词法 →
    语法 → 解析 → 类型检查，任一阶段失败输出渲染诊断并退出）；
    每个测试在**全新解释器 + 独立 512MB 栈线程**中运行（隔离、确定性）；
  - **失败判定三类**：返回 `Result` 且为 `Err(...)`（载荷即原因）、
    运行时错误（`rt_error_text`）、线程 panic（其余测试不受影响）；
    正常返回（含 Ok/Unit）即通过；汇总 `N 通过, M 失败`，失败退出码非 0；
  - 无测试时的报错给出 `#[test]` 用法示例（用户导向诊断）。
- **附带修复（报错用户导向收敛）**：
  - 索引越界消息不再默认附宿主机 Rust 栈帧（13 行 `std::backtrace`
    噪声）—— Aine 层信息（idx/len/内容/函数栈）本已具体；宿主栈帧改由
    `FLOWC_RT_BACKTRACE=1` 显式开启（仅调试解释器自身需要）；
  - `fn_context()` 跳过空函数名、current_fn 与栈尾重复时不再重复输出
    （修复「于函数  <- xxx」双空形态）。
- **验收集**：`examples/testdemo.aine`（5 个 #[test]：算术/fib/字符串
  反转/排序求和/vec 查询，走 stdlib 模块，5/5 过、退出码 0）；
  `examples/testfail.aine`（1 过 2 故意失败：Err 失败与越界失败演示，
  退出码 1，失败原因逐条给出）。
- **回归测试**（interp_tests.rs 新增 3 项，cargo test 162 全绿）：
  属性传播到 AST、测试发现顺序与运行分类、testdemo 全套端到端。
- **文档同步**：Flow_Language_Reference.md 新增 §12 测试；自举路线图
  M6 交付物勾选 M6-TEST-1。

## M2 Conformance 子集 + 一元优先级修复 — 完成 ✅（本轮）

- **目标**：推进计划 M2 —— 类型/Ownership(值语义)/Value Aine/Summary
  从既有能力提炼为**正式可运行用例集**。形态与 M23 测试框架天然协同：
  正向行为集 = `aine test` 运行的 Aine 测试文件；负向诊断集与
  VF/Summary 契约 = cargo test API 级断言。
- **正向行为集（conformance/，31 个 #[test] 全过）**：
  - conf_types.aine（10）：字面量推断/结构体字段/内嵌 Vec 字段/枚举带
    载荷 match/Option Some-None/i32 四则与一元负号/f64/String 拼接相等/
    布尔逻辑/Vec 索引读取；
  - conf_values.aine（6）：值语义契约 —— 传参后源副本独立、拼接不改源、
    结构体字段互不干扰、调用方实参保持不变、函数返回新所有权值、
    字段传参后其余字段可读；
  - conf_closures.aine（6）：独立闭包值（1/2 参、String/i32）、捕获
    闭包（隐藏参数提升）、捕获后源变量可读、闭包谓词；
  - conf_errors.aine（5）：Ok/Err 构造判定、match 解构、`?` 传播链、
    首次失败即返回、Option 两路；
  - conf_modules.aine + confmod_helper.aine（4）：`mod` 同目录载入、
    扁平注册原名直调、模块内类型跨模块透明、stdlib 搜索路径。
- **负向诊断集（src/conformance_tests.rs，6 项）**：错误程序必须被拒绝
  且错误码稳定 —— T3001 let 类型不符、T3008 实参数不符、N3001 未定义名
  （含 did-you-mean 建议内容断言）、T3003 非布尔条件、T3006/T3009 结构体
  多余字段、正确程序零误报。
- **VF/Summary 契约（同文件，5 项）**：全函数覆盖摘要、参数类型/纯度
  （无任务/无全局写）/返回来源、VIR 每函数非空、合规程序零所有权错误、
  ty_is_copy 与 SizeClass 分类契约。
- **一元运算符优先级修复（Conformance 首战立功）**：conf_types 的 i32
  四则用例暴露 `-5 + 3 == -8`！parse_prefix 一元操作数以 min_bp=13 解析，
  与 `+/-` 的 lbp(13) 相同导致操作数吞掉后续加减（实际按 `-(5+3)` 解析），
  违反语法 §2.5（UnaryExpr 层级高于 Add/Mul）。修复：一元操作数
  min_bp 13→14（四站点 Minus/Bang/Ampersand/Star），`-5+3=-2`、
  `-5*3=-15`、`10- -3=13` 全部符合语法表。cargo test 162 全绿（无既有
  用例依赖旧错误行为）。
- **自举 fixpoint 复验（解析器改动后的硬门）**：host 27 用例全过 →
  重提 self_host.c（9036 行）→ zig cc -std=c23 零错误 → self_host.exe
  27 用例全过 → 输出逐行比较：11379 行中仅 1 行不同（host 运行器尾部
  额外打印 main 返回值 `Ok(())`，非编译器输出差异）。**fixpoint 成立**。
  （注：本次比较以行尾归一进行 —— host stdout 为 LF、C 程序为 CRLF，
  此前记录的"逐行全同"同为此口径。）
- **M6-TCK 评估结论**：typeck 子集迁移 Aine 的前置"Index 节点携带
  span"仍未满足（parser.rs 构造 Expr::Index 无 span 字段）；迁移维持
  分期计划不动，本次不强行迁移。
- **全量验证**：cargo test 173 全绿（162 既有 + 11 conformance）；
  6 个测试套件 36 用例全过；fib/parser_expr 等示例回归正常。

## M4 语言面补全：assert / 索引赋值 / 统一 fn 类型 / f-string 嵌套引号 / 字符索引 — 完成 ✅（本轮）

- **目标**：用户导向语言面缺口全修复（简洁高效易学，编译器承担复杂度）。
  设计决策 D14（统一函数类型+捕获即快照）、D15（字符串字符索引）入登记簿。
- **① assert 内建**：interp native（1/2 参形态，非 bool 明确报错，
  失败消息带函数栈）+ BUILTIN_GLOBALS 注册 + C 运行时 fl_assert
  （stderr + abort）+ flee 发射改写（缺省补消息）。
- **② 索引赋值 v[i]=x / m[k]=v**：解析器本已接受（通用 Assign 目标）；
  interp 加 Index 目标臂（Vec 原地重建重绑定 / Map 键值替换 / String 明确
  拒错并给等价写法）；typeck 的 mut 资格检查扩展到索引目标（T3010）；
  Map 读取路径补 String 索引臂（与写入对称，键不存在报错带键名）；
  **自举编译器全涟漪**：Stmt 枚举加 SAssignIdx 变体、flparse 两个解析位、
  rewrite_lift_calls 兄弟臂、flstr stringify、flee C 发射
  （`((ELEM*)v.data)[i] OP rhs;`，类型未知发射 #error 显式失败）。
  值语义验证：副本写入不影响源。
- **③ 统一函数类型（D14）**：`fn(A,B) -> R` 进双解析器
  （Rust parse_type KwFn 臂 / Aine parse_ty 括号深度感知——含分隔符
  先判后减深的顺序要点）；Ty::Fn + display + assignable（Fn-Fn 宽松同型）
  + type_of_type；resolve_type Fn 臂；fmt/valueflow 穷尽臂；
  **C 发射**：c_param_decl 把 fn 类型参数发为 C 函数指针 `R (*name)(A, B)`
  （flee 两个参数发射位统一换用）。调用点零改动（指针调用即 C 惯例）。
  语义：捕获即快照（interp env.snapshot() 既有行为正式化）。
- **④ f-string 嵌套引号**：宿主 lexer 新增 scan_fmt_string_body
  （插值深度感知，嵌套字符串消费到闭合引号）；fllex 的 f-string 扫描
  同步实现（含"引号先判终止再判嵌套"的教训修复——首版漏 depth==0 终止
  分支导致 f-string 吞掉后续源码，探针定位后修复）。
- **⑤ 字符索引（D15）**：解释器 len/索引/切片本已字符级（本决策正式化，
  中文验证 zhstringtest：len==6/`[0]=="你"`/反转/切片全对）；C 后端
  UTF-8 辅助函数（fl_chars_len/fl_char_off/fl_char_at/fl_substr_ch）
  已入运行时**备妥但未切换发射**——实测发现自举代码内部依赖 len 与
  索引的自洽性（仅 len 切字符会导致含中文源码在编译路径错位，
  自宿主 case13 解析失败 token 减半）；C 侧统一切换列为 M6-TCK
  类型表落地后的后续项（登记簿 D15 已记录实测依据）。
- **自举编译器暴露的既有缺口（本轮实证，源层规避）**：
  match 嵌套解构模式（`EIndex(EIdent(x), y)`）C 发射不正确 →
  平坦模式 + Option 中转（expr_ident_name 辅助）；
  match 表达式块臂尾值发射为 0、无类型绑定的 f-string 插值误发 fl_num →
  均为 M6-TCK 类型表家族前置项，已实证登记。
- **Conformance C6**（conf_language.aine，9 用例全绿）：assert 通过/
  索引赋值 Vec/Map/值语义/原地排序/f-string 嵌套引号/fn 类型具名+闭包/
  中文字符索引。assert 失败演示移入 examples/testfail.aine。
- **全量验证（串行单跑）**：cargo test 173 全绿；7 个测试套件
  （40 conformance + testdemo 5 + testfail 4）全过；自宿主 fixpoint
  成立——29 用例全过（新增 case29 索引赋值+assert、case30 函数指针），
  11873 行仅宿主运行器尾行 Ok(()) 口径差；zig cc 零错误。
- **文档同步**：Reference §5.3（字符索引）/§11.4（统一函数类型）/
  §11.5（嵌套引号）/§13（断言与索引赋值）；登记簿 D14/D15；
  自举路线图 M6 清单更新。

## ABCD 补全轮：aine build / Map 索引赋值 C 路径 / M3 分配计数实测 — 完成 ✅（本轮）

- **C2 `aine build`（新增 CLI）**：`aine build <file> [-o out]` → 前端检查 →
  解释执行 Aine 转译器生成 C → zig cc 原生可执行。机制：Interp 新增
  inject_global，宿主注入全局 `cli_args`（run 命令的额外实参同样透传给
  程序）；transpiler 增加编译模式（args.len>=2 时按 [目标,输出C] 编译）；
  **编译产物同样支持命令行实参**（flee 发射 `int main(_argc,_argv)` +
  全局 `fl_args` 填充，EField/EIndex 特例映射）。无 CLI 入口的缺口就此关闭。
- **A3 Map 索引赋值/读取 C 路径**：运行时新增 fl_map_find/set_i/f/s、
  get_i/f/s 七助手（键 char*，值三分型；键缺失 abort 带键名）；flee 的
  SAssignIdx 补 Map 分支（`=` 直写、复合赋值读-改-写）；EIndex 读取补
  Map 分支。**裸 `Map.new()` 推断不足** → 需显式注解
  `let mut m: Map<String, i32>`（否则 #error 给出可操作指引）。
  端到端验证：解释器与编译产物输出一致（含中文键值）。
- **C1/M3 闭包与容器真实分配计数（实测数据）**：C 运行时注入计数器
  （malloc/realloc 宏包裹 + constructor 注册 atexit，FLOW_ALLOC_STATS=1
  时 stderr 报告）。实测：**fib(20) 纯递归 = 0 堆分配**（数值全栈上）；
  **Vec push 100 次 = 100 次分配**（每 push 一次 realloc——B5-M29 值语义
  架构的量化实证）；**Map 3 键 = 6 次分配**（每键 keys/vals 各一次 push）。
  结论：数值/控制流零堆开销；容器按操作次数线性分配——**分配逃逸画像
  首次量化**，作为 B5-M32（深拷贝基线+move 消除）的基线数据。
- **B 家族实证（本轮继续定位）**：captest 编译暴露**捕获闭包调用点未
  补传隐藏参数**（`addn(5)` vs 提升签名 `addn(n, x)`）——Reference §11.1
  "原生发射已落地"表述过于乐观，实际口径修正为：隐藏参数提升已实现，
  调用点补传未实现（A2 的准确边界）。print 的未知类型数值表达式发 %s
  导致段错误 → 新增 **expr_num_hint**（语法级类型提示：EInt/EIdent 变量表/
  ECall 返回表/EField len/EIndex Vec/Map/EBin 算术比较递归/EUnary），
  print 与 f-string 选格式共用——**表达式级类型表的第一个实用切片**。
- **D 卫生**：3 个死代码警告（FnvHasher/module_items/expr_span2）
  标注 allow 清零；验证产物 host_out/self_out/fixpoint_diff 保留作记录。
- **全量验证**：cargo 173 全绿；7 套件 45 用例全过；宿主 29 用例 +
  自宿主 29 用例全过；fixpoint 12383 行仅宿主运行器尾行口径差；
  zig cc 零错误；aine build/idxtest、/tmp/mapidx2 双路径输出一致。
- **A5 fn 类型返回值（本轮推进至 80%）**：typedef 方案落地——fltype 新增
  fn_type_id（规范化 typedef 名）+ c_type 命中 fn( 分支；flee 为签名中的
  fn( 返回类型生成 typedef；SDef 签名与 let 绑定推断已用上（get_op 的
  签名/op 绑定均发为 C 函数指针）。**剩最后一环**：flcollect 的变量类型
  收集器未传播 fn 类型（op 被记为 int）→ hint 链断 → printf 仍 %s 段错误。
  解释器路径完整支持（含 call-of-call get_op(false)(20,2)）。
  另登记 flparse 缺口：单行多语句块（a; b; continue）解析失败 → 多行形式
  规避（同族已知限制）。
- **遗留口径（不变）**：A1 C 侧字符切换（待 M6-TCK 类型表）、A2 捕获
  闭包调用点补传（本轮定位到精确缺口）、A4 fstr B5-M33、A5 fn 返回值
  （typedef 方案未实施）、A6 命名空间（设计稿排期）、B2/B3（match 块臂
  尾值、无类型绑定插值——同属类型表家族）。

## G2.0-②：@ 属性统一 — 直接切换完成 ✅（本轮）

- **决策修正**：属性形态从「兼容期并存」改为**直接切换**——Aine 预发布期
  无外部用户，唯一存量是自身代码库，直接改 + 全库重写更干净（兼容期
  策略是给有存量用户的语言的）。其余 G2.0 项（var/module 三元组/闭包/
  match 臂）同此口径执行。
- **变更**：parser.rs 属性唯一形态  / 
  （parse_at_attribute； 分支移除，trait 方法处同步）；
  全库 8 个 Aine 文件  → ；Reference 示例同步。
  src/*.rs 内的  是 Rust 自身语法（cargo 层），不属 Aine，保持。
- **验证**：cargo 173 全绿；4 套件全过（含 @/@ 形态混跑回归）；
  宿主 29 用例不变。

## G2.0-①③⑥ + A5 闭环：var / module·export / return 续读 / fn 类型传播 — 完成 ✅（本轮）

- **③ module·export（直接切换收尾）**：全库 15 个 Aine 文件 `mod`→`module`；
  token 表移除 `pub`/`use`/`mod` 映射、新增 `export`；parser `KwExport`
  同 pub；文本内联器双形态补丁（`module x;` 曾致 29 用例归零，抓修）。
- **① var（最大迁移项）**：KwVar 关键词 + parser 双入口
  （parse_let_stmt=let/parse_let_stmt_mut=var，共享 finish_let）；
  fmt 双路径输出 var；flparse `var` 分发 + `let mut` 接受移除；
  UI `@state var`、全局 `@global var|let` 分发（at_item_decorator 前瞻
  扩展到 KwVar）；全库 559 处 + Rust 测试源内嵌 Aine 片段（转义串
  正则按缩进替换，Rust 自身语句不受影响）迁移。教训记录：
  `@global var` 入口检查只认 let → 两个测试失败，重构为 var/let 双入口。
- **⑥ return 续读（双侧）**：Rust parser 新增 starts_return_value
  （表达式起始 token 白名单）；flparse 补语句关键字排除（原激进续读
  会吞下一语句）。实测 `return
42` → 42。
- **A5 闭环（最后一环）**：fn_type_id 改显式字符集映射（is_ident_continue
  收 char 与 String 切片在 C 侧不匹配）；expr_num_hint 补 fn 类型变量调用
  与高阶调用链（get_op(0)(...)）两个臂；c_print_stmt hint 移到形状
  match 之后覆盖兜底（%zu 除外）。**fnret 端到端 22/40、EXIT=0**——
  fn 返回值 + 函数指针调用链在编译路径完整工作。
- **验证**：cargo 173 全绿；7 套件 45 用例全过；宿主+自宿主各 29 用例；
  fixpoint 12545 行仅尾行口径差；Grammar 产生式同步
  （module/export/var/@ 已入正文，G2.0 表 ①②③ 标已实施 ✓）。
- **遗留**：④ 闭包 `x =>`（与尾部块联动重设计）、⑤ match 块式
  （★★★★☆ 缓行）、trait→interface（择机）、B1/B2/B3 深修（类型表家族）、
  A2 捕获闭包调用点补传、_fix_*.py 与 probe_lex.aine 清理。

## A2 闭环：捕获闭包调用点补传 — 完成 ✅（本轮）

- **缺口定位**：rewrite_lift_expr 的注释声称"顶层调用点已在
  rewrite_program_calls 的 ECall 臂补传"——该臂**不存在**，lift 后的
  `addn(int n, int x)` 调用点仍发 `addn(5)`（隐藏参数缺失）。
- **修复**：rewrite_lift_expr 的 ECall found 分支补传——按 lift_caps_txt
  把捕获变量名展开为 EIdent 前插实参（`addn(5)` → `addn(n, 5)`）；
  f-string 路径的注入机制不变。
- **附带登记（B5 已知限制家族新成员）**：变体构造直接作 push 实参 →
  C 发射 `&右值` 编译错误 → 源层规避（先 var 绑定再 push）。
- **验证**：captest 编译端到端 `15 / hi!! / mk=107 base_still=100`、
  EXIT=0（与解释器逐字一致）——捕获闭包在原生路径首次完整工作；
  cargo 173 全绿；宿主+自宿主各 29 用例；fixpoint 尾行口径差。
- **口径修正**：Reference §11.1 恢复"双侧已支持"表述（此前因缺口
  改为"调用点补传未实现"，本轮闭环后更新）；补 D14 捕获即快照引用。
- **卫生**：14 个 _fix_*.py 临时脚本与 probe_lex.aine 经回收站清理。
- **G2.0 ④ 设计注记入 Grammar**：`x =>` 与 match 臂 `=>` 消歧靠位置，
  建议与 ⑤ 同批落地或 ⑤ 先行；`(x: i32) => ...` 注解形态与零参
  `() => ...` 保留。

## 自举路线图状态

B0 ✅ → B1 ✅ → B2 ✅ → B3 ✅ → B4 ✅（parser_stmt.aine）→ B5：
M1-M12 ✅ → M13 ✅ → M14 ✅ → M15 ✅ → M16 ✅ → M17 ✅ → M18 ✅ → M19 ✅ → M20 ✅ → M21 ✅ → M22 ✅ → M23 ✅ → M24 ✅ → **M25 ✅（本轮）** →
B5 收尾：Aine 编译器（transpiler.aine 编译为原生）已能编译真实 Aine 程序
（fib/hello/strings/records/mapsum/todo/json/mapnest）与前端组件
（lexer/parser_expr/parser_stmt）为可运行 C，输出与解释器一致；
fixpoint diff=0（25 用例）。剩余缺口：闭包独立值、import/module、
f-string 方法链插值。
下一步：闭包独立值 / import/module（自举刚需，拆分上帝文件的前提）/
进入 M6 交付物（stdlib 先行）。
## 自举 fixpoint 达成（本轮）✅

**host 29 例 = self 29 例,输出逐行一致**(仅剩 host 解释器打印 main 返回值 `Ok(())` 的环境差异,self 版返回退出码)。

### case21: char→String 实参转换修复
- 根因:valueflow_lite 把 `let s = "a5"`(字符串累加器)改写为 `fl_strdup_lit("a5")`,而 `var_info_of_stmts` 的 ECall 类型推断不认识该内建 → 变量类型掉到 `infer` → `is_char_typed(s[0])` 判 false → `fl_char_to_str` 转换缺失。
- 修复:fltype.aine `var_info_of_stmts` ECall 分支把 `fl_strdup_lit` 与 `read_file` 并列识别为 String。

### B2 家族漏网之鱼:值臂缺 return(解释器正常、编译执行出错)
块式 match 臂迁移后,凡 `Pat { expr }` 形式的值臂(无显式 return)在解释执行下靠尾表达式返回,但 C 发射只认显式 return → 编译后的编译器返回未定义值(垃圾指针/`\x01`/空),自举时逐点暴露。本轮修复 14 处:
- fltype.aine:`pat_bind_path`(303/314)、`enum_field_ty`(362)、`pat_literal`(334)、`block_tail_expr`(SExpr/SReturn)、`base_type`(EIdent/ECall/EField 载荷)、`expr_ctype`(Some 载荷×2)、`expr_ctype` EIndex Vec 分支、`some_member`(EIdent/EField 载荷)、`method_ret` 调用臂
- flparse.aine:`expr_ident_name` EIdent 臂、`rewrite_lift_expr` 兜底臂
- flstr.aine:`stringify`(EInt/EFloat/ERange/EIndex/ETry/EField/arm_body_text SExpr/stringify_stmt SExpr)
- flee.aine:`EUnary`、`ETry`、`ERange` 发射臂
- flee.aine `fstr_c`:以 `{` 开头的单插值 f-string(`f"{cur}"`)会先 push 空 `"lit:"` part → 整体 return "";改为空字面量不 push

### 调试方法(避免再犯批量改)
逐例增量:每次只定位一个 host/self 分歧点,修复后立即重跑完整 fixpoint(host 29 → 提取 SELF-BEGIN(用 rfind,块内含 SELF-END 字面量)→ zig cc → self 29 → diff)。self 崩溃用 `write_file` 落盘探针定位(避开 stdout 缓冲截断),host 自建 7 个 closuretest 单元二分定位 desugar 崩溃点。

## 全库改名 Flow → Aine(本轮)✅

**名称体系**:语言名 Aine、源码后缀 `.aine`、CLI 命令 `aine`、GitHub 仓库 `aine-lang`、对外全称标识 Aine-Lang。

**替换范围(全部完成,含内部标识符统一)**:
- 对外:Flow→Aine、flowc→aine、.flow→.aine、flow.toml→aine.toml(文档/代码/字符串)
- C 运行时前缀:fl_* → al_*(al_vec/al_push/al_strcat/al_opt 等 51 个标识符)
- 值流趟:valueflow→valueal(含 valueflow.rs→valueal.rs)、vf_*→va_*、VFScan→VAScan、VFLiteResult→VALiteResult
- 六模块:fllex/allex、flparse/alparse、flstr/alstr、flcollect/alcollect、fltype/altype、flee/alee(文件名+module 声明)
- 环境变量 FLOW_ALLOC_STATS→AINE_ALLOC_STATS、测试数据字符串 "flow"→"aine"、测试名同步
- 保留:通用英文词 overflow(溢出)/control flow(控制流)/flowing 等

**验证**:cargo test 171 全绿;host 29 例全绿;自举 fixpoint 重新达成(self 29 例 = host 29 例,输出逐行一致)——改名零语义破坏。

## 自举 B5-M33(第一项):match 尾构造转译器层修复 — 完成 ✅(本轮)

- **背景**:B5-M31 以源层规避(rewrite_lift_calls 语句式改写)掩盖了转译器缺口:
  函数尾 match 的臂体 SBlock 块尾为值表达式(构造调用/拼接)时被当语句发射,丢返回值。
- **修复(转译器层)**:c_stmt 增加 `tail: bool` 参数(14 个递归调用点传 false);
  SDef 分支函数体最后一条为 SMatch 且非 main 时传 true;SMatch 臂体发射在
  tail 模式下检测 SBlock 块尾值表达式(非 break/continue/print/方法调用),
  前段语句正常发射 + 尾值发射为 `return c_expr(...)`。
- **验证**:host 29 例全绿;自举 fixpoint 重新达成(self 29 = host 29,输出逐行一致);
  cargo test 171 全绿;端到端(describe: Some 臂块尾 `s + x` 拼接)解释器与 C 输出一致。
- **保留**:rewrite_lift_calls 语句式改写保留(稳定防御,移除需另轮回归)。
- **连带发现(已登记,非本次)**:整数字面量 match 模式 C 发射缺口
  (pat_name 对数字返回空 → `n.tag == ` 空)、if-expr 类型推断在无载荷变体臂
  的缺口(裸变体名需类型上下文)、嵌套 Option 构造实参发射缺口。

## 自举 B5-M33(第二项):整数字面量 match 模式 + EField 单索引类型推断 — 完成 ✅(本轮)

- **整数字面量模式**:`match n { 0 => ... }` 此前 C 发射破损
  (`first_ident("0")` 返回空 → `n.tag == ` 空比较)。修复:alee.aine
  EMatch cond 构造增加数字模式判别(首字符为数字或 `-`)→ 值比较
  `scrut == 0`。端到端:classify/fizz 解释器与 C 输出一致。
- **EField 单索引类型推断缺口**(自举才暴露):`arm.pat[0]`(String 字段
  单索引)被推断为 String 而非 char——EIdent base 分支推 char,EField
  base 分支却推了字段类型本身。导致 `p0 != "-"` 的 char 比较在自举
  产物中发射为 strcmp。修复:altype.aine EIndex-EField 分支
  `else { types.push("char") }`(与 EIdent 分支一致)。
- **验证**:host 29 例全绿;fixpoint 重新达成(self 29 = host 29);
  整数字面量场景 host 解释/host build/self build 三路输出一致;
  cargo test 171 全绿。
- **连带登记**:负数字面量模式(`-1 =>`)解析不支持(parse_match 期望
  `=>` 前为标识符),留待后续。

## 自举 B5-M33(第三项):负数字面量 match 模式 — 完成 ✅(本轮)

- **缺口**:`match n { -1 => ... }` 解析失败(期望 '=>' 遇到 '-')。
  Rust parse_pattern 对 Minus token 无分支 → 模式解析回退 Wildcard。
- **修复**:parser.rs parse_pattern 增加 Minus 分支——`-` 后跟 Int 时
  解析为 `Pattern::Lit(Expr::Int(-v))`(负数字面量模式)。
- **验证**:host 解释 / host build / self build 三路输出一致
  (sign(-1)→neg-one 等);Flow 自写解析器路径(经 self_host.exe)
  正确收集 pat="-1" 并发射 `n == -1`;host 29 例 + fixpoint +
  cargo 171 全绿。

## 回归固化:transpiler.aine 29 → 31 用例 ✅(本轮)

- **case31 lit-pattern**:整数字面量模式(0/1)与负数字面量模式(-1)发射值比较
  (`(n == -1)` 等)——固化 B5-M33-2/3,防退化。
- **case32 tail-match**:函数尾 match 臂体块尾值发射
  (`return al_strcat(s, x);` / `return "none";`)——固化 B5-M33-1。
- 验证:host 31 例全绿;fixpoint 重新达成(self 31 = host 31,逐行一致);
  cargo 171 全绿。

## M6-FMT-2:fmt_tool 输出对齐 aine fmt(diff=0)— 完成 ✅(本轮)

- **fmt_tool.aine 重写**:实现规范格式化(fmt_expr/fmt_stmt/fmt_if/arm_fmt/
  ty_fmt),与 Rust Formatter 对齐:4 空格缩进、最小括号(优先级表 binop_prec)、
  if/while/for 条件无括号、match 臂 `pat => value` 块式臂体、结构体字面量
  多行、类型文本规范化(泛型/函数类型逗号空格、箭头两侧空格)、支持
  cli_args 文件参数。
- **修复 fmt_tool 陈旧定义**:Stmt 补 SAssignIdx(与 transpiler.aine 对齐)。
- **连带修复 alparse(自举解析器)**:
  1. 结构体字面量字段支持逗号分隔(与 Rust parser 对齐,幂等格式化
     输出逗号字段可重解析);
  2. 结构体字面量 `{` 后换行(TSemi)跳过——数组内多行结构体字面量
     looks_like_struct 检测修复。
- **验证**:10 文件(fib/records/mapnest/strings/mapsum/hello/closuretest/
  stdtest/idxtest/hotest)fmt_tool 输出与 `aine fmt` **diff=0**(仅尾换行
  输出层差异)且幂等;host 31 例 + fixpoint + cargo 171 全绿。

## 自举 B5-M33(最后一项):fstr AST 插值路径启用 — 完成 ✅(本轮)

- **背景**:f-string 插值此前为文本启发式(fstr_piece 处理 .len/调用/字段/
  索引/纯变量),方法链/嵌套调用插值(`f"sum={nums.iter().map(x => x).sum()}"`)
  不支持;AST 路径 WIP 自 M28 登记段错误(崩于 c_expr ECall(EField(len)),
  归因 C 后端生存期)。
- **启用**:fstr_piece 开头加 AST 路径——整段 tokenize+parse_expr 成功且
  消费到 TEof 时走完整 c_expr,按 expr_ctype 包装(al_num/al_fnum/
  al_char_to_str),bool 按 base_type 特判 al_bool;失败回退文本版。
- **段错误根因结论**:M29 值语义(深拷贝基线)已修复底层生存期问题,
  重新启用后自宿主零段错误(此前未重新验证)。
- **连带修复(expr_num_hint EIndex 分支 B2 家族)**:`Vec<`/`Map<` 分支
  缺 return → `print(v[0]+v[1]+v[2])` 发射 `printf("%s\n", int)`
  → 原生段错误。补 return 后 idxtest 端到端恢复。
- **验证**:host 31 例全绿;fixpoint 重新达成;9 程序
  (fib/hello/strings/records/mapsum/closuretest/stdtest/idxtest/hotest)
  端到端解释器 vs 原生一致(含方法链插值 sum=6);cargo 171 全绿。

## M2 值流核心补全:Escape 显式分析 + 摘要显示 — 完成 ✅(本轮)

- **评估结论**:M2 分析器(valueal.rs)已覆盖 Owned/Copy/Move/Projection/
  Consume/Borrow/Capture/Materialize、VIR、Summary+组合(call_val substitute)、
  预算(depth cap 降级)、Optimization Explain(opt 命令);缺口为
  **Escape 显式分析**(DoD 概念)。
- **实施**:
  1. FnSummary 加 `escapes: Vec<usize>`(逃逸参数索引);
  2. 逃逸判定:返回 prov 精确为 Input(i)(本体返回,所有权转移/存储);
     投影返回(Field/Element)为借用返回——调用点已物化返回视图,
     参数本身不逃逸(收窄避免与既有 materialization 计数重复);
     任务捕获逃逸已由 consumed 机制覆盖;
  3. call_val 接入:被调方逃逸参数 → 调用点视图物化 + F2003 ESCAPE
     提示(Optimization Explain);
  4. 摘要显示 `[esc:i]`(vf 输出:`pass (u) -> Input(0) [esc:0]`)。
- **测试**:+2(escape_detected_on_returning_param、
  no_escape_materialization_for_copy_types);cargo 173 全绿;
  host 31 例 + fixpoint 全绿。
- **M2 DoD 对照**:九个概念全实现;VIR ✓;Summary+组合+预算 ✓;
  Optimization Explain ✓(F2001 materialization / F2002 budget /
  F2003 escape);跨函数组合测试(composition_propagates_field、
  materialization_hint_on_consuming_call)✓;精度降级一致(budget
  cap → Unknown)✓。

## M3 大对象验证:运行时实测补全 — 完成 ✅(本轮)

- **背景**:M3 报告原为分析层验证(§74 指标 synthetic 基准),运行时实测
  标注"待 codegen"——codegen 现已就绪。
- **实测**:examples/bigstr.aine(分层 String 构建 + Vec 构建遍历),
  `aine build` → zig cc → 原生运行 + AINE_ALLOC_STATS 真实分配计数。
  10MB(str_len=10485760, 100 万 Vec, 165 万次分配, ~5 秒);
  100MB(str_len=104857600, 1000 万 Vec, 1655 万次分配, ~2 分钟)。
- **连带修复(cli_args 内建类型推断 3 处)**:var_info EIdent(cli_args)→
  Vec<String>、EIndex cli_args[i]→String、c_stmt SLet EIndex cli_args→
  char*、expr_is_string cli_args[i]→String(此前 `cli_args[0] == "100"`
  错发指针比较)。al_strcat_own 指数扩容(与 strlen O(n²) 无关,分层
  构建绕行)。
- **已知限制登记**:al_strcat_own 每次 strlen → 循环拼接 O(n²);
  String 表示重构(带 len/cap)列为运行时优化候选,分层构建可绕行。
- **验证**:host 31 + fixpoint + cargo 173 全绿;M3 报告更新
  (分析层 + 运行时双通道)。

## M4 补全:trait 约束 / FFI / flowpkg — 完成 ✅(本轮)

### trait(interface 约束 + 转译器 impl 方法)
- **Flow 转译器**(此前完全缺 impl):alparse 支持 `implement I for T { }`、
  self 无类型形参、interface 声明(占位跳过)、extern 无体声明;alee
  发射 impl 方法(Type_method + self 绑定 struct T + 前向声明 + 调用
  兜底 `T_mname(base, args)`);alcollect 4 个收集器含 impl 方法
  (参数进变量表,self 类型填 impl 类型)。
- **Rust typeck**:resolve 把 Item::Impl 编码进 Mod.name(`impl|I|T`),
  typeck 建 impls 表(interface→实现类型),参数检查
  assignable_via_interface——实现类型可传 interface 参数,未实现报
  T3005(负例验证);resolve_bodies Fn lookup 容错(方法名非顶层符号)。
- 端到端:interface+implement+方法调用,解释器与原生输出一致。

### FFI(extern fn)
- Rust parser:extern 声明允许无函数体;解释器调用 extern 报清晰错误
  ("仅原生运行时支持");转译器:extern → 空体 SDef → 仅前向声明
  (定义循环跳过空体);C 符号名即函数名,外部实现链接。
- 端到端:extern al_ext_atoi + 外部 C 实现 → 原生运行输出 124。
- 注意:extern 声明 C 库函数(atoi/strlen)会因 const 签名冲突——
  FFI 面向自定义符号。

### flowpkg(ainc.toml 工具)
- pkg_tool.aine 修复:Stmt 补 SAssignIdx(旧版定义不同步)、main 签名
  检查串更新;端到端:解析 aine.toml → 生成 C → build OK。
- 验证:host 31 + fixpoint + cargo 173 全绿。

## M5 并发 + UI 联合 — 完成 ✅(本轮)

按 Aine 原则(Concurrency Guide §1/§3、UI Guide §4/§5、登记簿 D3/D4/D8)实施:

### 转译器 go/ui(此前完全缺)
- alparse:go{}/go!{}/ui{} 语句解析 → SBlock 降级(结构化任务语义,
  转译器层同步执行块,与解释器一致;所有权/捕获检查在 Rust 侧
  valueal;真实并发为运行时实现细节,登记)。
- 端到端:go 块内赋值/print、ui 块内写 state,解释器与原生输出一致。

### UI 规则检查(valueal,UI Guide §4.2/§5.3)
- **ui{} 内禁 go{}/go!{}** → V4004("请在 ui{} 外部创建后台任务");
- **@global 写入只能在 ui{} 内** → V4005(global_symbols 收集自
  HirItem::Global)。

### Send/Sync 自动推断(Concurrency Guide §3.2/§3.3)
- 字段级组合推断:struct 全字段 Send 则 Send;容器按类型参数;
  标量/String/闭包快照 Send;
- **@not_send** 显式标记(贯穿 parser→resolve→HIR→typeck→valueal),
  嵌套字段传播;
- 跨任务捕获非 Send → V4006 业务语言诊断
  ("不能安全发送到后台任务;原因;建议")。
- +4 测试(go_inside_ui/global_write/not_send/send_ok);
  cargo 177 全绿;host 31 + fixpoint 全绿;go/ui 端到端一致。

### 说明
- 运行时真实并发(CreateThread/调度器)与 Future/Task 类型、UI 事件
  回调 `?` 传播:解释器/转译器同步语义已就位,真实异步执行列运行时
  里程碑(登记,不偏离原则:结构化任务与所有权边界是语义,执行是细节)。

## M6 MVP:记账本静态验收 + §72 指标 + Build/Publish — 完成 ✅(本轮)

- **记账本端到端(静态验收)**:account_book.aine check 0 警告、vf 0 错误。
  转译器支持 `ui Name { }` 组件定义(结构感知跳过,声明式体;UI 运行时
  M7 发射可执行代码,登记);main 的 db.init/run_ui 调用链路发射。
- **连带修复**:c_type 缺 i64 映射(→ void* 错)→ 补 `long long`。
- **§72 联合验收报告**:vf 命令输出指标
  (dangling reference / data race / illegal state access / ownership
  transfer / unnecessary full copy)。account_book:0/0/0/0/0 全达标。
- **Build/Publish**:tools/package.sh —— single(自包含 aine.exe)与
  portable(exe+examples+stdlib+conformance+日志)两种发布物;
  验证版本与运行。
- **验证**:host 31 + fixpoint + cargo 177 全绿。
- **MVP 剩余登记**:SQLite/HTTP 模块(C 内建扩展,MVP 后置)、
  UI 运行时窗口/控件(M7)、真实并发调度(M7 运行时)、Book 文档。

## M7 完整 UI 与工具链:LSP / Profiler / Debugger 基础 — 完成 ✅(本轮)

按 M7 DoD(§5 DoD 5/6/7)实施,全部端到端验证:

### IDE(LSP,DoD 5 基础)
- `aine lsp`:JSON-RPC over stdio(vscode/neovim 兼容)
- initialize(能力声明:诊断/悬停/补全/跳转)、didOpen/didChange →
  **全流水线诊断推送**(lex/parse/resolve/typeck/valueal 五段)、
  hover(函数摘要 `fib (n) -> Unknown` / 绑定类型 `x: i32`)、
  completion(关键词+符号)、definition(符号跳转)、shutdown/exit
- 修复:JSON 转义(\n 等)处理、hover 查 bindings(定义处)
- 端到端测试脚本验证全部能力

### Profiler(DoD 7)
- `aine profile <file>`:六区报告(CPU=五段流水线耗时 / Memory /
  Allocation / Borrow)+ Aine 专属(Materialization / Large Copy /
  Borrow 结果)+ 规模(tokens/函数/VIR)+ 诊断计数
- mapsum:0.28ms 全流水线,决策分布输出

### Debugger(DoD 6 基础)
- `aine debug <file> <行号...>`:断点执行——解释器命中断点时记录
  行号与局部变量快照,程序继续运行;输出断点报告 + 完整程序输出
- Interp 加 breakpoints/bp_hits/locs 行号表;value_short 摘要
- 端到端:断点 3/5 命中,变量快照正确(total=0 → a=5,total=5)

### 登记(后续)
- UI 运行时窗口/控件/动画/热重载(独立 UI 后端,1.0 完整)
- Debugger 交互式暂停/单步/异步栈;LSP Quick Fix/跳转引用完善
- 验证:host 31 + fixpoint + cargo 177 全绿

## 1.0 收尾:UI 运行时 + The Aine Book — 完成 ✅(本轮)

### UI 运行时(此前为 run_ui 桩)
- interp 组件定义表(ui_defs);run_ui 执行组件 render 块:
  - 控件调用(Text/Button/TextInput/List/Row/Column/ListItem)求值为
    缩进组件树(终端渲染);
  - @state 初始化与状态摘要(`[state] count=3, label="hello"`);
  - f-string 插值读 @state(render 内 Text(f"计数: {count}"));
  - **事件回调不自动执行**(尾块闭包仅作为树的孩子,副作用由事件
    驱动——不偏离 UI Guide §4 语义);
  - 组件名作值承载为名字字符串(run_ui(Counter))。
- 记账本端到端:真实渲染完整组件树 + 状态(records/total/输入)。
- 连带修复:render 块尾表达式(tail)渲染;expr_ctype EIndex 切片
  (s[a..b])类型(曾判 char → al_char_to_str 错,现 char*)。
- 更新 account_book_runs_with_ui_stubs 断言。

### The Aine Book(1-15 章,至 SQLite/HTTP)
- The_Aine_Book.md:安装/工具链、你好 Aine、基本语法、类型系统、
  字符串、集合、结构体与枚举、模式匹配、错误处理、模块与包、
  闭包、所有权、并发、UI、工具链与下一步。
- 示例全部来自真实代码并**抽验**(14 项输出:fib=55/area=12.56/
  got 5/closure=42/sum=6/task-ok 等,解释器=原生)。

### 验证
host 31 + fixpoint + cargo 177 全绿。

## 1.0 收尾续:db 模块 + Book 22 章 + Cookbook — 完成 ✅(本轮)

- **db 模块(标准库 9/11 方向)**:解释器内建,文件 JSONL 存储——
  init/insert(结构体序列化 JSON)/query(SQL 子集 SELECT/FROM/ORDER BY
  DESC,行 → Map<String,String>)/delete(按 id)。端到端:insert 2 行、
  排序查询、删除后 1 行。
- **记账本适配**:db.query 行 → Record 显式映射(rows.get()→字段);
  check 0 警告,渲染正常。
- **Book 16-22 章**:高级主题(视图/move)/发布打包/数据库/HTTP 预告/
  编译器内部/规范速览/生态下一步。
- **Cookbook**:12 个可运行示例(Hello/斐波那契/字符串/集合/Map 计数/
  枚举/Option/错误传播/闭包/模块/并发/数据库)。
- 验证:host 31 + fixpoint + cargo 177 全绿。

## M8 可做项三件:Build/Publish 全模式 + HTTP 模块 + Std Lib Reference — 完成 ✅(本轮)

- **Build/Publish**:tools/package.sh 扩展 bundle 模式(exe+src+stdlib+
  examples+文档+工具+verify.bat 冒烟脚本)+ **签名预留接口**
  (SIGNATURE.txt:signtool 流程 + SHA256 指纹 + certutil 校验)。
- **HTTP 模块**:解释器内建 `http.get(url)`(std::net,HTTP/1.0 连接
  关闭即响应尾);本地 python http.server 端到端验证(200 返回 body、
  404 返回 Err);`http` 注册进 BUILTIN_GLOBALS。
- **Std Lib Reference**:tools/gen_stdlib_ref.py 从 stdlib/*.aine 提取
  43 个 API 签名,生成 12 项属性参考(用途/签名/参数/返回/示例/错误/
  性能/线程安全/C 映射/所有权/注意事项/版本);db/http 内建手工补全。
- 验证:host 31 + fixpoint + cargo 177 全绿。

## 全 Aine 迁移 Phase 2:typeck 分期迁移(检查器 v1)— 完成 ✅(本轮)

- **altypeck.aine**(约 260 行 Aine 类型检查器):未定义函数/实参个数/
  结构体字段存在/结构体字面量字段 四类高频检查;定位为"函数名#语句
  序号"(AST 无 span,粒度登记改进)。
- **接入**:transpiler build 模式在转译前运行 tck_program(flatten 后
  程序,含模块展开),诊断输出 `type-check: ...`,有错即失败;
  **Rust front_pipeline 加 with_typeck 开关——build 跳过 Rust typeck,
  类型检查由 Aine 承担**(run/test 保留完整 Rust 检查)。
- **误报修复三轮**:模块未展开(用 flatten 后)、let 闭包名视为已知函数、
  fn 类型参数逗号(括号感知 split)、fn 类型参数调用(参数名表跳过)。
- **验证**:31 例全绿 + 9 程序端到端一致 + 坏程序拦截(参数个数/字段
  两处)+ fixpoint + cargo 177。

## 全 Aine 迁移 Phase 2 扩展:未定义变量检查 — 完成 ✅(本轮)

- **altypeck 增加 EIdent 未定义变量检查**:保守名单(参数/内建/函数/
  全程序变量表/枚举变体/结构体/枚举类型名),0 误报。
- **修复**:tck_expr 内部递归调用(ECall 实参/EBin 操作数/EStruct 字段/
  EMatch scrut 等)漏传 params 导致参数错位(vnames 位置收到空)——批量
  补全;debug 打印全部移除。
- 注:未定义变量与 Rust resolve 的 N3001 重叠(resolve 先拦截),检查器
  为 Aine 侧独立实现,resolve 迁移时直接可用。
- 验证:31 例 + 9 程序 build + fixpoint + cargo 177 全绿。

## 全 Aine 迁移 Phase 2 扩展 + Phase 3 + Phase 4 — 完成 ✅(本轮)

### Phase 2 扩展:返回类型/运算类型检查
- altypeck 增加:返回类型不匹配(字面量形态 vs 函数返回类型,
  保守)、算术运算符不支持字符串、字符串不能与数字相加;
  freturns 表贯穿 tck 链;修复空参数 fptypes 分割误报。
- 验证:31+9 全绿;坏程序拦截(返回 "hello" 给 i32、s = "a" + 5)。

### Phase 3:valueal 迁移(Aine 所有权检查器)
- **SGo/SUi AST 变体**(transpiler/alstr/fmt_tool/pkg_tool 同步;
  alparse 解析;alee 发射同块)——go/ui 标记不再丢失。
- **alvalueal.aine**(约 150 行):go{} 捕获的复合类型变量
  (Vec/Map),之后在函数内再次使用 → "已移入后台任务,不能再使用"。
- 接入 build;31+9 零误报;捕获后使用报错、仅捕获不报。

### Phase 4:解释器自举(alinterp)
- **alinterp.aine**(约 380 行 Aine 写的最小解释器):fn/let/var/if/
  while/for 区间/return/print/算术/比较/字符串拼接/f-string 插值
  (变量与函数调用)/用户函数调用(按名查找+递归)/尾表达式返回。
- 连带:**宿主补 Option/Result 方法**(unwrap_or/unwrap/is_some——
  call_method 此前无 Option 分支,记账本回调实际执行会用到)。
- **自举验证:alinterp 解释执行 fib.aine 与 hello.aine,输出与
  aine run 逐行一致**;strings 等含方法调用的程序为已知限制
  (最小解释器范围,登记)。
- 修复:char 索引/数值比较(宿主 String 值比较限制)、SIf 分支缺失
  (无限递归)、尾表达式 print 重复。

### 验证
host 31 + fixpoint + cargo 177 + 9 程序 build 全绿。


## alinterp 全量迁移闭环:解释执行 transpiler.aine 与宿主逐字节一致 ✅(本轮)

### 目标与达成
- alinterp.aine 由最小解释器扩为全量解释器:解释执行 transpiler.aine
  (case1-32:解析/类型收集/值语义/C 发射/自转译),输出与
  `aine run transpiler.aine` **15339 行逐字节一致**(diff 0,
  含 NUL 字节行)。每轮全量验证 ~50 分钟(纯解释双倍开销)。
- 修复链(按定位顺序):
  1. SWhile/SFor break/continue 前保留循环内 env 更新(参数循环
     break 丢变量,解析错乱);
  2. EIf/EBlock 表达式返回块尾值;ai_call 错误吞噬(4 调用点补
     ctrl=="error" 传播);SLet ? 检查;pat_match AOption/AResult;
  3. **EStr f-string 内容误判**:求值期 `len() >= 2` 检测把普通
     字面量 `"f\""`(解码内容恰为 f",长度 2)当 f-string 插值成空串
     ——与 C 发射器 alee.aine 的 `>= 3` 不一致,统一为 >= 3
     (alinterp/alstr/alparse 三处);
  4. alparse/alstr 增加转义解码(al_unescape_body,与宿主 lexer
     decode 一致;宿主在 Rust 解析期解码,alinterp 解析文件文本
     必须自行解码)——普通串解码;f-string 保留 f" 前缀;
  5. **Some("i32") 载荷字面量模式**:ai_pat_match 只比对构造器名,
     不校验载荷值 → vt=Some(String) 误中 Some("i32") 分支
     (print %s→%d、al_num 缺失等全部类型分类偏差的单一根因);
  6. **EMatch/SMatch 臂体尾值**:语句形 match 作函数尾时取命中臂
     块尾值(分类器 expr_ctype 等大量依赖),载荷绑定提取为
     ai_arm_bind 共用;ai_tail 增 SMatch;
  7. **嵌套模式括号深度**:ai_pat_payload 在首个 ")" 停止,
     SReturn(Some(e)) 提取成 "Some(e" → 补深度跟踪;
     ai_pat_match/ai_arm_bind 支持嵌套 Some/Ok/Err 载荷校验与绑定;
  8. main 返回值打印(宿主打印 Ok(()),alinterp 对齐);
     移除启动 ALP 打印(逐行一致要求)。
- 转义/flatten 探针等全部调试 trace 清理。
- 保留 mini1-8/tpdrv 回归;新增 mini24/25/26 快速双引擎等价验证
  (2s:类型分类/match 尾值/嵌套模式);mini9/mini13 已回收。
- 验证:cargo test 全绿;宿主 transpiler 输出与参照一致;
  alinterp 解释 transpiler 逐字节一致(15339 行);mini 套件全过。

## 2026-09-02/04 Studio 自绘壳 v0(视觉对齐 mockup)

- egui 画布参照实现(src/bin/aine_studio.rs):Painter 直绘五区布局
  (顶栏/rail/左栏/编辑器/Assistant/Problems/状态栏),主题常量逐条
  对应 mockup CSS;token 级语法高亮(状态机:标点/单词/字符串/注释);
  文件点击切换;当前行高亮 + 光标。作为视觉基准(其后 Aine 原生壳
  逐像素对齐该方向)。
- alee C 模板:rt_* 自绘运行时扩展 ——
  1. al_comp 增 rad/size/mono 字段;AL_MAXC 128→2048;
  2. 新增 AL_SRECT(圆角描边)/AL_HLINE(1px 横线)渲染分支;
  3. D2D 宏调用补 strokeStyle 实参(MinGW 头为函数式宏,少参报错);
  4. 新原语:rt_begin/rt_fill/rt_rrect/rt_srect/rt_hline/rt_text
     (RGB int 0xRRGGBB + mono + size,al_tf 按 size 缓存 TextFormat)/
     rt_click(注册点击区)/rt_wait_click(事件循环取点击,控制流留在
     Aine 侧)/rt_invalidate;
  5. rt_window 直接 ShowWindow(原依赖 rt_run 显示,事件循环模式下
     窗口不可见);
  6. ui{} DSL 整数实参误包 atoi(320) → EStr 才包(ui_int_arg)。
- ide.aine 重写(~300 行,纯 Aine):D2D 上绘制完整 mockup 五区;
  Seg 结构 + line_segments 状态机做 token 级高亮;main 内 while +
  rt_wait_click 事件循环,点击文件行切换选中。
- 宿主链路配套:src/hir.rs BUILTIN_GLOBALS 增 rt_* 13 项;
  altypeck.aine 两处内建白名单同步;src/main.rs 链接补
  -ld2d1 -ldwrite -luuid -lole32。
- 发射器陷阱记录:
  1. Aine 整数类型名为 i32,`int` 不存在(误用 → C 端 void* 返回);
  2. `var x = <i32形参>` 无标注时 VALite 字符串累加改写误判
     (x = al_strcat_own(x, ...))——守卫只认显式 i32/f64/bool 标注,
     需写 `var x: i32 = ...`;
  3. 无返回类型函数的末语句为表达式时会生成 `return expr;`(void 函数
     返回值)→ 末尾补显式 `return`。
- 验证:cargo test 全绿(180+3+3);counter.aine(ui{} 旧路径)重建
  通过;mini25 双引擎一致;ide.aine → ide_gen.exe 编译运行,
  截图确认与 mockup 视觉一致。

## 2026-09-04 Studio 壳: 窗口自适应缩放

- 问题: draw_all 尺寸写死 1200x760, 最大化后仍画原布局(用户反馈
  "放大咋还是那块")。
- 修复: alee 模板新增 rt_w()/rt_h()(GetClientRect 实时客户区)、
  WM_SIZE 置 g_resize_flag + InvalidateRect、rt_wait_event()(点击
  返回区域索引 / 缩放返回 -1 / 退出返回 -2, 统一事件源)。
  ide.aine main 循环收到 -1 后回顶用新尺寸重画; W/H 改 rt_w()/rt_h()
  动态取值, 右侧/底部锚定元素全部随之重排。
- 验证: 最大化截图五区正确铺满; 合成点击第二文件行, 选中态切换
  (高亮条+绿点移动)。Tab 标题/代码内容仍为演示数据 — S1e-3 接
  compiler_service 后换真实文件。

## 2026-09-04 S1e-3a: IDE 接真实数据

- alee C 模板补 file_list(char* path) -> al_vec（FindFirstFileW +
  UTF-8 转换 + strcmp 排序, 与宿主语义一致）。上次会话的该内建
  随 alee.aine 还原丢失, 本次重加。
- 发射器: SLet 推断 file_list → al_vec; push(rvalue) 取址覆盖补
  EIndex 分支（String 切片/Vec<String> 索引 → &(char*){...} 复合
  字面量, 否则 int）。此前仅覆盖 ECall/EInt/EFloat/EStr/EBin。
- ide.aine: 文件树 = file_list("examples") 过滤 .aine（Aine 侧
  ends_with）, 点击行 → read_file 打开真文件, split_lines(Aine)
  按行渲染 + 逐 token 高亮, 超宽行按 max_x 裁剪不越权绘制
  Assistant 面板; Tab 标题/徽标/状态栏显示真实文件名。
- 验证: 编译运行截图 —— 列表真名排序、点击 alparse.aine 后编辑器
  显示其真实内容（含中文注释渲染）; counter 重建/cargo test
  180+3+3/mini25 全绿。

## 2026-09-04 S1e-2/3b: Scintilla 编辑控件接入 + Aine 容器样式

- vendor/scintilla 构建: 原 DLL 实为 ar 静态库(-shared 与 -static 冲突,
  zig 产出静态库); 去掉 -static 重建 → 真 PE。bin/Lexilla.dll 为旧
  有效构建。
- SETTEXT 在该 zig 构建中静默失效(ret=1 但文档不变; DirectFunction
  同样) → 绕过: CLEARALL+ADDTEXT(len,text), GETTEXT 读回, 探针验证
  28 字节往返无损。
- Lexilla ILexer5 路线放弃: lexer 挂载成功但从不产出样式; vtable 直调
  Version() 返回垃圾(虚表 thunk 缺陷, -O0/-O1 同样) → zig c++ 对该
  代码库的多继承/虚表 codegen 不可信。
- 转向容器化样式(官方协议): sci_new 里 SETILEXER5(4003,NULL) 显式
  容器模式; al_rt_proc 增 WM_NOTIFY → SCN_STYLENEEDED(2000) 分发 →
  g_style_cb(Aine 回调, rt_on_style 注册) → ide.aine 的 style_cb 用
  自己的 line_segments 逐 token STARTSTYLING(2032)/SETSTYLING(2033)
  写样式字节。关键坑: ADDTEXT 后文档被标记为已样式化到末尾, 通知的
  position 不可靠 → style_cb 忽略 pos, 每次从 0 全量重刷(通知频率
  低, 开销可忽略)。
- 消息号纠错(背错一堆, 以 include/Scintilla.iface 为准):
  SetCodePage=2037(非 2137), SetMarginTypeN=2240, SetMarginWidthN=2242
  (非 2241/2243), StyleSetFont=2056(非 2053, 2053 是 StyleSetBold —
  之前把字符串指针当 bool 发导致全员加粗), SetCaretLineVisible=2096,
  SetCaretLineBack=2098, SetSavePoint=2014。单参数 set 的实参走
  wParam(曾把 65001 发进 lParam → 全文乱码)。
- ide.aine 编辑闭环: 点击文件 → GETMODIFY(2159) 有改动才
  save_current(旧文件) → open_file 新文件 → sci_set_text +
  SETSAVEPOINT。sci_fit 随窗口自适应重摆。sci_get_text 增行尾归一
  (\r\n|\r → \n), 杜绝异常行尾落盘。
- 自动保存行尾损坏事故: 早期构建的自动保存把 account_book/alcollect/
  alparse 写成孤立 \r 行尾(全文一行), transpiler 因此报未定义名。
  已 git 恢复 + sci_get_text 归一化兜底。
- 验证: 中文注释正确渲染; 注释灰/关键字紫/类型绿/字符串橙全套生效;
  输入文本→切文件自动落盘(grep 验证)→内容往返一致; cargo test
  180+3+3 全绿; counter/mini25 通过。

## 2026-09-05 S1e-3c: Problems 面板接真实编译器诊断（rpc 直调语义）

- 架构选择: 不走子进程 JSON-RPC, 而是 IDE 内嵌编译器前端 —— ide.aine
  尾部 module 行展开 altype/altypeck/allex/alparse/alstr/alcollect,
  进程内直调 tokenize → parse_program_safe → tck_product 系（与
  compiler_service 的 workspace.diagnostics 同一函数链, 免 IPC 序列化）。
- 两个模块系统约定(首次踩明):
  1. 枚举变体不过模块边界 → 消费文件须本地定义用到的 enum/struct
     (alinterp/compiler_service 均如此)。ide.aine 头部放 alinterp 的
     完整 AST 定义块(TokKind/Token/Expr/Stmt/MatchArm/Field/
     StructField/EVariant/Param/EPResult/SPResult/BlockResult/TyResult/
     FlatResult/VarInfo);
  2. module 行放文件末尾(本地类型先注册; 置顶会与模块内定义冲突,
     报"未定义名称 TSemi"之类错位诊断 —— 展开是文本级内联, 诊断
     行号/路径标的是合并视图)。
- 行号映射: PErr.index 是 token 下标, 换行在词法层是 TSemi("\n"),
  数它得行号(tok_line)。
- ide.aine 新增: Diag{code,msg,line,sev}; compute_diags(解析错误
  E0001 + 行号, 无解析错时 tck_program 全量 W0401, 各限 60 条);
  compute_symbols/compute_types(collect_fn_names/collect_user_types);
  clip_to(按字符截断防越权绘制)。Problems 真实渲染计数/行/跳行
  (点击行 → GotoLine=2024 0基 + SetFocus=2380); SYMBOLS 真实
  fn/type 列表; 状态栏真实 "N error(s)" / "Check OK"。
- 发射器修复(push 复合字面量取址, 均为这次内嵌编译器暴露):
  1. push(枚举构造器调用): fn_return 未知 → variant_owner 取
     Owner → &(struct Owner){ ctor(...) }(Owner_Name() 是返回匿名
     struct 的普通函数, 裸 & 右值非法);
  2. push(Vec 元素 EIndex): 元素是 lvalue → 直接 &(此前误发
     &(int){...} 对 struct 元素非法);
  3. alparse 源配合: parse_program 的 @global 分支 push(SGlobalVar(...))
     改 let sv: Stmt 中转(构造器 rvalue 场景源码层规避, 与发射器
     双保险)。
- 验证: ide.aine(含 6 模块 ≈6k 行展开)编译通过, exe 232KB→511KB
  (内嵌前端); 截图确认 Problems 显示真实 tck 输出
  "main: 未定义变量 AccountBook"(ui{} 块类型不在 tck 词表, 符合
  预期)与真实计数; SYMBOLS 显示 account_book 的真实 main/Record;
  cargo test 180+3+3 全绿; mini25/counter/transpiler 自举
  (tp_check.exe)全部通过。

## 2026-09-05 S0.6 Language Rendering bootstrap — Language Veil (最核心语言面)

- 用户指正: 多自然语言表面(架构第 7 条/S0.6)是产品最核心差异点,
  此前被完全跳过。本轮补齐 bootstrap。
- alrender.aine(新增, 声明式 Language Profile):
  - zh_words/zh_maps(22 关键字对: 让/函数/返回/如果/否则/循环/遍历/
    于/匹配/中断/继续/结构/枚举/模块/导入/类型/实现/公开/作为/真/假/
    打印 ↔ canonical)+ zh_ops/zh_op_maps(14 运算符对: 加/减/乘/除/余/
    大于/小于/大于等于/小于等于/等于/不等于/与/或/非);
  - transform(text, zh) 双向逐 token 替换: 跳过字符串/注释, 最长 run
    收集(CJK 连续段), 行号/列不变(诊断坐标直通);
  - surface_to_canonical / render_zh / is_zh_kw。
- 验收(S0.6 判据, rtest.aine):
  1. round-trip zh→canonical→zh 逐字节一致(roundtrip OK);
  2. zh_demo.aine 与 en_demo.aine 双表面 → SAME CANONICAL AST
     (aine ast 输出去 span 后逐行一致, span 平移即 Source Map 职责);
  - 途中修: ASCII 运算符(+ > == 等是 run 终止字符)反向渲染漏映射 →
    非 run 分支加 2/1 字符运算符探测; "->" 加语法保护不翻译。
- ide.aine 接入:
  - @global g_zh(bool, 默认 zh-CN 用户导向); 顶栏 中/EN 切换按钮;
  - tr() 全 chrome 本地化(项目/符号/助手/问题/检查通过/错误/警告/
    提问框/助手文案/符号类别/命令提示), Tab 徽标显示表面语言;
  - 切换 = 缓冲区整体重渲染(render_zh ↔ surface_to_canonical) +
    重算诊断(canonical 化后进同一 parse/tck 管道) + SETSAVEPOINT
    (表面切换视为无损重渲染);
  - 语法着色 is_zh_kw 扩展(zh 关键字同关键字色);
  - 诊断/Symbols 管道改吃缓冲内容(canonical 化后)。
- rt_click 语义修复(隐藏多时的 bug): 注册序号当事件 id → 命中测试
  返回注册下标, EN 按钮(先注册=0)点击被文件行分支吞掉、Problems
  100+ 偏移假设从未成立。改 al_clk{id} + rt_click(...,id) +
  g_last_click = id; 调用点: EN=200, 文件行=10+fi, 问题行=1000+pi;
  main 分支同步。调试期加过 WM_LBUTTONDOWN 坐标日志(已移除)。
- 教训(再次): Python heredoc 写含 \n 的 Aine 字符串字面量会被吃
  转义 → 断字符串; 且旧 IDE 进程开着文件缓冲会覆盖磁盘编辑(先杀
  进程再改文件)。转义用 chr(92) 拼接最稳。
- 验证: cargo test 180+3 全绿; mini25/counter 通过; IDE 截图确认
  zh 默认 UI、EN 切换(chrome+缓冲整体重渲染)、zh 切回, 诊断在两种
  表面下一致。
- 待办(S0.6 完整验收剩余): canonicalizer 进编译器主管道(aine build
  直编译 zh 表面源码)、Source Map 双向坐标、Profile 外置化、
  更多表面(ja/de/…)、诊断消息的表面渲染。

## 2026-09-06 IDE 推倒重来: VSCode 布局重建

- 问题: 前几轮在烂代码上叠补丁, 导致文件树重叠/编辑器空白/布局破碎。
  用户反馈 "？？你看看现在是什么烂东西"。
- 方案: 推倒重来, 参照 VSCode 布局, 从零开始。
- 步骤1: 布局框架 (纯视觉, 981→216行)
  - 顶栏 (Title Bar): 深色背景 + Logo + 菜单项 + 语言下拉按钮
  - 左侧边栏: EXPLORER + 文件树
  - 中央: Tab 栏 + 面包屑 + 编辑器
  - 底部面板: PROBLEMS/OUTPUT/TERMINAL Tab
  - 状态栏: VSCode 蓝色背景, 分支/错误/行列/语言
- 步骤2: 真实功能 (140→216行)
  - 文件树: 真实目录扫描, 点击切换文件
  - 编辑器: egui TextEdit, 加载真实文件内容, 可编辑
  - 诊断: 真实调用 aine check, Problems 面板显示结果
  - 状态栏: 真实行列/错误/警告计数
  - 语言下拉: 6语言按钮 (EN/中文/日本語/Deutsch/Français/Русский)
  - 快捷键: F5 检查 / F6 检查
- 教训:
  1. 不要在烂代码上叠补丁, 推倒重来更快
  2. 每步改动后立即截图验证视觉
  3. 分块实现, 每块单独编译验证
  4. `'{'` 在 char 字面量中被 Python 计数器误判 — 用正确方法计括号
- 验证: cargo test 180 全绿; 截图确认 VSCode 风格布局+真实内容+全交互

## 2026-09-19 Aine Studio IDE 大规模迭代

### 编译器修复
- Vec.push 恢复返回容器（continue 修复误改返回 Unit）
- N0001 诊断：枚举变体与 UI 内建组件名冲突时 check 警告
- Option.unwrap 确认正常（旧 exe 问题）

### AI 接入
- AI 后端从手写 HTTP 切换到 Umber Runtime (umber_ffi.dll C ABI)
- 多厂商 Deployment 并存、5 协议透传、真流式、思维链、token 用量
- catalog.json 9624 模型选择器、runtime_status、Demo 模式、停止按钮
- 系统提示+当前文件上下文、Markdown 渲染、审批闭环

### Language Veil
- veil.rs 六语言词表镜像 alrender.aine（含 import 修正）
- 编辑器表面切换/双视图预览/canonical 导出/高亮覆盖
- T51 源映射 source_map()

### 架构
- 异步任务层（check/build/run/git/终端全后台化）
- 统一命令注册表 Cmd enum
- LSP 客户端（诊断推送/hover/completion/definition）
- 文件树缓存、编码检测(GBK Win32 API)、双实例检测、check 去抖
- 通知 toast、会话恢复、布局持久化

### UI
- Dark Modern 主题、活动栏 5 视图、面板 Enter、Tab 光标插入
- 命令面板模糊匹配+Enter、PROBLEMS BP/Debug 按钮、首错自动跳转
- 大纲视图+引用计数、任务系统、Code Lens 首错条、状态栏可点+Rx

### 详细状态
- 见 docs/AINE_STUDIO_STATUS.md
