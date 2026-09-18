# Aine IDE 架构方案 v3.6(冻结 + 进度快照)

> 状态:**接口已冻结**(v3.3 冻结,此后 v3.4-v3.6 为增量修订),实现持续推进。
> 原则:先可用(UI+AI),后替换(全 Aine);接口先于实现。
> 版本历史:v3.3 多语言工程/评审修订 → v3.4 UI 跳过 Rust 壳 → v3.5 渲染拍板(D2D/DWrite 自绘 + Scintilla)→
> **v3.6(2026-09-04)文档一致性修订 + 实现进度快照**。

---

## 0. 总纲(一切决策的判据)

1. **接口先于实现**:跨子系统交互一律走"语义对象 + 方法契约";Aine 实现与临时宿主实现只换进程、不换协议。协议版本进每个请求,语义对象可加字段、不可改义。
2. **编译器语义是唯一事实源**:诊断/符号/类型/高亮/架构图全部由 Compiler Service 产出;UI 与 AI 只消费,不各自解析。**不要再改编译器架构去适应 IDE/AI——让 IDE/AI 通过 Tooling Protocol 消费 Compiler。**
3. **AI 是普通客户端,只提出变更**:AI 与用户共用同一套 Tooling Protocol;产品哲学:**AI 从不直接修改工作区,AI 只提出变更(DiffSet)**。协议层强制:AI 不得调用 `workspace.write`,AI 侧仅获得 `read / revision / applyDiff`。
4. **统一资源/安全边界模型**(生态总纲):Aine 对外部世界只规定统一的资源与安全边界;具体语言通过不同 backend 接入——不设计万能 FFI,不让任何外部对象模型污染 Aine 的值语义。
5. **协议与传输解耦**:Tooling Protocol 定义方法与对象;传输是可插拔实现(stdio/socket/pipe/TCP),P0 采用 JSON-RPC over stdio。
6. **五句话分工**(全平台架构哲学):
   > Compiler 提供事实 · Workspace 提供状态 · Development 提供行动 · AI 负责决策 · UI 负责呈现。
7. **语言表面与语义分离**(多自然语言编程,Language Veil):
   > Aine 只有一门语言语义(Canonical Aine),但拥有很多种人类语言表面(zh/en/ja/de/fr/ru…)。
   > 表面层只负责"程序如何被人表达",Compiler Core 负责"程序到底是什么";表面语言可以切换,
   > 但语义、类型、所有权、构建产物与运行行为不变。改变表面 = 重新渲染,不是翻译程序。
8. **多语言工程与互操作分层**(三条线,严格分离,不混成一个 Multi-language 模块):
   > ① 多自然语言编程 → Language Rendering(表面层,第 7 条)
   > ② 多语言源码工程 → Language Frontend Registry + Build Graph(Compiler 一级能力,≠FFI)
   > ③ 多语言互操作 → Native FFI / Foreign Runtime / IPC(调用外部代码)
   > 统一的是**工程与工具链接口**,不是强迫所有语言拥有同一种语言语义;
   > 各语言保留原生 AST 与原生语义(语言内语义不互相转换/伪装);Common IR 是可选能力。

---

## 1. 现状基座(为什么"用自己"成立)

```
已达成:
├─ L1 编译器本体 100% Aine (transpiler.aine + al* 模块 ≈9.3k 行)
│    └─ 验收: alinterp 解释 transpiler 与宿主逐字节一致 (15339 行 diff 0)
├─ transpiler 自举编译 → 原生 C exe (fixpoint: self_host 自生成 == aine 生成)
└─ 结论: Aine 写的程序(含服务/IDE 逻辑)写完即自举编译成原生进程;
         不背 alinterp ~400x 解释包袱 —— 解释器是开发期工具, 非交付形态

缺口(服务化 + AI + 生态必需):
├─ ① 持续 I/O   逐行读 stdin        (JSON-RPC 服务循环)      无
├─ ② JSON 编解码 协议对象/模型 API                            无
├─ ③ HTTP 客户端 流式/SSE 调模型 API (db/http 为 Nil 桩)      无
├─ ④ 容错解析   半截代码出 AST        parse 全有或全无
├─ ⑤ 查询 API   definition/references/type_at/hover
│              (collect/altype 表已含 80% 数据, 缺查询入口/索引)
├─ ⑥ ui{} 后端  窗口/控件/事件        ✅ 已实现: E4 原生控件原语
│                                     + S1e 自绘运行时(rt_*, D2D/DWrite)
├─ ⑦ FFI 语言面 extern/unsafe/opaque (产物即 C, 路径最短)    无
└─ ⑧ 语言渲染层 表面词法↔canonical/   无 (新增 alrender 层,
                source map/双向渲染      现有 L1 不动)
   ⑨ 多语言工程  Frontend Registry/     无 (C Frontend/Build Graph/
                Build Graph/跨语言关系   跨语言索引均待建, S0.7)
```

### Language Core 内部(含新增 Language Rendering)

```
Source (surface: zh-CN/en/ja/…, 按文件声明)
   │
   ▼
Language Rendering Layer (新增: alrender.aine 族)
   ├── Language Registry / Profiles (声明式: zh/en/ja/de/fr/ru)
   ├── Surface Lexer           表面词法 → Canonical Tokens
   ├── Grammar Mapping         表面语法短语 → canonical 节点
   ├── Canonicalizer           语义规范化(大于/より大きい → GreaterThan)
   ├── Source Map              canonical token ↔ 表面行列 (双向)
   ├── Renderer / Formatter    canonical → 指定表面 (双向渲染)
   └── Diagnostic Localization 结构化诊断 → 表面语言文字
   │
   ▼
Canonical Aine (英文语法即 canonical; 现有 L1 不变)
   ├── allex / alparse → alcollect/altype/altypeck/alvalueal → alee
```

铁律:只渲染"语言骨架"(关键字/语法短语/标准库别名/编译器消息);**不触碰用户内容**
(标识符/字符串/注释/数字)。用户标识符不自动翻译(可显式 `#[display(zh="…")]` 别名,
可选)。诊断结构化、语言无关(`code/severity/expected/found/args`),文字呈现由渲染层本地化
(保留标准术语头便于搜索)。Source Map 是硬需求:诊断/断点永远定位回**用户书写的表面坐标**。
新增语言 = 新增 Profile,不改 altype/altypeck/alvalueal/alee/IDE/AI。

### Multi-Language Engineering(多语言源码工程;与表面层/FFI 并列的第三条线)

