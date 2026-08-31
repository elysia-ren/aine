# Aine Compiler Internals

> **版本**：V0.1（初版）
>
> **状态**：本版收录《Aine 设计决策登记簿》定稿版中属于编译器内部的决策（D1/D2/D6/D7/D10），作为实现 aine 的第一份内部规范。
>
> 面向：编译器贡献者、高级开发者、编译器研究人员（交付物 20）。
>
> 已收录章节：
>
> - §1 架构约束（D6）
> - §2 只读参数视图化 pass 与内部表示（D1、D7）
> - §3 View-eligible 规则与物化成本模型（D2）
> - §4 Send / Sync 推断规则（D4，引用）
> - §5 Codegen 策略（D10）

---

## §1 架构约束（优先级最高）

> **来源：登记簿 D6（🟢 已确认）**

### 1.1 精度降级不变量

> **摘要精度只影响生成代码的质量（分配数、复制次数、代码大小），绝不改变程序的可观察语义。任何精度组合（Level 0-5、跨模块缓存缺失、预算回退）下，同一源码必须产生行为一致的合法程序。**

### 1.2 派生约束

1. 所有分析 pass（Ownership / Escape / Capture / Summary）的输出**只能**影响 lowering 与优化选择，不得进入语义判定路径；
2. Materialize 是合法回退（V5.8 §9），物化决策必须附带可解释原因（§3.3）；
3. 分析预算超限 → 保守回退（V5.8 §59），禁止无限分析；
4. 本不变量纳入 Conformance 专项（同源码 Level 0/3/5 + 缓存全失效，行为断言一致）。

---

## §2 只读参数视图化 pass 与内部表示

> **来源：登记簿 D1、D7（🟢 已确认）**

### 2.1 目标

对满足条件的按值参数自动以视图实现，用户源码不可见；使 `print(user.name)` 对 100 MB String 的 Allocation Count == 0。

### 2.2 视图化条件（全部满足才允许）

1. 函数体对参数**只读**（无写入、无 move、无存储到状态/容器）；
2. 参数**未逃逸**（未进入任务/UI 操作/闭包捕获后被存储）；
3. 调用方实参可追踪为本地/字段投影（View-eligible，见 §3）；
4. **非泛型函数**（第一版硬限制；泛型视图化列入 Phase 2 后评估）。

### 2.3 内部表示：`str_view` 等视图类型

- `str_view`（及同类内部视图类型）是**编译器内部表示**：不出现在用户可见 API、用户源码、文档、IDE 显示与诊断文本（诊断模板层过滤）；
- 标准库高性能 API（`read_view` 类）公开返回 `&str`（借用内部缓冲区）或拥有值，内部实现可用 `str_view`；
- 用户在源码书写 `str_view` → 编译错误（提示使用 `String` 或 `&str`）。

### 2.4 pass 位置

在 VIR 之后、MIR 之前（V5.8 §56 流水线中"Borrow / Compiler View Lowering"步骤内）执行。

---

## §3 View-eligible 规则与物化成本模型

> **来源：登记簿 D2（🟢 已确认）**

### 3.1 View-eligible 返回规则

满足**全部**条件才允许 Borrow/View，否则 Materialize：

1. 所有 return 分支的值可追踪到同一调用作用域内的参数/局部字段投影；
2. 返回值在调用点未被存储、未跨任务 / UI 边界；
3. 无动态索引 / 不确定 provenance（动态索引、循环内选择等默认不 eligible）。

### 3.2 物化成本三档（定稿数值）

| 档位 | 单次物化对象大小 | 行为 | 实现要点 |
|---|---|---|---|
| Cheap | < 64 KB | 静默 | 直接物化，无提示 |
| Material | 64 KB - 1 MB | 非阻塞性能提示（默认开，可配置关闭） | 提示引用 §58 模板（F2104 风格），含原因与可选方案 |
| Large | > 1 MB | 强制提示 + IDE 仪表盘计数 | 不得悄悄产生高成本复制（V5.8 §10） |

> 阈值按"单次物化对象大小"计，全局可配置（aine 配置）。

### 3.3 物化诊断数据

每次物化至少记录：原因（数据流链 A→B→C）、成本档位、大小、可选方案（接受/拆分/显式 `&T`）。供 `flow explain` 与 IDE 物化仪表盘使用（交付物 7 / 25）。

---

## §4 Send / Sync 推断规则

> **来源：登记簿 D4（🟢 已确认），完整语义见《Flow_Concurrency_Guide.md》§3**

实现要点：

1. 字段级自动推断（struct / enum 各变体字段；容器按类型参数；闭包捕获按捕获内容）；
2. `#[not_send]` 显式标记仅影响跨任务/跨 Domain 检查；
3. 第一版不引入 `unsafe impl Send`；FFI 边界类型默认 not_send（经包装类型显式声明跨任务能力）；
4. 诊断输出业务语言（"这个值不能安全地发送到后台任务"），IDE 高级视图展开约束链。

