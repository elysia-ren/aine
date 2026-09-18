//! Language Veil（T48-T53）：Aine 表面语言双向渲染（Rust 侧镜像 examples/alrender.aine）。
//! 词级映射；跳过字符串与注释；en 为 canonical（恒等）。
//! IDE 用法：Tab.surface != 1 时显示 render_to(内容)，保存前 surface_to_canonical 写盘。

/// 语言剖面索引：0=zh-CN 1=en(canonical) 2=ja 3=de 4=fr 5=ru
pub const SURFACE_NAMES: [&str; 6] = ["中文", "EN", "日本語", "Deutsch", "Français", "Русский"];

pub struct Profile {
    pub code: &'static str,
    pub words: &'static [(&'static str, &'static str)], // (surface, canonical)
}

pub fn profile(idx: usize) -> Profile {
    let idx = idx.min(5);
    match idx {
        0 => Profile { code: "zh-CN", words: &[
            ("让", "let"), ("函数", "fn"), ("返回", "return"), ("如果", "if"), ("否则", "else"),
            ("循环", "while"), ("遍历", "for"), ("于", "in"), ("匹配", "match"),
            ("中断", "break"), ("继续", "continue"), ("结构", "struct"), ("枚举", "enum"),
            ("模块", "module"), ("导入", "import"), ("实现", "impl"), ("公开", "pub"), ("作为", "as"),
            ("真", "true"), ("假", "false"), ("打印", "print"),
        ]},
        1 => Profile { code: "en", words: &[] },
        2 => Profile { code: "ja", words: &[
            ("関数", "fn"), ("もし", "if"), ("それ以外", "else"), ("繰り返し", "while"),
            ("各", "for"), ("内", "in"), ("返す", "return"), ("真", "true"), ("偽", "false"),
            ("構造", "struct"), ("列挙", "enum"), ("輸入", "import"), ("使う", "use"), ("表示", "print"),
            ("中断", "break"), ("継続", "continue"), ("マッチ", "match"),
        ]},
        3 => Profile { code: "de", words: &[
            ("Funktion", "fn"), ("wenn", "if"), ("sonst", "else"), ("solange", "while"),
            ("für", "for"), ("in", "in"), ("zurück", "return"), ("wahr", "true"), ("falsch", "false"),
            ("Struktur", "struct"), ("Aufzählung", "enum"), ("importieren", "import"), ("benutzen", "use"), ("drucken", "print"),
            ("abbrechen", "break"), ("weiter", "continue"), ("Muster", "match"),
        ]},
        4 => Profile { code: "fr", words: &[
            ("fonction", "fn"), ("si", "if"), ("sinon", "else"), ("tantque", "while"),
            ("pour", "for"), ("dans", "in"), ("renvoyer", "return"), ("vrai", "true"), ("faux", "false"),
            ("structure", "struct"), ("énum", "enum"), ("importer", "import"), ("utiliser", "use"), ("afficher", "print"),
            ("interrompre", "break"), ("poursuivre", "continue"), ("motif", "match"),
        ]},
        _ => Profile { code: "ru", words: &[
            ("функция", "fn"), ("если", "if"), ("иначе", "else"), ("пока", "while"),
            ("для", "for"), ("в", "in"), ("вернуть", "return"), ("истина", "true"), ("ложь", "false"),
            ("структура", "struct"), ("перечисление", "enum"), ("импорт", "import"), ("использовать", "use"),
            ("печать", "print"), ("прервать", "break"), ("продолжить", "continue"), ("образец", "match"),
        ]},
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || !c.is_ascii() && c.is_alphabetic()
}

/// 词级双向变换（镜像 alrender.aine prof_transform）：
/// to_canonical=true 表面→canonical；false canonical→表面。
/// 跳过字符串 `"…"` 与 `//…\n` 注释。
pub fn transform(text: &str, idx: usize, to_canonical: bool) -> String {
    if idx == 1 || idx >= 6 { return text.to_string(); }
    let prof = profile(idx);
    let map = |run: &str| -> String {
        for (from, to) in prof.words {
            let (f, t) = if to_canonical { (*from, *to) } else { (*to, *from) };
            if run == f { return t.to_string(); }
        }
        run.to_string()
    };
    let mut out = String::with_capacity(text.len() + 64);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    let n = chars.len();
    let mut mode = 0u8; // 0=code 1=string 2=comment
    while i < n {
        let c = chars[i];
        if mode == 0 {
            if c == '"' { mode = 1; out.push(c); i += 1; continue; }
            if c == '/' && i + 1 < n && chars[i + 1] == '/' { mode = 2; out.push(c); i += 1; continue; }
            if is_word_char(c) {
                let mut j = i;
                while j < n && is_word_char(chars[j]) { j += 1; }
                let run: String = chars[i..j].iter().collect();
                out.push_str(&map(&run));
                i = j;
                continue;
            }
            out.push(c);
            i += 1;
        } else if mode == 1 {
            if c == '"' { mode = 0; }
            out.push(c);
            i += 1;
        } else {
            if c == '\n' { mode = 0; }
            out.push(c);
            i += 1;
        }
    }
    out
}

/// T51: 双向源映射 — 表面文本与 canonical 文本逐行对齐
/// 返回 Vec<(表面行号1基, canonical行号1基)>
pub fn source_map(surface_text: &str, canonical_text: &str) -> Vec<(usize, usize)> {
    let s_lines: Vec<&str> = surface_text.split('\n').collect();
    let c_lines: Vec<&str> = canonical_text.split('\n').collect();
    let mut map = Vec::new();
    let mut si = 0usize;
    let mut ci = 0usize;
    while si < s_lines.len() && ci < c_lines.len() {
        map.push((si + 1, ci + 1));
        let sn: String = s_lines[si].split_whitespace().collect();
        let cn: String = c_lines[ci].split_whitespace().collect();
        if sn == cn { si += 1; ci += 1; }
        else if sn.is_empty() { si += 1; }
        else if cn.is_empty() { ci += 1; }
        else { si += 1; ci += 1; }
    }
    while si < s_lines.len() { si += 1; map.push((si, ci.max(1))); }
    while ci < c_lines.len() { ci += 1; map.push((si.max(1), ci)); }
    map
}

/// canonical → 指定表面（显示用；en 恒等）
pub fn render_to(text: &str, idx: usize) -> String { transform(text, idx, false) }
/// 表面 → canonical（保存/编译前调用；en 恒等）
pub fn surface_to_canonical(text: &str, idx: usize) -> String { transform(text, idx, true) }

/// 该表面语言是否含此词（语法高亮着色用）
pub fn lang_has_word(idx: usize, w: &str) -> bool {
    if idx == 1 || idx >= 6 { return false; }
    profile(idx).words.iter().any(|(from, _)| *from == w)
}