```
Language Frontend Registry (源语言; 与 Language Registry(表面) 严格区分)
   ├── Aine Frontend   = 现有 L1 (allex/alparse/alcollect/altype/altypeck/alvalueal/alee),
   │                     终态目录 frontends/aine, 不拆现有实现
   ├── C Frontend      P0: Aine 原生最小实现(声明/原型级解析: 符号+extern 绑定;
   │                      诊断可接外部 clang 增强)
   ├── Rust/C++ Frontend  P1: 经 External 编译器(cargo/rustc)纳入统一工程
   └── External Frontends  P2+: Python/JS/Java… (外部编译器 + 统一工程接入)
         │
         ▼
Project Semantic Model (统一工程语义, 语言语义保持原生)
   ├── Symbol{language,…} · Diagnostic · Definition · Reference
   └── Cross-language Relation (ffi_binding/imports/exports/links_to/generated_from)
         │
         ▼
Build Graph (workspace/build 层): 谁依赖谁/谁先编译/谁链接/哪个 runtime 必需
         │
   ┌─────┴────────┐
Aine IR/Backend   External Artifact (C object/Rust artifact…)
   └──────┬───────┘
         Link/Run → 统一 executable
```

三种接入模式:① Native Frontend(Aine/C,深度支持)② External Compiler Frontend(Rust/C++,
Aine 不重写语言编译器,外部编译器纳入统一 Project/Build/Diagnostic/Artifact)
③ Foreign Runtime/Service(Python/Java/Node 作为独立程序经 IPC——属互操作线,不是源码输入)。

三种关系分层(不得混入同一模型):
- 语言内关系(Rust fn→Rust type)→ 各 Frontend
- 跨语言源码关系(Aine extern fn → Rust exported symbol)→ Compiler Service 跨语言索引
- 构建产物关系(crate → .a → linker → exe)→ Development Service / Build Graph

---

## 2. 目标架构树(产品视图)

```
┌────────────────────── Aine IDE (唯一产品入口, 一个软件) ─────────────────────┐
│                                                                              │
│  ┌────────────────────────────── UI 进程 ─────────────────────────────────┐  │
│  │  Studio Shell: Window/Layout/View/Command/Notification/State/Extension │  │
│  │  五区布局 (NAV | TABS/EDITOR | ASSISTANT | BOTTOM) · Editor 内核(自绘)  │  │
│  │  Command Bar · Focus Mode · Model Center                               │  │
│  └───────────────────────────────┬────────────────────────────────────────┘  │
│                                  │  Tooling Protocol (JSON-RPC over stdio,   │
│                                  │  传输可插拔: socket/pipe/TCP 后续)         │
│        ┌───────────┬─────────────┼──────────────┬───────────────┐           │
│        ▼           ▼             ▼              ▼               ▼           │
│  Workspace     Compiler      Development     AI Runtime     Run/Debug      │
│  Service       Service       Service         (Aine 实现)     (按需)         │
│  (提供状态)     (提供事实)     (提供行动)       (负责决策)      target.exe     │
│  project/      parse/diag/   build/run/      Agent/Task     调试器(P3)     │
│  file/         symbols/def/  test/debug/     Context/Router                 │
│  document/     refs/hover/   format/package  Provider×2                    │
│  revision/     lens/arch                                                    │
│  changes/                                                                   │
│  config/                                                                    │
│  session                                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
```

进程边界:UI/Workspace/Compiler/AI 常驻;Development 懒启动;Run/Debug 每次拉起。每个进程 = 可替换单元,换实现不换协议。

内部构成(与 §1/§4 一致):Compiler Service 内含 Frontend Registry(Aine/C/Rust…,按
Document.language 分派)与 Language Rendering(表面层,仅 aine 文件);Development
Service 内含 Build Graph 与工具链;Workspace 持有 Project/Document 权威状态。

### 状态边界(四态归属,防越界即防乱)

| 域 | 拥有 | 不含 |
|---|---|---|
| **Workspace** | project / file / **document** / revision / changes / config / session | 光标、选区 |
| **Compiler** | AST / semantic index / diagnostics / types / compiler cache | 文件内容副本 |
| **AI** | conversation / task / step / context / model state | 写盘能力 |
| **UI** | cursor / selection / scroll / layout / active view / focus / panel visibility | 权威源码 |

---

## 3. 角色分工(技术选型)

| 部件 | 实现方 | 角色 | 退役条件 |
|---|---|---|---|
| 语言核心/编译器 | Aine(L1) | 本体 | —(自举完成) |
| Compiler / Development / AI Runtime | Aine | 本体 | — |
| Workspace Service | Rust 起步 | backend | Aine 自实现顶替 |
| Std API 临时 backend | Rust | 临时实现方 | alee C 模板自实现 |
| UI 壳 | **Aine**(ui{} → alee Win32 C 后端) | 本体 | —(E4 已验证: counter.exe 真窗口) |
| 编辑器内核 | **Scintilla 控件**(成熟原生编辑器, SCI_* 消息; 高亮/行号/IME/撤销内建) | 第三方原生控件 | 保持(不自制 Monaco) |
| 开发期解释器 | Aine(alinterp) | 验证工具 | 仅保证核心语义一致;新语言面(extern 等)报"仅原生支持" |
| 外部库(开发期) | 优先 Rust(crates),经 C ABI/backend 接入 | 临时依赖 | 需要时换 Aine 自实现或原生库 |

---

## 4. 接口标准

### 4.0 协议分层总图

```
┌──────────────────────────────────────────────────────────┐
│                语义对象层 (Diagnostic/Symbol/DiffSet/…)   │ ← 三方共同地基
├──────────────────────────────────────────────────────────┤
│ Language/Runtime API   Aine 程序自己能调用的能力         │
│   (Std API: read_line/json/http/env/sleep/io/ui/ffi)     │
├──────────────────────────────────────────────────────────┤
│ Tooling Protocol       操作 Aine 项目的接口              │
│   workspace.*  compiler.*  dev.*                        │
│   (IDE/CLI/AI/Debugger/三方工具共用)                     │
├──────────────────────────────────────────────────────────┤
│ AI Protocol            Agent 管理接口 (ai.*)             │
└──────────────────────────────────────────────────────────┘
传输层(可插拔): stdio(P0) | socket | named pipe | TCP
```

### 4.1 Language/Runtime API(Std API;接口冻结,backend 可换)

```text
已有档:   print · assert · read_file · write_file · append_file
          file_exists · now · cli_args
服务化档: read_line()                               逐行读 stdin
          json_encode(v) / json_decode(s)           语义对象 ↔ JSON
          http_request(method,url,headers,body,on_chunk)   流式/SSE
          env(name) · sleep(ms)
UI 档:    ui{} 后端 window/控件/事件/布局(后续里程碑)
FFI 档:   extern fn 声明 · opaque 句柄 · unsafe 调用(原生路径)
```

