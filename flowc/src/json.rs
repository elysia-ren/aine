//! 最小 JSON 编解码(Std API: json_encode/json_decode,零依赖)
//! 语义见 docs/STD_API_SPEC.md §2:
//! - encode: Int/Float/Bool/String/Vec/Map/None(null)/Unit(null);
//!   Err(Result) 报错; Struct/Variant/闭包等不支持 → 运行时错误
//! - decode: null→Option(false), 数字按形态 Int/Float, 对象→Map

use crate::interp::Value;

pub fn encode(v: &Value) -> Result<String, String> {
    let mut out = String::new();
    encode_into(v, &mut out)?;
    Ok(out)
}

fn encode_into(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) => out.push_str(&fmt_f64(*f)),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Str(s) => encode_str(s, out),
        Value::Unit => out.push_str("null"),
        Value::Option { some: false, .. } => out.push_str("null"),
        Value::Option { some: true, value } => encode_into(value, out)?,
        Value::Result { ok: true, value } => encode_into(value, out)?,
        Value::Result { ok: false, .. } => {
            return Err("json_encode: Err 值无法序列化".to_string())
        }
        Value::Vec(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_into(it, out)?;
            }
            out.push(']');
        }
        Value::Tuple(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_into(it, out)?;
            }
            out.push(']');
        }
        Value::Map(entries) => {
            out.push('{');
            for (i, (k, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_str(k, out);
                out.push(':');
                encode_into(val, out)?;
            }
            out.push('}');
        }
        other => {
            return Err(format!(
                "json_encode: 不支持的类型 {}",
                other.type_name()
            ))
        }
    }
    Ok(())
}

/// f64 → JSON 数字: 整数值输出整数字样, 其余最短表示(与 al_fnum 精神一致)
fn fmt_f64(f: f64) -> String {
    if f == f.trunc() && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        let s = format!("{}", f);
        s
    }
}

fn encode_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---------------- decode ----------------