---

## §5 Codegen 策略

> **来源：登记簿 D10（🟢 已确认）**

### 5.1 后端

- **采用 LLVM**：五平台 target 覆盖、与 Skia/工具链生态衔接；
- Codegen 作为独立模块（便于未来替换）；自研后端列为远期（非 1.0）。

### 5.2 配套策略

1. **LLVM 版本锁定**：跟随 LLVM 长期支持节奏（18 个月升级窗口），aine 发布时锁定并分发匹配工具链（签名、离线安装）；
2. **ABI 承诺**：1.0 前不承诺 ABI 稳定（同一版本内二进制兼容；跨版本需重编译）；
3. 移动端经 LLVM 对应 target 输出（UI 后端另见 D11 / UI Guide §6）。

### 5.3 流水线位置

MIR → Optimization → Codegen（LLVM IR 生成）→ Native Object → Link（V5.8 §56）。

---

## 附录 A：本版迁移来源

| 章节 | 来源决策 | 状态 |
|---|---|---|
| §1 | D6 | 🟢 已确认 |
| §2 | D1、D7 | 🟢 已确认 |
| §3 | D2 | 🟢 已确认 |
| §4 | D4（引用） | 🟢 已确认 |
| §5 | D10 | 🟢 已确认 |

## §6 自举编译器架构（aine 自举线，B5 系列 · 实况记录）

> Rust aine 为参考实现与开发宿主；以下为 **Aine 语言自举编译器** 的实际结构
> （全部由 Aine 编写，经 fixpoint 闭环验证：自宿主编译自身，输出与宿主逐行全同）。

### 6.1 模块布局（六模块 + 根文件）

| 模块 | 职责 | 行数 |
|---|---|---|
| transpiler.aine（根） | AST 类型定义 + 27 用例验证序列 | ~380 |
| allex | 词法分析 | ~150 |
| alparse | 递归下降解析 + 模块打平（flatten_stmts）+ 闭包脱糖（desugar_closures）+ 值语义（value_semantics）+ 值流子集（valueal_lite） | ~1500 |
| alstr | 字符串工具与源码回显（stringify 系） | ~310 |
| alcollect | 顶层信息收集器（类型/变体/函数/变量表） | ~260 |
| altype | 类型查询（c_type/base_type/var_info 等 45 函数） | ~1150 |
| alee | Aine→C 发射（c_expr/c_stmt/c_program） | ~1750 |

### 6.2 编译管线（c_program 薄壳）

```text
read 源 → tokenize(allex) → parse_program(alparse)
  → flatten_stmts      （mod name; → 读入 name.aine，防环；搜索路径 同目录→stdlib/）
  → desugar_closures   （独立闭包→顶层函数提升 + 捕获分析 + 隐藏参数 + 调用点改写）
  → value_semantics    （var-from-var：源未再用→MOVE+置零；否则→CLONE 深拷贝）
  → valueal_lite     （字符串累加器判定 → al_strcat_own 原地追加，消除 malloc+拷贝）
  → c_program_flat     （11 张类型表重算 → 逐语句发射 C）
```

### 6.3 值语义内建（解释器 / C 双侧对称）

| 内建 | 解释器 | C 发射 |
|---|---|---|
| al_vec_clone | Arc 共享（持久化） | 真深拷贝 malloc+memcpy |
| al_zero_vec / al_zero_map | 空 Vec/Map | 置空结构 |
| al_map_clone | entries 克隆 | 键值双深拷贝 |
| al_strdup_lit / al_strcat_own | 语义等价拼接 | strdup / realloc 原地追加 |

### 6.4 已知限制（均登记，均有等价 workaround）

1. **f-string 内方法链直插**（如 `f"{v.iter().map(...).sum()}"`）在原生路径
   段错误（fstr_piece AST 分支，见 wip_fstr_ast_path.aine.txt）；
   等价写法：先 `let s = ...; f"{s}"`（链本身走完整 c_expr 路径）。
2. **match 尾表达式为构造调用**：转译器丢返回值（B5-M32 定位）；
   源层规避：语句式改写（res 累积 + 显式 return）。转译器层修复列后续
   （需"是否函数尾"上下文）。
3. typeck 诊断与 formatter 的组件迁移：M6 分期计划（M6-FMT-1/2、M6-TCK）。

## 附录 B：待填充（Phase 1-4 期间）

AST/HIR 数据结构、VIR 节点定义、Ownership/Escape/Capture 算法、Summary 表示与组合、预算机制、增量编译与缓存、诊断系统、Source Map、Optimization Explain 实现。