规则:每个能力 = 接口(冻结)+ backend(可换:Rust 宿主临时 / alee C 模板终态 / extern FFI 直连原生库)。行为定义以 Aine 侧为准。

### 4.2 语义对象(统一 ID;带 v 版本,可加字段不可改义)

**ID 体系**:多项目/多窗口/多任务并存时一切以 ID 定位,不靠路径:
`WorkspaceId · ProjectId · DocumentId · RevisionId · TaskId · DiffSetId`

```jsonc
Pos        { line, col }          Range { start, end }   // 表面坐标(用户所见)
Diagnostic { id, workspaceId, projectId, file, range, severity:"error|warn|info",
             code:"E0312",
             message_key:"type_mismatch", args:{expected:"Connection", found:"Option<Connection>"},
             code_snippet? }
             // 结构化、语言无关; 显示文字由 Language Rendering 按表面语言渲染
Symbol     { id, language, kind, name, file, range, decl_range, type?, children?, relations? }
             // language 必填 (Rust::Client / C::Client / Aine::Client 可并存)
             // relations: calls/owns/borrows/returns/implements/imports/exports/
             //            ffi_binding/links_to/generated_from
Revision   { id: RevisionId, documentId, base }        // 文件版本
Document   { id: DocumentId, file, revision, dirty, text, based_on_revision,
             language,              // 源语言: "aine"|"c"|"rust"|…(Frontend 分派依据)
             surface_language }     // 自然语言表面: "zh-CN"|"en"|…(仅 aine 有效)
Project    { id: ProjectId, default_language, supported_languages[],
             languages[], targets[], dependencies[], toolchains[] }   // 元数据
BuildGraph { nodes:[{target, language, sources[], deps[], artifact}],
             edges:[{from, to, kind:"source|link|runtime"}] }
Dependency { language, kind:"package|crate|lib|runtime", source, version,
             artifact, build, runtime }
DiffFile   { file, hunks:[{start, lines_old[], lines_new[]}] }
DiffSet    { id: DiffSetId, files:[DiffFile], base_revision,
             status:"pending|checked|applied|rejected" }
Task       { id: TaskId, goal, scope:{files,symbols},
             steps:[{title,status,summary?,files?,model_hint?}],
             diffs:[DiffSet], compiler:{errors,warnings}, state }
```

### 4.3 Tooling Protocol

**workspace.\*(提供状态:项目/文件/文档/revision 的权威视图)**

| 方法 | 说明 |
|---|---|
| workspace/open(path) / close | 项目会话(返回 WorkspaceId/ProjectId) |
| workspace/openDocument(file) | 打开工作副本 → Document(此后编辑基于它) |
| **workspace/edit(docId, edits)** | **人工编辑提交通道**: edits:[{range,text}](键入/删除/粘贴
      均由编辑器增量提交) → revision++ → 通知 compiler 增量刷新 |
| workspace/files / read | 文件访问 |
| **workspace.write** | **普通工具写入(编辑器保存/工具落盘);AI 不得调用——协议层权限** |
| workspace/revision(docId) | 当前版本;变更通知 workspace/changed |
| workspace/applyDiff(diffSetId) | 应用 DiffSet → revision++ → 通知 compiler 增量刷新 |
| workspace/config / session | 项目配置 / 会话状态 |

编辑流:编辑器键入 → `workspace/edit` 增量提交(权威 Document 在 Workspace,UI 无副本);
保存 = `workspace.write` 落盘;AI 修改 = `applyDiff`。`edit`/`applyDiff` 均校验
`based_on_revision`,过期(他人已改)返回冲突错误,由调用方(UI/AI)重基后再提交。

**compiler.\*(提供事实:理解程序;无副作用;多 Frontend 统一入口)**

> compiler.* 按 `Document.language` 分派到对应 Frontend(Aine = 现有 L1;C = 声明级
> 最小 Frontend;Rust/C++ = External 编译器接入)。各语言语义保持原生,只统一输出
> Tooling 对象。跨语言源码关系(ffi_binding/imports/exports)由本服务的跨语言索引提供。

| 方法 | 返回 | UI 落点 |
|---|---|---|
| frontends() | 可用源语言列表(aine/c/rust/…) | 语言识别 |
| parse(documentId→tokens) | 高亮 token(按表面语言渲染) | Editor |
| render(documentId, surface) | canonical→指定表面文本(切换显示/复制,不写盘) | 语言切换 |
| languages() | 可用表面语言 + profile 信息 | 语言菜单 |
| diagnostics(documentId?) | [Diagnostic](surface 坐标) | Problems + 行内 |
| symbols(documentId?) / symbolsAt(pos) | [Symbol] / 类型+定义 | NAV SYMBOLS / 悬浮 |
| definition(pos) / references(pos) | SymbolRef | F12 / 右键 |
| crossReferences(pos) | 跨语言关系(ffi_binding/imports/exports/links_to) | 跨语言导航 |
| hover(pos) | 类型/签名(表面语言文字) | 悬浮 |
| typeAt(pos) | 类型 | 语义高亮 |
| semanticLens(documentId) | fn 级 relations | Semantic Lens |
| architecture(projectId) | 模块依赖图 | Architecture View |
| refresh(documentId, revision) → 通知 | 新诊断推送 | 编辑后自动 |

输入侧:parse 按 `Document.surface_language` 经 Surface Lexer→Canonicalizer 进 alparse;
输出侧:一切坐标(诊断/断点/符号)经 Source Map 换算回表面坐标返回。容错解析完成后,
所有语义方法在坏代码上仍须有结果。C/Rust 等语言文件无表面层,直接经各自 Frontend。

**dev.\*(提供行动:操作项目;有副作用,懒启动进程)**

| 方法 | 说明 |
|---|---|
| dev/build · dev/run · dev/test | 编译/运行/测试(日志+Diagnostic;多语言聚合结果) |
| dev/buildGraph(projectId) | 构建图:依赖/顺序/产物/链接(多语言工程核心) |
| dev/link | 跨语言产物链接(alee 产物 + C object + Rust artifact → exe) |
| dev/debug | 调试会话(断点/调用栈/变量, 跨语言栈帧统一呈现, P3) |
| dev/format(documentId) | 格式化(写盘走 workspace) |
| dev/package | 包管理(语言感知 Dependency, P2) |

