# Aine 标准库参考（Std Lib Reference）

> 每个 API 的 12 项属性：用途 / 签名 / 参数 / 返回 / 示例 / 错误 / 性能 / 线程安全 /
> C 映射 / 所有权 / 注意事项 / 版本。

## 模块 collections

### collections.range_vec(n: i32) -> Vec<i32>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：range_vec 的用途说明） |
| 签名 | `fn range_vec(n: i32) -> Vec<i32>` |
| 参数 | n: i32 |
| 返回 | Vec<i32> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.vec_reverse(v: Vec<i32>) -> Vec<i32>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：vec_reverse 的用途说明） |
| 签名 | `fn vec_reverse(v: Vec<i32>) -> Vec<i32>` |
| 参数 | v: Vec<i32> |
| 返回 | Vec<i32> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.vec_max(v: Vec<i32>) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：vec_max 的用途说明） |
| 签名 | `fn vec_max(v: Vec<i32>) -> i32` |
| 参数 | v: Vec<i32> |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.vec_concat(a: Vec<i32>, b: Vec<i32>) -> Vec<i32>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：vec_concat 的用途说明） |
| 签名 | `fn vec_concat(a: Vec<i32>, b: Vec<i32>) -> Vec<i32>` |
| 参数 | a: Vec<i32>, b: Vec<i32> |
| 返回 | Vec<i32> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.sort_ints(v: Vec<i32>) -> Vec<i32>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：sort_ints 的用途说明） |
| 签名 | `fn sort_ints(v: Vec<i32>) -> Vec<i32>` |
| 参数 | v: Vec<i32> |
| 返回 | Vec<i32> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.sum_ints(v: Vec<i32>) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：sum_ints 的用途说明） |
| 签名 | `fn sum_ints(v: Vec<i32>) -> i32` |
| 参数 | v: Vec<i32> |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### collections.contains_int(v: Vec<i32>, target: i32) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：contains_int 的用途说明） |
| 签名 | `fn contains_int(v: Vec<i32>, target: i32) -> bool` |
| 参数 | v: Vec<i32>, target: i32 |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 io

### io.write_file(path: String, content: String) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：write_file 的用途说明） |
| 签名 | `fn write_file(path: String, content: String) -> bool` |
| 参数 | path: String, content: String |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### io.append_file(path: String, content: String) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：append_file 的用途说明） |
| 签名 | `fn append_file(path: String, content: String) -> bool` |
| 参数 | path: String, content: String |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### io.file_exists(path: String) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：file_exists 的用途说明） |
| 签名 | `fn file_exists(path: String) -> bool` |
| 参数 | path: String |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 json

