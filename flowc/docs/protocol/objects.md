# Tooling Protocol 语义对象 v1(冻结)

> 版本:v1。规则:可加字段、不可改义;请求带 "v":1。
> 关联:docs/AINE_IDE_ARCH.md v3.3 §4.2 / docs/STD_API_SPEC.md

所有对象以 JSON 表示(经 Std API json_encode/json_decode 传输)。

```jsonc
// 位置与范围(坐标 = 表面坐标, 用户所见; 0 起始行/列)
Pos     { "line": int, "col": int }
Range   { "start": Pos, "end": Pos }

// 诊断: 结构化、语言无关; 文字由渲染层按表面语言输出(code 术语头保留)
Diagnostic {
  "id": string,
  "workspaceId": string, "projectId": string,
  "language": "aine|c|rust|…",
  "file": string,
  "range": Range,
  "severity": "error|warn|info",
  "code": "E0312",
  "message_key": "type_mismatch",
  "args": { "expected": "…", "found": "…" },
  "code_snippet": string?
}

// 符号: language 必填; relations 见下
Symbol {
  "id": string, "language": string,
  "kind": "fn|struct|enum|field|param|var|module|extern|…",
  "name": string,
  "file": string, "range": Range, "decl_range": Range,
  "type": string?,
  "children": [Symbol]?,
  "relations": [
    { "kind": "calls|owns|borrows|returns|implements|imports|exports|ffi_binding|links_to|generated_from",
      "to": SymbolRef }
  ]?
}

Revision { "id": string, "documentId": string, "base": int }

// 工作副本: 用户正在编辑尚未落盘的内容; UI 不持有第二份权威源码
Document {
  "id": string, "file": string,
  "revision": int, "dirty": bool, "text": string,
  "based_on_revision": int,
  "language": "aine|c|rust|…",
  "surface_language": "zh-CN|en|…"     // 仅 language=aine 有效
}

Project {
  "id": string,
  "default_language": string,
  "supported_languages": [string],
  "languages": [string], "targets": [Target], "dependencies": [Dependency],
  "toolchains": [string]
}

BuildGraph {
  "nodes": [ { "target": string, "language": string, "sources": [string],
               "deps": [string], "artifact": string } ],
  "edges": [ { "from": string, "to": string, "kind": "source|link|runtime" } ]
}

Dependency {
  "language": string,
  "kind": "package|crate|lib|runtime",
  "source": string, "version": string?,
  "artifact": string?, "build": string?, "runtime": string?
}

DiffFile   { "file": string, "hunks": [ { "start": int, "lines_old": [string], "lines_new": [string] } ] }
DiffSet    { "id": string, "files": [DiffFile], "base_revision": int,
             "status": "pending|checked|applied|rejected" }

Task {
  "id": string, "goal": string,
  "scope": { "files": [string], "symbols": [string] },
  "steps": [ { "title": string, "status": "todo|doing|done|failed",
               "summary": string?, "files": [string]?, "model_hint": string? } ],
  "diffs": [DiffSet],
  "compiler": { "errors": int, "warnings": int },
  "state": "planning|running|awaiting_review|done"
}
```

## 版本纪律
- 所有请求/响应顶层带 `"v": 1`。
- 语义对象只加字段不改义;新方法向后兼容;协议表变更须升 v。