### 4.4 AI Protocol(`ai.*`;AI 负责决策)

```text
ai/ask           一次性问答(context 引用)
ai/explain       入参 Diagnostic|Pos → 解释
ai/fix           入参 Diagnostic+Document → DiffSet(绝不直接写盘)
ai/task/start    goal+scope → Task(计划先行)
ai/task/step     推进/暂停/停止
ai/task/review   AI 自审自己的 DiffSet
ai/applyDecision Accept/Reject/逐文件 → workspace.applyDiff → 自动 refresh → 诊断回喂
ai/agent         自由 Agent 会话(工具循环, P2)
```

**AI 权限(协议层强制)**:AI Runtime 对 Workspace 仅获得 `read / revision / applyDiff`;`workspace.write` 对 AI 关闭。AI 修改唯一通道 = DiffSet → workspace.applyDiff:可审计、可撤销、可回退。

**AI Context(语言感知)**:Context Engine 只消费 Tooling 结果——按当前 Document.language
取对应 Frontend 的 diagnostics/symbols/definitions/references/types + 跨语言关系
(ffi_binding 等)+ dev.buildGraph;对无符号表的语言(如无 Frontend 的文件)降级为
词法级上下文,并显式标注来源等级(`semantic|lexical|raw`),防止 AI 幻觉出类型信息。
AI 不自行解析任何语言源码。

闭环:编辑 → revision++ → refresh → 行内 E0312 → [修复] → ai/fix(结构化 Diagnostic + AST/符号上下文)→ DiffSet → 逐块审批 → ai/applyDecision → workspace.apply → revision++ → refresh → 0 诊断 → dev/test 绿。

### 4.5 Model 四层

```
Agent Engine ──只认──► Router.select(任务画像) ──► Provider.session(model)
```

- **Capability(声明式)**:`context_window / streaming / reasoning:"none|native|emulated" / tool_call:"native|emulated|none" / parallel_tool_calls / structured_output / vision / prompt_cache / native_extensions[]`
- **Provider(每厂一个)**:`models()`、`chat(流式)`、`tools()`(native/emulated 差异在此消化)、`native(name,args)`。**P0 冻结:OpenAICompatible + Anthropic**——前者验证标准兼容覆盖面,后者验证"非 OpenAI 格式的真正 Provider"抽象是否正确;其余厂商 P1+ 按需求增加,不为数量而堆适配器。
- **Router(任务画像→模型;策略表在 models.toml,可改)**:
| 任务 | 默认 | 任务 | 默认 |
|---|---|---|---|
| completion | fast(Qwen/本地) | architecture/大重构 | strong |
| explain | main(DeepSeek,经 OpenAICompatible) | review | 可配第二家 |
| fix/refactor | main(经 OpenAICompatible) | | |
  支持任务内混合模型(Planner=strong/执行=main/审查=另一家),Task.step 级 model_hint 驱动。
- **Agent API**:Agent 只声明"要 tool_call+reasoning+长上下文的模型",不认厂商。

### 4.6 生态对接层(三层)

```
                    Aine 程序
                        │
             Aine 语言面(最小新增集)
       extern fn 声明 · opaque 句柄 · unsafe 调用
                        │
     ┌──────────────────┼─────────────────────┐
     ▼                  ▼                     ▼
① Native FFI       ② Foreign Runtime      ③ IPC/RPC
Aine ↔ C ABI       Aine ↔ Python/JVM/      Aine ↔ 外部服务
                     .NET/JS
     │                  │                     │
C/Rust/Zig/C++     opaque handle 资源模型  Tooling Protocol /
(zig cc 链接)      (拥有/释放/跨线程由     JSON-RPC/HTTP
                    Aine 管, 内部生命周期
                    不假装知道)
```

- **① Native FFI(最高优先)**:Aine 产物即 C → `extern fn` + 发射器直出 + zig cc 链接即闭环。**FFI v1 规范:opaque 句柄 + 受控 extern 函数集,不开放裸指针作为默认接口;`unsafe` 语义 = 允许进入 foreign boundary,不等于裸指针/任意内存操作全面开放**(裸指针作为更晚能力)。例:`extern type Socket` + `socket_open/socket_send/socket_close`——Aine 只知 Socket 是 opaque resource。
- **② Foreign Runtime(需要时再建)**:外部对象 = opaque 资源,由 Aine 资源表管理,不进值语义。
- **③ IPC/RPC**:高层服务当独立进程接入;传输复用 Tooling Protocol 的可插拔传输。
- 原则:不设计万能 FFI;统一资源/安全边界 + 各语言 backend。Rust↔Aine 同为 native+ownership 导向,是早期 richer interop 首选(仍经 C ABI 起步)。

### 4.7 传输与纪律

- Tooling Protocol 定义方法与语义对象;**传输可插拔**:P0 = JSON-RPC 2.0 over stdio;socket/named pipe/TCP 为后续实现(远程开发由此扩展)。
- 请求带 `"v":1`;语义对象只加字段不改义;新方法向后兼容。
- UI 永不直接读源文件做语义;AI 永不直接写盘(只经 DiffSet→workspace.apply);编译器状态由 revision 事件驱动,无轮询。

---

## 5. UI 方案

### 5.1 验收原则(产品布局规范)

代码优先默认布局 · AI 随叫随到(Inline=Ctrl+I / Assistant=Ctrl+Shift+I / Agent=Ctrl+Shift+Enter)· 一切功能靠近代码(错误旁 [解释][修复][忽略])· Focus Mode 一键全隐 · Diff 一等公民 · AI 永不强迫全屏。
**多语言表面**:编辑器右下语言指示器(`中文 · Aine`),点击切换表面语言 = 调 `compiler.render` 重绘,**不写盘、不改语义**;补全/悬浮/诊断/格式化全部经 canonical 语义 + 表面渲染,不维护多套逻辑。**复制 = 输出 canonical 通用文本(贴论坛/评审);粘贴 = 自动本地化为当前表面语言**。
**AI 侧栏主体 = 当前工作,不是聊天记录**:Context(文件/符号/诊断)→ Task(目标/计划/步骤)→ Diff(待审批变更);对话只是其中一部分。运行失败(`Process exited with code 1`)旁提供 [Explain failure][Fix][Debug with AI]——与编译错误同一 AI 链路。

### 5.2 Studio Shell 内部架构(UI 内部规范,防膨胀)