### json.is_digit(c: String) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：is_digit 的用途说明） |
| 签名 | `fn is_digit(c: String) -> bool` |
| 参数 | c: String |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.skip_ws(src: String, i: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：skip_ws 的用途说明） |
| 签名 | `fn skip_ws(src: String, i: i32) -> i32` |
| 参数 | src: String, i: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.parse_str_raw(src: String, i: i32) -> Result<SResult, String>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：parse_str_raw 的用途说明） |
| 签名 | `fn parse_str_raw(src: String, i: i32) -> Result<SResult, String>` |
| 参数 | src: String, i: i32 |
| 返回 | Result<SResult, String> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.parse_num(src: String, i: i32) -> Result<PResult, String>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：parse_num 的用途说明） |
| 签名 | `fn parse_num(src: String, i: i32) -> Result<PResult, String>` |
| 参数 | src: String, i: i32 |
| 返回 | Result<PResult, String> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.parse_value(src: String, i: i32) -> Result<PResult, String>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：parse_value 的用途说明） |
| 签名 | `fn parse_value(src: String, i: i32) -> Result<PResult, String>` |
| 参数 | src: String, i: i32 |
| 返回 | Result<PResult, String> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.parse_arr(src: String, i: i32) -> Result<PResult, String>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：parse_arr 的用途说明） |
| 签名 | `fn parse_arr(src: String, i: i32) -> Result<PResult, String>` |
| 参数 | src: String, i: i32 |
| 返回 | Result<PResult, String> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.parse_obj(src: String, i: i32) -> Result<PResult, String>

| 属性 | 内容 |
|---|---|
| 用途 | （待补：parse_obj 的用途说明） |
| 签名 | `fn parse_obj(src: String, i: i32) -> Result<PResult, String>` |
| 参数 | src: String, i: i32 |
| 返回 | Result<PResult, String> |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.join(items: Vec<String>, sep: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：join 的用途说明） |
| 签名 | `fn join(items: Vec<String>, sep: String) -> String` |
| 参数 | items: Vec<String>, sep: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.escape_str(s: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：escape_str 的用途说明） |
| 签名 | `fn escape_str(s: String) -> String` |
| 参数 | s: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### json.stringify(j: Json) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：stringify 的用途说明） |
| 签名 | `fn stringify(j: Json) -> String` |
| 参数 | j: Json |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 math

### math.abs_i32(n: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：abs_i32 的用途说明） |
| 签名 | `fn abs_i32(n: i32) -> i32` |
| 参数 | n: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.abs_f64(n: f64) -> f64

| 属性 | 内容 |
|---|---|
| 用途 | （待补：abs_f64 的用途说明） |
| 签名 | `fn abs_f64(n: f64) -> f64` |
| 参数 | n: f64 |
| 返回 | f64 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.min_i32(a: i32, b: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：min_i32 的用途说明） |
| 签名 | `fn min_i32(a: i32, b: i32) -> i32` |
| 参数 | a: i32, b: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.max_i32(a: i32, b: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：max_i32 的用途说明） |
| 签名 | `fn max_i32(a: i32, b: i32) -> i32` |
| 参数 | a: i32, b: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.clamp_i32(n: i32, lo: i32, hi: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：clamp_i32 的用途说明） |
| 签名 | `fn clamp_i32(n: i32, lo: i32, hi: i32) -> i32` |
| 参数 | n: i32, lo: i32, hi: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.gcd(a: i32, b: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：gcd 的用途说明） |
| 签名 | `fn gcd(a: i32, b: i32) -> i32` |
| 参数 | a: i32, b: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### math.pow_i32(base: i32, exp: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：pow_i32 的用途说明） |
| 签名 | `fn pow_i32(base: i32, exp: i32) -> i32` |
| 参数 | base: i32, exp: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 path

### path.path_join(a: String, b: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：path_join 的用途说明） |
| 签名 | `fn path_join(a: String, b: String) -> String` |
| 参数 | a: String, b: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### path.path_dir(p: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：path_dir 的用途说明） |
| 签名 | `fn path_dir(p: String) -> String` |
| 参数 | p: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### path.path_base(p: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：path_base 的用途说明） |
| 签名 | `fn path_base(p: String) -> String` |
| 参数 | p: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### path.path_ext(p: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：path_ext 的用途说明） |
| 签名 | `fn path_ext(p: String) -> String` |
| 参数 | p: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 strutil

### strutil.reverse(s: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：reverse 的用途说明） |
| 签名 | `fn reverse(s: String) -> String` |
| 参数 | s: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### strutil.count_char(s: String, target: String) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：count_char 的用途说明） |
| 签名 | `fn count_char(s: String, target: String) -> i32` |
| 参数 | s: String, target: String |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### strutil.pad_left(s: String, width: i32, pad: String) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：pad_left 的用途说明） |
| 签名 | `fn pad_left(s: String, width: i32, pad: String) -> String` |
| 参数 | s: String, width: i32, pad: String |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### strutil.starts_with_digit(s: String) -> bool

| 属性 | 内容 |
|---|---|
| 用途 | （待补：starts_with_digit 的用途说明） |
| 签名 | `fn starts_with_digit(s: String) -> bool` |
| 参数 | s: String |
| 返回 | bool |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 time

### time.now_secs() -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：now_secs 的用途说明） |
| 签名 | `fn now_secs() -> i32` |
| 参数 | 无 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### time.elapsed_since(start: i32) -> i32

| 属性 | 内容 |
|---|---|
| 用途 | （待补：elapsed_since 的用途说明） |
| 签名 | `fn elapsed_since(start: i32) -> i32` |
| 参数 | start: i32 |
| 返回 | i32 |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

### time.fmt_duration(secs: i32) -> String

| 属性 | 内容 |
|---|---|
| 用途 | （待补：fmt_duration 的用途说明） |
| 签名 | `fn fmt_duration(secs: i32) -> String` |
| 参数 | secs: i32 |
| 返回 | String |
| 示例 | （待补） |
| 错误 | 无（纯函数） |
| 性能 | O(1) / O(n)（待补） |
| 线程安全 | 纯函数，无共享状态 |
| C 映射 | （待补） |
| 所有权 | 按值传递/返回（值语义） |
| 注意事项 | （待补） |
| 版本 | 0.1.0 |

## 模块 db

- `db`:解释器内建模块（见下文“内建模块”）。

## 模块 http

- `http`:解释器内建模块（见下文“内建模块”）。

## 内建模块 db / http（解释器运行）

### db.init(path) -> Result<(), String>
| 属性 | 内容 |
|---|---|
| 用途 | 打开/创建数据库文件（JSONL 行存储） |
| 签名 | `db.init(path: String)` |
| 参数 | path：数据库文件路径 |
| 返回 | Ok(()) 或 Err(消息) |
| 示例 | `db.init("account.db")?` |
| 错误 | 文件不可写时 Err |
| 性能 | O(1) |
| 线程安全 | 仅解释器内建（原生需 C 扩展） |
| C 映射 | 无（原生待 C 扩展） |
| 所有权 | 按值 |
| 注意事项 | 每行 JSON 对象含 __table 字段 |
| 版本 | 0.1.0 |

### db.insert(table, record) -> Result<(), String>
| 属性 | 内容 |
|---|---|
| 用途 | 追加一行（结构体序列化 JSON） |
| 签名 | `db.insert(table: String, record: struct)` |
| 参数 | table：表名；record：结构体值 |
| 返回 | Ok(()) 或 Err |
| 示例 | `db.insert("records", Record { id: 1 })` |
| 错误 | 文件写入失败时 Err |
| 性能 | O(1) 追加 |
| 线程安全 | 仅解释器 |
| C 映射 | 无 |
| 所有权 | 按值 |
| 注意事项 | 结构体字段序列化为 JSON 键值 |
| 版本 | 0.1.0 |

### db.query(sql) -> Result<Vec<Map<String, String>>, String>
| 属性 | 内容 |
|---|---|
| 用途 | SQL 子集查询（SELECT/FROM/ORDER BY DESC） |
| 签名 | `db.query(sql: String)` |
| 参数 | sql：`SELECT * FROM t [ORDER BY col [DESC]]` |
| 返回 | 行列表（列名 → 值字符串的 Map） |
| 示例 | `let rows = db.query("SELECT * FROM records ORDER BY time DESC")?` |
| 错误 | 表不存在返回空列表（不报错） |
| 性能 | O(行数) 全表扫描 |
| 线程安全 | 仅解释器 |
| C 映射 | 无 |
| 所有权 | 按值（行值字符串化） |
| 注意事项 | 值统一为字符串，需显式转换（to_i32/to_f64） |
| 版本 | 0.1.0 |

### db.delete(table, id) -> Result<(), String>
| 属性 | 内容 |
|---|---|
| 用途 | 按 id 删除行 |
| 签名 | `db.delete(table: String, id: i32)` |
| 参数 | table：表名；id：行 id |
| 返回 | Ok(()) |
| 示例 | `db.delete("records", id)?` |
| 错误 | 文件不可写时 Err |
| 性能 | O(行数) |
| 线程安全 | 仅解释器 |
| C 映射 | 无 |
| 所有权 | 按值 |
| 注意事项 | 重写文件（非原地删除） |
| 版本 | 0.1.0 |

### http.get(url) -> Result<String, String>
| 属性 | 内容 |
|---|---|
| 用途 | HTTP GET 请求（std::net） |
| 签名 | `http.get(url: String)` |
| 参数 | url：`http://host[:port]/path` |
| 返回 | Ok(body) 或 Err(消息) |
| 示例 | `let r = http.get("http://127.0.0.1:8765/a.txt")` |
| 错误 | 连接失败/非 200 时 Err |
| 性能 | 网络往返 + 响应大小 |
| 线程安全 | 仅解释器（原生需 C 扩展） |
| C 映射 | 无 |
| 所有权 | 按值（body 字符串） |
| 注意事项 | HTTP/1.0 连接关闭即响应尾 |
| 版本 | 0.1.0 |

