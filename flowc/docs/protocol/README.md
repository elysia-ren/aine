# Tooling Protocol v1

冻结于 2026-09-03(AINE_IDE_ARCH v3.3)。P0 传输 = JSON-RPC 2.0 风格逐行 JSON over stdio;
传输可插拔(socket/pipe/TCP 后续),协议与传输解耦。

- [语义对象](objects.md)
- [方法表](methods.md)

## 版本与纪律
- 请求/响应带 `"v":1`;语义对象只加字段不改义;新方法向后兼容。
- UI 永不直接读源文件做语义;AI 永不直接写盘(仅 DiffSet→workspace.applyDiff)。
- 坐标一律表面坐标(用户所见);Source Map 由 Compiler 内部维护。

## 实现状态(Track)
- [x] Std API 宿主实现(read_line/env/sleep/json_encode/json_decode/http_request)
- [x] alinterp 同步 + mini_std 双引擎一致
- [ ] compiler_service.aine(服务循环 + 方法分派)
- [ ] 查询 API(definition/references/type_at/hover)
- [ ] 容错解析
- [ ] 自举原生服务 exe