```text
Studio Shell
├── Window Manager     窗口生命周期
├── Layout Manager     五区布局/可隐藏/持久化
├── View System        View 注册制: Editor/Explorer/Search/Run/Debug/
│                      Tests/Packages/Docs/Assistant…
├── Command System     Command Bar(Ctrl+K: 命令 + 自然语言→任务识别)
├── Notification System
├── State Store        单向数据流; UI 状态仅含光标/选区/滚动/布局/活动视图/焦点
│                      (权威文件内容在 Workspace 的 Document, 见 §2 状态边界)
└── Extension / Contribution System (P2)
```

### 5.3 组件排期

| 区 | P0(能用) | P1 | P2 |
|---|---|---|---|
| NAV | 文件树(按语言分组)、诊断角标 | SYMBOLS / DEPENDENCIES | Architecture View |
| EDITOR | 自绘缓冲、语法高亮(按 Document.language 分派: aine→服务 token / c→C Frontend)、行内诊断、错误旁 AI 按钮 | 语义高亮/悬浮/F12/References(跨语言) | Semantic Lens、Focus Mode+浮动 AI |
| ASSISTANT | 当前工作视图(Context+Task,非聊天记录)、explain/fix | AI Task(计划/步骤/文件)、Diff 逐块审批 | 混合模型任务、Agent |
| BOTTOM | Problems(多语言聚合)、Terminal | Tests、AI Tasks | Run/Debug |
| 全局 | Command Bar(Ctrl+K:命令+自然语言→任务识别)、Model Center、语言切换器 | Diff 视图 | 插件系统 |

---

## 6. 核心闭环(用户视角只有一个按钮)

```
编辑 → revision++ → compiler.refresh → 行内 E0312 → [修复]
     → ai/fix(Diagnostic 对象) → DiffSet → 逐块审批
     → ai/applyDecision → workspace.applyDiff → revision++ → refresh → 0 诊断
     → dev/test → 绿
```

---

## 7. 实现路线

> **执行策略(用户导向)**:主产品路径 = S0 → S1 → S2("能用"优先,UI+AI 先到手)。
> S0.5/S0.6/S0.7 是语言面扩展(FFI/表面层/多语言工程),接口已在 S0 冻结,
> 实现与 S1/S2 **并行推进、按需穿插**,不阻塞 UI/AI 主路径。

```
S0  平台冻结主线先行 ✅(完成度: Std API/协议/服务/容错解析 ✅; 原生化差 JSON AVal 化)
    ├─ Std API(Language/Runtime)冻结 + 宿主临时 backend + alee 模板同步
    ├─ Tooling Protocol 冻结: workspace.*/compiler.*/dev.* 契约(含 Document/权限)
    ├─ 多语言对象契约冻结: Document.language / Symbol.language / BuildGraph /
    │    Dependency / crossReferences —— 一次定齐, 实现分阶段
    ├─ Language Rendering Interface 冻结: surface_language 字段 + Profile schema
    │    + Source Map 契约 + 诊断 message_key/args 结构
    ├─ transpiler 服务循环(read_line → dispatch → JSON 行) + 查询 API
    │    (definition/references/type_at/hover 索引)
    └─ 容错解析(编辑器生命线)
    判定: 服务 exe 原生运行, 外部发 JSON 收诊断/符号/变更集
    注: S1 最小 UI Shell 仅作协议验收客户端并行验证, 不允许 UI 反向塑造协议

S0.5 Native FFI 最小集 ⬜
    ├─ extern/unsafe/opaque 语言面 + 发射器直出 + zig cc 链接
    └─ 真实 C 库用例闭环(Socket 类 opaque 样例)
    判定: Aine 程序调用外部 C 库编译运行成功

S0.6 Language Rendering bootstrap ⬜
    ├─ alrender.aine 最小实现: Language Profile(zh-CN + en, 声明式)
    │    + Surface Lexer → Canonicalizer → alparse
    │    + Renderer/Formatter(canonical → zh/en) + Source Map(双向)
    ├─ 中文/英文各写同一程序 → 同一 Canonical AST
    └─ 验收: AST/类型/所有权/构建等价 · round-trip(zh→canonical→zh)·
             诊断定位回中文坐标 · 不依赖 LLM · 离线可编译
    判定: 中文源码与英文源码编译产物一致; IDE 可在 zh/en 间切换显示

S0.7 Multi-Language Engineering P0 ⬜
    ├─ Frontend Registry 抽象: LanguageFrontend 接口(detect/parse/diagnostics/
    │    symbols/definition/references/hover/format/lower)
    ├─ C Frontend(Aine 原生最小实现: 声明/原型级解析 → 符号+extern 绑定;
    │    诊断可接外部 clang 增强)
    ├─ Build Graph: 统一 build/link/run(alee 产物 + C object → 同一 executable)
    └─ 跨语言符号: extern fn ↔ C 符号 的 ffi_binding 关系
    验收(project: main.ain + native.c + tests): 自动识别语言 · 双 Frontend 诊断 ·
        文件/符号导航 · definition/reference · build graph · Aine→C ABI 调用 ·
        统一 build/test/run · AI 可见跨语言上下文 · 一个最终 executable
    判定: main.ain 调用 native.c 的符号, 统一构建出可运行 exe

S1  Aine UI 主线 🔄(S1a/S1b/S1c ✅, S1d 壳 v0 ⬜)
    ├─ S1a ✅ ui{} DSL → 自绘运行时映射: Window/Text/Button/Panel/List/
    │    事件块 → window_new/label_new/button_onclick...(E4 已验证原语层)
    ├─ S1b ✅ 控件面扩充: input(EDIT)/listbox 已内建; 菜单/Row/Column 后置
    ├─ S1c ✅ 顶层 var(@global var 全链: alparse/alee/alcollect)
    ├─ S1d ⬜ IDE 壳 v0(Aine): 窗口+文件树(listbox)+编辑区(Scintilla)+Problems
    │    +按钮条; 同进程 rpc() 直调 svc_handle(协议冻结 ⇒ 同进程直调 ≡
    │    跨进程 JSON-RPC, 拆分随时可做)
    ├─ S1e ✅ 自绘运行时(D2D/DWrite + mockup 主题; counter 自绘窗口已验证;
    │    Scintilla 集成 ⬜ — wscite DLL 仅导出 DirectFunction, 需源码自编)
    └─ 判定: 全程无 Rust UI 代码; 壳本身是 aine build 产物;
          打开项目 → 编辑 → 行内诊断出现

S2  AI 接入 ⬜
    ├─ AI Runtime(Aine): Provider×2(OpenAICompatible/Anthropic) + Router
    ├─ ai/ask/explain/fix → DiffSet → 审批 UI → workspace.apply → 自动重编译
    │    (AI 生成的表面程序同样经 compiler canonicalize, 与用户程序同路径)
    └─ Model Center(models.toml)
    判定: 错误旁点 [修复] → Diff 审批 → 接受 → 编译转绿, 全程不出 IDE

S3  深化 ⬜
S4  Aine 化收尾 ⬜(每步换进程不换协议):
    ├─ Std backend → alee C 模板自实现; 语言补命名空间/真模块
    ├─ 编辑器内核自绘深化(EDIT 控件 → Aine 自绘; UI 壳 S1 已是 Aine, 无需替换)
    ├─ 服务拆分为独立进程(如需远程/多窗口): 同进程直调 → stdio JSON-RPC
    ├─ 目录演进: L1 → frontends/aine, 多语言 Frontend 归位 frontends/*;
    │    Language Rendering/Profile 归位 languages/*
    └─ 终态: Rust 只剩宿主工具链(aine build/run + 前端诊断), UI 零 Rust
```

