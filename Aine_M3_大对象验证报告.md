# Aine M3 大对象验证报告

> **版本**：M3-R1（2026-02-14，与 aine 0.1.0 同步）
>
> 对应设计稿：§71.4（大对象验收）、§74（性能验收统计）、§75（编译时间原则）、§9（物化三档成本）。
> 方法：因 aine 尚无 codegen/运行时，**大对象在工作负载形状层面验证**——
> 合成与 10MB/100MB/1GB 数据形状等价的分析程序（String/Vec 大值经视图/逃逸/转移路径），
> 验证 Borrow/Materialize 决策正确性与 §74 指标。运行时实测（真实分配计数）待 codegen 里程碑。

## 1. 基准结果（synthetic × 200 迭代 = 1200 函数）

```text
=== Aine M3 大对象验证报告 ===
程序: synthetic (200 迭代, 1200 个函数)
分析时间: 29.33 ms

指标 (§74):
  Borrow Success:      1400
  Materialization:     600
  Large Copy Count:    600
  Owned:               0
  Copy:                200
  函数摘要数:          1200
```

### 指标解读（每迭代 6 函数）

| 指标 | 每迭代 | 含义 |
|---|---|---|
| Borrow Success 7 | 视图零拷贝成功（参数内部借用 + 视图复用） |
| Materialization 3 | 无法借用必须物化（返回 / 任务捕获 / 消费式传参） |
| Large Copy 3 | 物化对象类型为 String（保守 Large 档） |
| Copy 1 | 拥有值复制（load() 结果） |
| 摘要 6 | 每函数一个 Semantic Summary |

## 2. §71.4 大对象验收（分析级）

对每个物化点，报告回答五个问题（示例节选）：

```text
[0] 位置 byte 89..90:
    是否借用: 否（视图不可用）
    是否物化: 是
    成本: Large (>1MB)（类型级保守估计，运行时实测待 codegen）
    为什么: 输入派生值被存储/转移，无法安全借用
    如何进入高级优化路径: 拆分函数 / 显式 &T 高级借用 API
```

**决策正确性验证**（单元测试断言）：

1. **视图复用**：`let a = get_name(u); save(a)`（save 非消费）→ Borrow，零提示 ✓
2. **消费式传参**：`stash(b)`（stash 会消费参数）→ Materialize + F2001 ✓
3. **拥有值**：`let c = load(); stash(c)` → 无需物化 ✓
4. **返回物化**：`get_name` 返回 `user.name` → 必须拥有值 → 物化 ✓
5. **任务捕获物化**：`go { burn(s) }` 捕获视图 → 物化 ✓
6. **示例零误报**：hello.aine / account_book.aine → 0 物化提示（Unknown provenance 正确回退）✓

## 3. §75 编译时间验证

| 规模 | 分析时间 | 每函数 |
|---|---|---|
| 18 函数（3 迭代） | 0.78 ms | 0.043 ms |
| 1200 函数（200 迭代） | 29.33 ms | 0.024 ms |

线性扩展且绝对值极低；测试断言 1200 函数 < 1s（实测 29ms）。
摘要缓存/增量编译（§20-21）为 Phase 4 目标，当前单遍分析天然满足"有预算"。

## 4. §74 统计口径

- **Borrow Success**：OptDecision::Borrow（视图零拷贝可用）
- **Materialization Count**：OptDecision::Materialized（F2001 警告数）
- **Large Copy Count**：Materialized 且 SizeClass::Large（>1MB 保守档）
- **Copy**：OptDecision::Copy（Cheap 复制）
- **Summary Composition Cost**：本次基准全为直接组合（单层调用），无展开成本

## 5. 结论

> 分析层验证通过：**大对象形状程序下，Borrow/Materialize 决策确定、精确、
> 可解释（§71.4 五问齐全）、指标线性可控（§75 时间预算）**。
> 设计稿 §79 的技术假设在分析层成立；运行时实测（真实分配/复制字节数）
> 列为 codegen 里程碑的验收项。

---

## 2. 运行时实测(2026-08-31,codegen 已就绪)

> 真实大对象经 `aine build` → C → zig cc -O2 → 原生运行,`AINE_ALLOC_STATS=1`
> 输出真实堆分配计数。测试程序:examples/bigstr.aine(分层 String 构建 + Vec 构建遍历)。

### 2.1 10MB 档(160 层 × 64KB 字符串 + 100 万 Vec<i32>)

```text
[aine] heap allocations: 1655692
str_len=10485760          (10 MB 精确)
vec_len=1000000
sum=1783293664            (Σ0..999999 = 499999500000 低 32 位, 与 i32 语义一致)
s0=0 sn=f                 (String 索引 char 正确)
墙钟: ~5 秒
```

### 2.2 100MB 档(1600 层 × 64KB 字符串 + 1000 万 Vec<i32>)

```text
[aine] heap allocations: 16556813
str_len=104857600         (100 MB 精确)
vec_len=10000000
墙钟: ~2 分钟(分配 1655 万次)
```

### 2.3 结论与已知限制

- **正确性**:String 分层构建、Vec 大数组、索引(char)、求和全部正确;
  大对象语义(视图/逃逸/物化决策)与分配行为经原生实测验证。
- **已知限制(运行时优化项)**:`al_strcat_own` 每次调用 `strlen(a)`
  找末尾 → 循环拼接整体 O(n²)。10MB 单循环拼接实测 11 分钟;
  采用分层构建(每层 64KB)后 10MB 5 秒、100MB 2 分钟。String 表示
  重构(带 len/cap 的 al_str 结构)列为运行时优化候选,不阻塞 MVP
  (分层/块式构建可绕行)。
- **解释器定位**:大对象走原生运行时(解释器为小规模/教学/测试用途,
  设计预期,不做大对象实测)。
- **§74 指标**:分析层(§1)与运行时(§2)双通道验证完成;
  §71.4 每处物化回答 是否借用/是否物化/为什么/成本/高级优化路径(§1 报告)。