pub fn decode(s: &str) -> Result<Value, String> {
    let bytes: Vec<char> = s.chars().collect();
    let mut p = Parser {
        cs: &bytes,
        i: 0,
    };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.i < p.cs.len() {
        return Err(format!("json_decode: 第 {} 字符后有残留", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    cs: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.cs.len() && self.cs[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.cs.get(self.i).copied()
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            None => Err("json_decode: 输入为空".to_string()),
            Some('n') => {
                self.expect("null")?;
                Ok(Value::Option {
                    some: false,
                    value: Box::new(Value::Unit),
                })
            }
            Some('t') => {
                self.expect("true")?;
                Ok(Value::Bool(true))
            }
            Some('f') => {
                self.expect("false")?;
                Ok(Value::Bool(false))
            }
            Some('"') => Ok(Value::Str(self.parse_string()?.into())),
            Some('[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(']') {
                    self.i += 1;
                    return Ok(Value::Vec(items.into()));
                }
                loop {
                    items.push(self.parse_value()?);
                    self.skip_ws();
                    match self.peek() {
                        Some(',') => {
                            self.i += 1;
                        }
                        Some(']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("json_decode: 数组缺少 ]".to_string()),
                    }
                }
                Ok(Value::Vec(items.into()))
            }
            Some('{') => {
                self.i += 1;
                let mut entries = Vec::new();
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.i += 1;
                    return Ok(Value::Map(entries));
                }
                loop {
                    self.skip_ws();
                    if self.peek() != Some('"') {
                        return Err("json_decode: 对象键必须为字符串".to_string());
                    }
                    let key = self.parse_string()?;
                    self.skip_ws();
                    if self.peek() != Some(':') {
                        return Err("json_decode: 对象缺少 :".to_string());
                    }
                    self.i += 1;
                    let val = self.parse_value()?;
                    entries.push((key, val));
                    self.skip_ws();
                    match self.peek() {
                        Some(',') => {
                            self.i += 1;
                        }
                        Some('}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("json_decode: 对象缺少 }".to_string()),
                    }
                }
                Ok(Value::Map(entries))
            }
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("json_decode: 非法字符 '{}'", c)),
        }
    }

    fn expect(&mut self, lit: &str) -> Result<(), String> {
        for ch in lit.chars() {
            if self.cs.get(self.i).copied() != Some(ch) {
                return Err(format!("json_decode: 期望 {}", lit));
            }
            self.i += 1;
        }
        Ok(())
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.peek() != Some('"') {
            return Err("json_decode: 期望字符串".to_string());
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err("json_decode: 字符串未闭合".to_string()),
                Some('"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.i += 1;
                    match self.peek() {
                        Some('"') => {
                            out.push('"');
                            self.i += 1;
                        }
                        Some('\\') => {
                            out.push('\\');
                            self.i += 1;
                        }
                        Some('/') => {
                            out.push('/');
                            self.i += 1;
                        }
                        Some('n') => {
                            out.push('\n');
                            self.i += 1;
                        }
                        Some('r') => {
                            out.push('\r');
                            self.i += 1;
                        }
                        Some('t') => {
                            out.push('\t');
                            self.i += 1;
                        }
                        Some('b') => {
                            out.push('\u{08}');
                            self.i += 1;
                        }
                        Some('f') => {
                            out.push('\u{0c}');
                            self.i += 1;
                        }
                        Some('u') => {
                            self.i += 1;
                            let hi = self.parse_hex4()?;
                            // 代理对: 高代理后紧跟 \uXXXX 低代理 → 组合
                            if (0xD800..0xDC00).contains(&hi)
                                && self.cs.get(self.i).copied() == Some('\\')
                                && self.cs.get(self.i + 1).copied() == Some('u')
                            {
                                let save = self.i;
                                self.i += 2;
                                let lo = self.parse_hex4()?;
                                if (0xDC00..0xE000).contains(&lo) {
                                    let c = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                    if let Some(ch) = char::from_u32(c) {
                                        out.push(ch);
                                        continue;
                                    }
                                }
                                self.i = save; // 非低代理: 回退按单字符
                            }
                            if let Some(ch) = char::from_u32(hi) {
                                out.push(ch);
                            } else {
                                return Err("json_decode: 非法 \\u 转义".to_string());
                            }
                        }
                        other => {
                            return Err(format!(
                                "json_decode: 非法转义 \\{}",
                                other.unwrap_or('?')
                            ))
                        }
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self.peek().ok_or("json_decode: \\u 需 4 位十六进制")?;
            let d = c.to_digit(16).ok_or("json_decode: 非法十六进制")?;
            v = v * 16 + d;
            self.i += 1;
        }
        Ok(v)
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.i;
        let mut is_float = false;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                is_float = true;
                self.i += 1;
            } else {
                break;
            }
        }
        let text: String = self.cs[start..self.i].iter().collect();
        if text.is_empty() || text == "-" {
            return Err("json_decode: 非法数字".to_string());
        }
        if is_float {
            text.parse::<f64>()
                .map(Value::Float)
                .map_err(|_| "json_decode: 非法浮点数".to_string())
        } else {
            text.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| "json_decode: 整数溢出".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(v: Value) -> Value {
        decode(&encode(&v).unwrap()).unwrap()
    }

    #[test]
    fn json_roundtrip_scalars() {
        assert!(matches!(rt(Value::Int(42)), Value::Int(42)));
        assert!(matches!(rt(Value::Bool(true)), Value::Bool(true)));
        assert!(matches!(
            rt(Value::Str("héllo\n\"世界\"".into())),
            Value::Str(_)
        ));
        assert!(matches!(
            rt(Value::Float(3.5)),
            Value::Float(f) if (f - 3.5).abs() < 1e-9
        ));
    }

    #[test]
    fn json_roundtrip_container() {
        let v = Value::Vec(
            vec![
                Value::Int(1),
                Value::Str("a\"b".into()),
                Value::Map(vec![("k".to_string(), Value::Bool(false))]),
            ]
            .into(),
        );
        let back = rt(v);
        match back {
            Value::Vec(items) => {
                assert_eq!(items.len(), 3);
                assert!(matches!(&items[2], Value::Map(e) if e.len() == 1));
            }
            _ => panic!("expected vec"),
        }
    }

    #[test]
    fn json_surrogate_pair() {
        let s = r#""\ud83d\ude00""#;
        match decode(s).unwrap() {
            Value::Str(t) => assert_eq!(&*t, "😀"),
            _ => panic!("expected str"),
        }
    }
}
