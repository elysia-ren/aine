#!/usr/bin/env python3
# 生成 Std Lib Reference：从 stdlib/*.aine 提取函数签名 → 12 项属性文档
import io, os, re

def extract(fn):
    src = io.open(fn, encoding='utf-8').read()
    apis = []
    for m in re.finditer(r'^fn ([a-z_0-9]+)\(([^)]*)\)\s*(->\s*([^\{]+))?', src, re.M):
        name, params, ret = m.group(1), m.group(2), (m.group(4) or 'void').strip()
        plist = [p.strip() for p in params.split(',') if p.strip()]
        apis.append((name, plist, ret))
    return apis

MODS = ['collections', 'io', 'json', 'math', 'path', 'strutil', 'time', 'db', 'http']

out = ['# Aine 标准库参考（Std Lib Reference）', '',
       '> 每个 API 的 12 项属性：用途 / 签名 / 参数 / 返回 / 示例 / 错误 / 性能 / 线程安全 /',
       '> C 映射 / 所有权 / 注意事项 / 版本。', '']

for mod in MODS:
    fn = f'stdlib/{mod}.aine'
    apis = extract(fn) if os.path.exists(fn) else []
    out.append(f'## 模块 {mod}')
    out.append('')
    if not apis:
        out.append(f'- `{mod}`:解释器内建模块（见下文“内建模块”）。')
        out.append('')
        continue
    for name, plist, ret in apis:
        args = ', '.join(plist)
        out += [
            f'### {mod}.{name}({args}) -> {ret}',
            '',
            f'| 属性 | 内容 |',
            f'|---|---|',
            f'| 用途 | （待补：{name} 的用途说明） |',
            f'| 签名 | `fn {name}({args}) -> {ret}` |',
            f'| 参数 | {args or "无"} |',
            f'| 返回 | {ret} |',
            f'| 示例 | （待补） |',
            f'| 错误 | 无（纯函数） |',
            f'| 性能 | O(1) / O(n)（待补） |',
            f'| 线程安全 | 纯函数，无共享状态 |',
            f'| C 映射 | （待补） |',
            f'| 所有权 | 按值传递/返回（值语义） |',
            f'| 注意事项 | （待补） |',
            f'| 版本 | 0.1.0 |',
            '',
        ]

out += [
    '## 内建模块 db / http（解释器运行）',
    '',
    '### db.init(path) -> Result<(), String>',
    '| 属性 | 内容 |',
    '|---|---|',
    '| 用途 | 打开/创建数据库文件（JSONL 行存储） |',
    '| 签名 | `db.init(path: String)` |',
    '| 参数 | path：数据库文件路径 |',
    '| 返回 | Ok(()) 或 Err(消息) |',
    '| 示例 | `db.init("account.db")?` |',
    '| 错误 | 文件不可写时 Err |',
    '| 性能 | O(1) |',
    '| 线程安全 | 仅解释器内建（原生需 C 扩展） |',
    '| C 映射 | 无（原生待 C 扩展） |',
    '| 所有权 | 按值 |',
    '| 注意事项 | 每行 JSON 对象含 __table 字段 |',
    '| 版本 | 0.1.0 |',
    '',
    '### db.insert(table, record) -> Result<(), String>',
    '| 属性 | 内容 |',
    '|---|---|',
    '| 用途 | 追加一行（结构体序列化 JSON） |',
    '| 签名 | `db.insert(table: String, record: struct)` |',
    '| 参数 | table：表名；record：结构体值 |',
    '| 返回 | Ok(()) 或 Err |',
    '| 示例 | `db.insert("records", Record { id: 1 })` |',
    '| 错误 | 文件写入失败时 Err |',
    '| 性能 | O(1) 追加 |',
    '| 线程安全 | 仅解释器 |',
    '| C 映射 | 无 |',
    '| 所有权 | 按值 |',
    '| 注意事项 | 结构体字段序列化为 JSON 键值 |',
    '| 版本 | 0.1.0 |',
    '',
    '### db.query(sql) -> Result<Vec<Map<String, String>>, String>',
    '| 属性 | 内容 |',
    '|---|---|',
    '| 用途 | SQL 子集查询（SELECT/FROM/ORDER BY DESC） |',
    '| 签名 | `db.query(sql: String)` |',
    '| 参数 | sql：`SELECT * FROM t [ORDER BY col [DESC]]` |',
    '| 返回 | 行列表（列名 → 值字符串的 Map） |',
    '| 示例 | `let rows = db.query("SELECT * FROM records ORDER BY time DESC")?` |',
    '| 错误 | 表不存在返回空列表（不报错） |',
    '| 性能 | O(行数) 全表扫描 |',
    '| 线程安全 | 仅解释器 |',
    '| C 映射 | 无 |',
    '| 所有权 | 按值（行值字符串化） |',
    '| 注意事项 | 值统一为字符串，需显式转换（to_i32/to_f64） |',
    '| 版本 | 0.1.0 |',
    '',
    '### db.delete(table, id) -> Result<(), String>',
    '| 属性 | 内容 |',
    '|---|---|',
    '| 用途 | 按 id 删除行 |',
    '| 签名 | `db.delete(table: String, id: i32)` |',
    '| 参数 | table：表名；id：行 id |',
    '| 返回 | Ok(()) |',
    '| 示例 | `db.delete("records", id)?` |',
    '| 错误 | 文件不可写时 Err |',
    '| 性能 | O(行数) |',
    '| 线程安全 | 仅解释器 |',
    '| C 映射 | 无 |',
    '| 所有权 | 按值 |',
    '| 注意事项 | 重写文件（非原地删除） |',
    '| 版本 | 0.1.0 |',
    '',
    '### http.get(url) -> Result<String, String>',
    '| 属性 | 内容 |',
    '|---|---|',
    '| 用途 | HTTP GET 请求（std::net） |',
    '| 签名 | `http.get(url: String)` |',
    '| 参数 | url：`http://host[:port]/path` |',
    '| 返回 | Ok(body) 或 Err(消息) |',
    '| 示例 | `let r = http.get("http://127.0.0.1:8765/a.txt")` |',
    '| 错误 | 连接失败/非 200 时 Err |',
    '| 性能 | 网络往返 + 响应大小 |',
    '| 线程安全 | 仅解释器（原生需 C 扩展） |',
    '| C 映射 | 无 |',
    '| 所有权 | 按值（body 字符串） |',
    '| 注意事项 | HTTP/1.0 连接关闭即响应尾 |',
    '| 版本 | 0.1.0 |',
    '',
]
io.open('Aine_Std_Lib_Reference.md', 'w', encoding='utf-8', newline='\n').write('\n'.join(out) + '\n')
print('generated Aine_Std_Lib_Reference.md')