---

## 8. 决策点(已拍板,冻结)

1. **S0 主线先行;S1 最小 UI Shell 并行作为协议验收客户端。UI 不反向塑造 Tooling Protocol。**
2. **ui{} S1 并行立项,但只实现最小后端与垂直样例;S4 完成 UI Shell/Editor 的 Aine 化替换。**
3. **P0 Provider:OpenAICompatible + Anthropic。其他 Provider 按需求在 P1+ 增加。**
4. **FFI v1:opaque 句柄 + 受控 extern 函数集;`unsafe` 表示进入 foreign boundary,不默认开放裸指针和任意内存操作。**
5. **Workspace/UI 状态边界按 §2 执行(四态归属);UI 不持有第二份权威源代码。**
6. **Workspace 增加 Document/Buffer 对象:Document 保存当前编辑内容及 base revision;UI 不持有第二份权威源码。**

| 决策 | 结论 |
|---|---|
| S0 / S1 顺序 | S0 主线先行,S1 最小并行(协议验收) |
| ui{} | S1 开始研究/垂直样例,S4 替换正式壳 |
| P0 Provider | OpenAICompatible + Anthropic |
| FFI v1 | opaque + 受控函数集,不裸指针 |
| Workspace / UI | 按四态边界执行,并补 Document/Buffer |
| 多自然语言 | 磁盘存储 = 用户书写表面(文件声明 surface_language);canonical 为编译中间;切换 = compiler.render 纯渲染不写盘;bootstrap = zh-CN + en;只渲染语言骨架,不碰用户标识符/字符串/注释;诊断结构化语言无关,文字按表面语言本地化;语义等价(同 AST/产物)为验收硬标准 |
| 多语言工程 | P0 = Aine + C(C Frontend = Aine 原生声明级实现,诊断可接 clang);P1 = Rust/C++(External 编译器接入);各语言保留原生语义,统一工程对象;L1 不拆,终态目录演进 frontends/* |
| UI 渲染(v3.5 拍板) | Win32 + Direct2D/DirectWrite 自绘壳 + Scintilla 编辑器控件;WebView2 排除;原生控件壳方案取消;ui{} → 自绘运行时(rt_*) |

**多自然语言补充决策(纳入冻结基线)**:
1. **存储**:P0 磁盘 = 用户书写表面(文件声明 `surface_language`);canonical 化发生在编译入口;IDE 切换语言 = 纯渲染,不写盘不改语义。(canonical 存储可作为 P1 配置项)
2. **范围**:bootstrap zh-CN + en;Profile 声明式,后续加 ja/de/fr/ru 只增 Profile 不改 Compiler/IDE/AI。
3. **铁律**:只渲染关键字/语法短语/标准库别名/编译器消息;用户标识符/字符串/注释绝不自动翻译;用户标识符可选显式 `#[display(zh="…")]` 别名。
4. **验收硬标准**:zh 与 en 写同一程序 → 同一 Canonical AST → 类型/所有权/构建产物一致;round-trip(表面→canonical→表面)语义等价;诊断/断点定位回用户表面坐标;不依赖 LLM、离线可编译。

**多语言工程补充决策(纳入冻结基线)**:
5. **P0 语言 = Aine + C**(Aine→C→zig cc 链路现成,C 是天然第二 Frontend/Build Target);C Frontend 以 Aine 原生实现声明级最小 Native Frontend,诊断可接外部 clang 增强。
6. **P1 = Rust/C++**:Rust 经 External 编译器(cargo/rustc)纳入统一工程(不重写 Rust Frontend);C++ 同线。P2+ = Python/JS/Java 走 External + Foreign Runtime + IPC。
7. **各语言保留原生 AST/语义**,不向 Aine AST 强行转换;统一的是 Symbol/Diagnostic/BuildGraph 等工程对象。
8. **终态目录演进** `frontends/aine · frontends/c · …`,现有 L1 不拆,即 `frontends/aine` 的实现。

---

## 9. 完成计划(细粒度执行清单,按序)

> 主线 = 产品路径(S0→S1→S2→S3);支线 = 语言面扩展,接口已在 S0 冻结,
> 插主线窗口推进;收尾 = Aine 化。每任务给"动作 | 产物 | 验证"。

### 主线 A:S0 平台冻结

#### A1 Std API(宿主临时 backend)
- T01 盘点现状 | 产出《Std API 现状对照表》:宿主 interp.rs builtin 分发点、alinterp.aine 的 `fname=="…"` 表、hir.rs BUILTIN_GLOBALS、alee C 模板内建区,逐项列出已有/缺失 | 对照表与代码一致
- T02 冻结签名 | 定 read_line/json_encode/json_decode/http_request/env/sleep 的 Aine 签名、参数、错误语义(写入 Aine_Std_Lib_Reference.md) | 签名文档评审通过
- T03 宿主实现 read_line/env/sleep | interp.rs builtin 分支 + hir.rs 注册;read_line 从 stdin 读一行(可含 \0 安全),env 读环境变量 | aine run 小程序调用三函数输出正确
- T04 宿主实现 json_encode/json_decode | 最小 JSON(对象/数组/字符串/数字/bool/null,UTF-8 转义,不引第三方或按仓库依赖策略) | round-trip 测试:encode(decode(x))==x
- T05 宿主实现 http_request | 阻塞 HTTP/1.1 GET/POST + 流式 body 回调(SSE 按行);headers/超时 | 本地 mock 服务返回流式数据逐块收到
- T06 alinterp 同步 | alinterp.aine 分发表加同签名 5 内置;read_line/json/env 行为与宿主一致(http 在 alinterp 报"仅原生"或同样走宿主桥) | 双引擎 mini 测试输出一致
- T07 验证 | mini_std.aine:read_line+json round-trip+env | 宿主与 alinterp 结果一致

#### A2 协议 schema 冻结
- T08 schema 文件 | docs/protocol/: 语义对象(json)+ 方法表(workspace/compiler/dev)+ 权限与版本规则 | 与 v3.3 文档 §4 逐字段核对一致
- T09 对象字段核对 | language/surface_language/based_on_revision/统一 ID 齐备;AI 禁 write 权限写入契约 | 评审通过

#### A3 服务循环 + 查询 API
- T10 服务入口 | transpiler.aine 加 build 模式分支:cli_args[0]=="--serve" 进入 read_line 循环(不退出) | --serve 起后 stdin 有输入即处理
- T11 JSON-RPC 帧层 | 请求{id,method,params}/响应/通知/错误;版本 v1 | 脚本收发各类型帧正确
- T12 方法分派第一批 | workspace.open/openDocument/edit/revision;compiler.parse/diagnostics/refresh;dev.build | 逐方法脚本验证
- T13 查询 API | definition/references/type_at/hover(基于 collect/altype 表 + 按名索引) | 对 mini 程序查询结果与预期一致
- T14 增量 | refresh(documentId, revision):按 revision 增量重解析,缓存复用 | 连续编辑只重算变更部分(粗测:大文件第二次 refresh 快于首次)
- T15 自举原生 | compiler_service.aine 由 transpiler 编译为原生 exe | exe 服务与解释器服务输出一致
- T16 CLI 验收 | test_client(python/curl):发 JSON 收诊断/符号/变更集 | 验收脚本通过

#### A4 容错解析
- T17 设计 | alparse 错误恢复方案:错误 token 跳过至语句边界;parse 返回"部分 AST+错误列表" | 设计说明
- T18 实现 | parse_program/parse_stmt/parse_expr 容错路径;好代码路径零改变 | 坏代码样例:出部分 AST 且不 panic
- T19 容错下语义 | 坏代码上 diagnostics/symbols 有结果 | 断句文件查询可用
- T20 回归 | 好代码路径:mini 套件 + transpiler 全量(15339 行)仍逐字节一致 | diff 0

### 主线 B:S1 最小 UI 壳

#### B1 Aine UI 基建(✅ 已完成)
- T21a ✅ ui{} 编译映射 | alparse ui{} 解析(已有 SUi)+ alee 发射到自绘运行时 rt_*(S1e) | counter 自绘窗口验证
- T21b ✅ 控件扩充 | input(EDIT)/listbox 内建 + C 模板(原生轨)+ 自绘 List;菜单/Row-Column 后置 | uiwidgets 可交互
- T21c ✅ 顶层 var | @global var 全链(alparse 解析/alee C 全局发射/collect 表注入) | 全局状态跨回调读写
- T21d rpc() 直调 helper | 壳内构造 JSON 请求→svc_handle→解析响应(同进程, 协议等价跨进程) | 壳拿到诊断/符号

#### B2 IDE 壳 v0(Aine)
- T22 壳骨架(Aine) | 五区布局: NAV(listbox 文件树)/EDITOR(EDIT 控件)/BOTTOM(Problems listbox)/按钮条 | aine build 出壳 exe, 布局可显隐
- T23 编辑闭环 | EDIT 内容→workspace/edit→refresh→Problems 显示诊断 | 编辑即出诊断
- T24 文件树交互 | 双击文件→openDocument→EDIT 载入;保存按钮→workspace.write | 文件可开可存

#### B3 命令/设置
- T29 Command Bar | Ctrl+K(input+listbox):命令注册表+执行 | 常用命令可执行
- T30 设置+语言占位 | settings.toml 读写;右下语言指示器(zh/en 待 E2 就绪先显示 en) | 设置持久化

### 主线 C:S2 AI 接入

#### C1 AI Runtime(Aine)
- T31 进程骨架 | ai_runtime.aine:复用 A3 服务循环;Std http/json | 起服务响应 ai/ask
- T32 Provider OpenAICompatible | chat 流式(SSE)/tools;Base URL+Key+Model 配置 | 对真实 API 流式对话成功
- T33 Provider Anthropic | native 消息/工具格式 | 同上
- T34 Capability+Router | 能力描述结构;静态路由表(models.toml) | 任务画像→模型选择正确

#### C2 对话/修复闭环
- T35 ai/ask+explain | 侧栏对话;诊断→解释(带 Context 引用) | UI 可用
- T36 ai/fix | Diagnostic+Document→DiffSet(base_revision) | 修改正确成 Diff
- T37 审批 UI | Diff 逐块 Accept/Reject/逐文件;视图 | 决策生效
- T38 闭环 | applyDecision→workspace.applyDiff→自动 refresh→诊断回喂 | **错误旁 [修复]→审批→接受→编译转绿(判定:能用软件达成)**
- T39 Model Center | models.toml 可视化编辑 | 换模型不改代码

### 主线 D:S3 深化
- T40 符号导航 | symbols→NAV SYMBOLS;definition/references→F12/右键(跨语言);typeAt→语义高亮 | 符号跳转可用
- T41 Semantic UI | semanticLens→Lens 视图;architecture→架构图;Focus Mode+浮动 AI(Ctrl+I) | 功能呈现
- T42 AI Task | task/start/step/review UI;混合模型(model_hint) | 工程任务计划执行
- T43 Tests/Debug | dev.test 聚合面板;dev.debug(跨语言栈帧);Run/Debug 进程 | 失败测试一键交 AI

### 支线 E(并行窗口,接口已冻结,顺序自选)

#### E1 FFI(S0.5)
- T44 语言面 | alparse/ast:extern fn/opaque 类型/unsafe 标注语法+变体 | 解析通过
- T45 发射 | alee:extern 声明直出 C;调用生成;zig cc 链接 | mini ffi 编译运行
- T46 执行器同步 | alinterp 对 extern 调用报"仅原生支持"(不静默) | 明确报错
- T47 用例 | 真实 C 库(socket 或 zlib)opaque 句柄闭环 | Aine 调 C 成功

#### E2 Language Rendering(S0.6)
- T48 Profile | languages/zh.json+en.json(关键字/语法/操作符/诊断模板)声明式 | 新增语言=新增文件
- T49 alrender 输入 | Surface Lexer(表面词法→canonical tokens)+Canonicalizer(语义规范化) | 中文源码→canonical AST 与英文一致
- T50 alrender 输出 | Renderer/Formatter(canonical→zh/en;复用 alstr.stringify) | 切换渲染正确
- T51 Source Map | 双向 token↔表面行列;诊断定位回表面 | 错误行号=用户所见行号
- T52 验收 | 中/英同程序同 AST;round-trip;类型/所有权/构建等价;不依赖 LLM | 验收清单全过
- T53 IDE 接入 | compiler.parse/render 带 surface_language;编辑器切换 zh/en 生效 | 切换显示不写盘

#### E3 Multi-Language P0(S0.7)
- T54 Frontend 抽象 | LanguageFrontend 接口+注册表(compiler.frontends) | 注册新语言不改核心
- T55 C Frontend | Aine 实现声明/原型级解析:符号表+extern 绑定 | native.c 符号可见
- T56 C 诊断增强 | 可选接 clang -fsyntax-only 并入统一诊断 | 诊断聚合正确
- T57 Build Graph | 对象+构建顺序+统一 link(zig cc 链 .c/.o/alee 产物) | 依赖顺序正确
- T58 跨语言关系 | ffi_binding/imports/exports 索引+crossReferences | Aine extern↔C 符号互通
- T59 验收工程 | main.ain+native.c+tests:统一 build/test/run→一个 exe | §S0.7 验收清单全过
- T60 IDE 多语言 | 高亮/诊断按 Document.language 分派 | C 文件语义可见

#### E4 ui{} 最小后端(✅ 已完成,演化为 S1e 自绘运行时)
- T61 渲染 | 宿主 run_ui:文本桩→真窗口渲染 ui{} 组件树(Window/Text/Button/Row/Column) | ui{} 出窗口
- T62 事件 | Button 点击→Aine 回调(@state 更新→重渲染) | counter 样例
- T63 垂直样例 | ui{} 写 counter/列表;与 Rust 壳并存验证 | Aine 自己出可交互窗口

### 收尾 F:Aine 化
- T64 Std backend Aine 化 | alee C 模板自实现 read_line/json/env(http 可后);宿主临时实现移除 | 行为不变(对照测试)
- T65 命名空间/真模块 | 语言补课:module 带命名空间;编译器/服务/工具多文件化 | 50 文件项目无名字冲突
- T66 UI Aine 化 | ui{} 成熟后重写 Shell/Editor(自绘缓冲 Aine 版) | Rust UI 退役
- T67 目录演进 | L1→frontends/aine;alrender→languages/*;C Frontend→frontends/c | 结构与终态一致
- T68 终态回归 | 全量:cargo 套件、transpiler 逐字节、服务双引擎、fixpoint | 全绿;Rust 仅 bootstrap

### 依赖要点
- A 组(01-20)不可跳序(地基);B 依赖 A3 + E4(已完成: Win32 原语层);C 依赖 A1+A3;D 依赖 B。
- E1 依赖 A1;E2/E3 依赖 A3(查询/服务);E4(T61-63)已完成最小闭环, T21a 是其延续。
- F 组:64 依赖 E1 经验与 A1 契约;65 依赖 E3 工程需求;66(编辑器自绘)依赖 T21b 控件面。
- 注: T15 原生化遗留(C 侧 JSON 嵌套 AVal 化)不阻塞 B(同进程直调绕开 JSON 文本);
  S2 原生 http 决策(AI Runtime 解释器跑 vs C 侧 TLS)在 S2 开工时拍板。


---

## 10. 实现进度快照 v3.7（2026-09-19，Aine Studio IDE）

> 上一份快照为 v3.6 头部。本快照反映 Aine Studio IDE（Rust/egui 实现）的**实际完成状态**。
> 详细逐项状态见 `docs/AINE_STUDIO_STATUS.md`。

### 已完成（较 v3.6 新增）

| 里程碑 | 内容 | 实现方式 |
|---|---|---|
| S1d IDE 壳 v0 | 编辑器/文件树/搜索/Git/大纲/任务系统/终端/测试面板 | Rust/egui（暂代 Aine 自绘，接口不变） |
| S2 AI 接入 (T31-T39) | 多厂商 Deployment/5 协议/能力路由/catalog 选择器/真流式/思维链/用量/审批闭环/Model Center | Umber Runtime (umber_ffi.dll) C ABI |
| S2 T53 Veil IDE 接入 | 编辑器 6 语言表面切换/双视图预览/canonical 导出/源映射 | Rust veil.rs 镜像 alrender.aine |
| T40 符号导航 | Go to Def (F12/Ctrl+右键/LSP)、Find Refs (Shift+F12)、hover (LSP)、Outline | ide_definition/ide_references/ide_symbols/ide_hover |
| T43 Tests/Debug | 测试面板(aine test 聚合)、调试器(aine debug 断点+报告) | 后台任务 |
| T29 Command Bar | Ctrl+K 命令面板，模糊匹配+Enter 执行，Cmd enum 单源 | Rust |
| T30 设置+语言 | settings 持久化(Provider/Key/Model/语言/侧栏宽/会话恢复)，6 语言 UI 翻译(50 键×6 表) | Rust |
| LSP 接入 | 内嵌 aine lsp 子进程：initialize/didOpen/didChange/publishDiagnostics/hover/completion/definition | JSON-RPC over stdio |
| workspace.edit 闭环 | 统一写盘入口 workspace_edit() + revision 递增 + 状态栏 Rx | Rust |
| 任务系统 | 活动栏 ⚙ 视图：aine_tasks.json 自定义任务点击执行 | Rust |
| 通知 toast | 右下角 4 秒过期 | Rust |
| 编码检测 | BOM/UTF-16 拒绝/GBK 真解码（Win32 API 936 代码页） | Rust |
| 架构修复 | 异步任务层(UI 零冻结)/check 去抖/统一 Cmd/命令面板模糊匹配/文件树缓存/double-instance 检测 | Rust |
| Bug 修复 | Vec.push 链式语义恢复 / N0001 UI组件冲突警告 / Option.unwrap 确认 / 首错跳转 / Code Lens / gutter 标记 / 状态栏可点 | Rust |

### 尚未开始（同 v3.6 缺口）

| 缺口 | 归属 |
|---|---|
| 编辑器自绘内核（折叠/多光标/缩放/Code Lens 行内） | 需替换 egui TextEdit |
| T42 AI Task 步骤 UI | S3 |
| 调试器单步/变量窗 | S3 |
| 架构图渲染 (T41) | S3 |
| LSP 增量诊断（区间 didChange） | S3 |
| E1 FFI / E3 多语言工程 | S0.5/S0.7 |
| 全 Aine 化 (T64-68) | S4 |
| accesskit 无障碍 | 低优先 |
