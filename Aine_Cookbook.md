# Aine Cookbook(12 个可运行示例)

> 每个示例均为可运行程序(解释器与原生输出一致)。来源:examples/ 与 Book。

## 1. Hello World

```aine
fn main() {
    print("你好,Aine")
}
```

## 2. 斐波那契

```aine
fn fib(n: i32) -> i32 {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(fib(10))    // 55
}
```

## 3. 字符串处理

```aine
fn main() {
    let s = "hello, aine"
    print(s.len())          // 11
    print(s[0])             // h
    print(s[7..11])         // aine
}
```

## 4. 集合求和

```aine
fn main() {
    var v: Vec<i32> = []
    v.push(1)
    v.push(2)
    v.push(3)
    var total = 0
    for x in v {
        total += x
    }
    print(total)            // 6
}
```

## 5. Map 计数

```aine
fn main() {
    let m = Map.new()
    for w in ["a", "b", "a"] {
        let cur = m.get(w).unwrap_or("0").to_i32()
        m.set(w, f"{cur + 1}")
    }
    print(m.get("a").unwrap_or("0"))    // 2
}
```

## 6. 枚举与模式匹配

```aine
enum Shape {
    Circle(f64)
    Rect(f64, f64)
}

fn area(s: Shape) -> f64 {
    match s {
        Circle(r) { 3.14 * r * r }
        Rect(w, h) { w * h }
    }
}

fn main() {
    print(area(Circle(2.0)))    // 12.56
}
```

## 7. Option 处理

```aine
fn describe(o: Option<i32>) -> String {
    match o {
        Some(x) { f"got {x}" }
        None { "none" }
    }
}

fn main() {
    print(describe(Some(5)))    // got 5
    print(describe(None))       // none
}
```

## 8. 错误传播

```aine
fn parse_num(s: String) -> Result<i32, String> {
    let n = s.to_i32()
    if n > 0 {
        Ok(n)
    } else {
        Err("需要正数")
    }
}

fn main() -> Result<(), String> {
    let n = parse_num("42")?
    print(n)                    // 42
    return Ok(())
}
```

## 9. 闭包与高阶函数

```aine
fn main() {
    let f = (x: i32) => x * 2
    print(f(21))                // 42
    let nums = [1, 2, 3]
    print(nums.iter().map(x => x * 10).sum())   // 60
}
```

## 10. 模块

```aine
module math;    // math.aine: fn double(x: i32) -> i32

fn main() {
    print(double(21))           // 42
}
```

## 11. 并发任务

```aine
fn main() {
    go {
        print("后台任务")
    }
    ui {
        print("UI 操作")
    }
    print("主流程")
}
```

## 12. 数据库(记账本数据层)

```aine
struct Record {
    id: i32
    desc: String
    amount: f64
    time: i32
}

fn main() -> Result<(), String> {
    db.init("account.db")?
    db.insert("records", Record { id: 1 desc: "a" amount: 1.5 time: 100 })?
    let rows = db.query("SELECT * FROM records ORDER BY time DESC")?
    print(f"rows={rows.len()}")
    return Ok(())
}
```

---

> 验证:`aine run <示例>`;完整应用见 examples/account_book.aine(记账本端到端)。
