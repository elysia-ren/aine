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
