# Tooling Protocol 方法表 v1(冻结)

> 传输:JSON-RPC 2.0 风格,逐行 JSON over stdio(每请求一行、每响应一行)。
> 请求:{"v":1,"id":n,"method":"…","params":{…}}  响应:{"v":1,"id":n,"result":…|"error":{code,message}}
> 权限:AI Runtime 对 workspace 仅 read/revision/applyDiff;workspace.write 对 AI 关闭。

## workspace.*(提供状态;权威:项目/文件/Document/revision)

| 方法 | params | result | 说明 |
|---|---|---|---|
| workspace/open | {path} | Project | 打开项目会话;检测语言/目标 |
| workspace/close | {projectId} | null | |
| workspace/openDocument | {projectId, file} | Document | 打开工作副本(读盘, dirty=false) |
| workspace/edit | {documentId, edits:[{range,text}]} | Revision | 人工编辑提交 → revision++ → 通知 compiler.refresh |
| workspace/files | {projectId} | [file] | |
| workspace/read | {projectId, file} | {text, revision} | |
| workspace/write | {projectId, file, text} | bool | 普通工具落盘;**AI 禁调** |
| workspace/revision | {documentId} | Revision | |
| workspace/applyDiff | {diffSetId} | Revision | 应用变更集 → revision++ → 通知 refresh;base_revision 过期返回冲突错误 |
| workspace/config | {projectId} | config | settings/project 配置 |
| workspace/changed(通知) | {documentId, revision} | — | 服务 → UI 推送 |

## compiler.*(提供事实;无副作用;多 Frontend 按 Document.language 分派)

| 方法 | params | result | 说明 |
|---|---|---|---|
| compiler/frontends | {} | [lang] | 可用源语言 |
| compiler/parse | {documentId} | [token] | token:{text, kind, range};按表面语言渲染 |
| compiler/render | {documentId, surface} | text | canonical→表面文本(切换/复制,不写盘) |
| compiler/languages | {} | [surface] | 可用表面语言 |
| compiler/diagnostics | {documentId?} | [Diagnostic] | 坐标=表面坐标 |
| compiler/symbols | {documentId?} | [Symbol] | |
| compiler/symbolsAt | {documentId, pos} | Symbol+type+decl | 悬浮 |
| compiler/definition | {documentId, pos} | SymbolRef | F12 |
| compiler/references | {documentId, pos} | [SymbolRef] | |
| compiler/crossReferences | {documentId, pos} | [SymbolRef] | 跨语言 ffi_binding/imports/exports |
| compiler/hover | {documentId, pos} | {type, signature, docs}? | |
| compiler/typeAt | {documentId, pos} | type | 语义高亮 |
| compiler/semanticLens | {documentId} | fn 级 relations | |
| compiler/architecture | {projectId} | 模块依赖图 | |
| compiler/refresh(通知→) | {documentId, revision} | [Diagnostic] | 编辑后增量 |

## dev.*(提供行动;有副作用;懒启动)

| 方法 | params | result | 说明 |
|---|---|---|---|
| dev/build | {projectId} | {logs, diagnostics} | |
| dev/run | {projectId} | {stdout, stderr, exit} | 拉起 target |
| dev/test | {projectId} | 聚合用例结果 | 多语言聚合 |
| dev/buildGraph | {projectId} | BuildGraph | |
| dev/link | {projectId} | artifact | 跨语言产物链接 |
| dev/format | {documentId} | text | 写盘走 workspace |
| dev/debug | {projectId} | 会话 | P3 |
| dev/package | {projectId} | — | P2 |

## ai.*(AI 负责决策;Aine 实现)

| 方法 | params | result |
|---|---|---|
| ai/ask | {context, prompt} | text |
| ai/explain | {diagnosticId 或 pos} | text |
| ai/fix | {diagnosticId, documentId} | DiffSet |
| ai/task/start | {goal, scope} | Task |
| ai/task/step | {taskId, action} | Task |
| ai/task/review | {diffSetId} | review |
| ai/applyDecision | {diffSetId, decision:"accept|reject|accept-file", file?} | Revision |
| ai/agent | {goal} | 会话(P2) |

## 冲突与事件
- workspace.edit/applyDiff 校验 based_on_revision;过期 → error {code:"CONFLICT", current_revision}。
- 编译器状态由 revision 事件驱动;无轮询。
