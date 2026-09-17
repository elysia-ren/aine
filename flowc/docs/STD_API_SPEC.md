# Aine Std API 规格 v1(现状对照 + 冻结签名)

> 状态:冻结(P0)。实现方:宿主(Rust)= 临时 backend;终态 = alee C 模板自实现。
> 快照(2026-09-04 v1.1):服务化档已全量实现并双引擎一致;UI 档已实现(原生控件 + 自绘运行时双轨);新增 file_list。
> 关联:docs/AINE_IDE_ARCH.md v3.6 §4.1

## 1. 现状对照表(T01)

| 能力 | 宿主 interp.rs call_named_value | alinterp.aine fname 表 | hir.rs BUILTIN_GLOBALS | alee C 模板 | 状态 |
|---|---|---|---|---|---|
| print | ✓ | ✓ | ✓ | ✓ | 已有 |
| assert | ✓(失败 panic) | ✓ | ✓ | ✓(abort) | 已有 |
| now | ✓(秒) | ✓ | ✓ | — | 已有 |
| read_file | ✓(5 次重试,错误前缀返回) | ✓ | ✓ | ✓(fopen rb) | 已有 |
| write_file | ✓(Bool) | ✓ | ✓ | — | 已有 |
| append_file | ✓(Bool) | ✗ | ✓ | — | 宿主有/alinterp 缺 |
| file_exists | ✓ | ✓ | ✓ | — | 已有 |
| cli_args | 全局注入 | 全局注入 | ✓ | ✓(al_cli_args) | 已有 |
| run_ui | 文本渲染桩 | AUnit | ✓ | — | 桩(E4 真渲染) |
| db/http | Nil 桩/方法面 | 无 | ✓ | — | 桩 |
| read_line | ✗ | ✗ | ✗ | ✗ | **缺失(本规格)** |
| env | ✗ | ✗ | ✗ | ✗ | **缺失** |
| sleep | ✗ | ✗ | ✗ | ✗ | **缺失** |
| json_encode/decode | ✗ | ✗ | ✗ | ✗ | **缺失** |
| http_request | ✗(仅 http 方法桩) | ✗ | ✗ | ✗ | **缺失** |
| file_list | ✓(目录平铺) | ✓ | — | ✗ | 已实现(S0) |
| window_new/label_new/button_new/set_text/get_text/button_onclick/ui_run | ✗(run_ui 文本桩) | 报"仅原生" | ✓ | ✓ | 已实现(E4) |
| input_new/listbox_new/listbox_add/clear/selected | ✗ | 报"仅原生" | ✓ | ✗(原生控件轨) | 已实现(S1b) |
| rt_window/rt_panel/rt_text/rt_button/rt_list/rt_set_comp_text/rt_run | 发射器生成 | 报"仅原生" | ✓ | ✓ | 已实现(S1e 自绘) |

注:Cargo.toml 零第三方依赖;json/http 手写实现(std::net)。

## 2. 冻结签名(T02)

```text
read_line() -> String
    // 从 stdin 读一行(去掉行尾 \n / \r\n);EOF 返回 ""
    // 测试可注入: Interp.input_lines 优先消费

env(name: String) -> String
    // 读环境变量;未设置返回 ""

sleep(ms: i32)
    // 阻塞 ms 毫秒

json_encode(v) -> String
    // 支持: i32/f64/bool/String/Vec/Map<String,任意> / None(null)
    // 不支持的类型(Struct/Variant/闭包等)→ 运行时错误(与 assert 同款终止)
    // f64: 整数形态输出整数文本(与 al_fnum 语义一致)

json_decode(s: String) -> AVal
    // null → None; true/false → bool; 整数 → i32; 小数/指数 → f64;
    // 字符串(UTF-8 转义还原) → String; 数组 → Vec; 对象 → Map<String,任意>
    // 非法 JSON → 运行时错误

http_request(method: String, url: String, headers: Map<String, String>,
             body: String, on_chunk: (String) => Unit) -> i32
    // 阻塞 HTTP/1.1; 返回 status 码; 网络/协议错误返回 0
    // on_chunk: 每收到一块文本(按行切分, 含 SSE 行/心跳空行)调用一次;
    //           支持闭包或函数名(String, 查全局函数)
    // 读模式: Transfer-Encoding: chunked → 逐 chunk;
    //         Content-Length → 读满后按行回调; 否则读到连接关闭(流式 SSE)
```

## 3. 双引擎约定

- 宿主(Rust interp)与 alinterp 均提供上述调用;alinterp 对 read_line/env/sleep/
  json_encode/json_decode 直接透传宿主 builtin(http_request 透传,回调需原生支持)。
- 行为以宿主为准,alinterp 逐字节一致(经 mini_std 验证)。
- alee C 模板实现 = 收尾 F(T64),不在 P0。
