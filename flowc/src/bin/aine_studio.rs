// Aine Studio — 完整 IDE v3
use eframe::egui;
use std::io::Write;
use std::process::Command;
use std::path::PathBuf;

// ── 项目根目录 ──
fn find_project_root() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let mut dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    for _ in 0..5 {
        if dir.join("examples").is_dir()
            && std::fs::read_dir(dir.join("examples")).map(|mut d| d.next().is_some()).unwrap_or(false) {
            return dir;
        }
        if !dir.pop() { break; }
    }
    std::env::current_dir().unwrap_or_default()
}

// ── CJK 字体 ──
fn load_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // 微软雅黑优先（现代 UI 字形），simhei 兜底
    for font_path in [
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simhei.ttf",
    ] {
        if let Ok(data) = std::fs::read(font_path) {
            fonts.font_data.insert("cjk".into(),
                egui::FontData::from_owned(data).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                if let Some(f) = fonts.families.get_mut(&family) {
                    f.push("cjk".into());
                }
            }
            break;
        }
    }
    // 符号/emoji fallback：▾▸●○√⏳ 以及 📁🔍🌿🤖 等（雅黑缺字形会显示 □）
    if let Ok(data) = std::fs::read("C:/Windows/Fonts/seguiemj.ttf") {
        fonts.font_data.insert("emoji".into(),
            egui::FontData::from_owned(data).into());
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            if let Some(f) = fonts.families.get_mut(&family) {
                f.push("emoji".into());
            }
        }
    }
    ctx.set_fonts(fonts);
}

// ── 主题：对标 VS Code Dark Modern ──
mod theme {
    use eframe::egui;
    // 分层表面：标题栏/活动栏最暗 → 侧栏 → 编辑器微亮，1px 边框分隔
    pub const BG_BASE: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x18);   // title/activity/sidebar/status
    pub const BG_EDIT: egui::Color32 = egui::Color32::from_rgb(0x1f, 0x1f, 0x1f);   // editor / active tab
    pub const BG_BAR: egui::Color32 = BG_BASE;
    pub const BG_SIDE: egui::Color32 = BG_BASE;
    pub const BG_STATUS: egui::Color32 = BG_BASE;
    pub const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(0x20, 0x20, 0x20);  // 弹窗/聊天气泡/输入底
    pub const BG_INPUT: egui::Color32 = egui::Color32::from_rgb(0x31, 0x31, 0x31);
    pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(0x2a, 0x2d, 0x2e);
    pub const FG: egui::Color32 = egui::Color32::from_rgb(0xcc, 0xcc, 0xcc);
    pub const FG_BRIGHT: egui::Color32 = egui::Color32::from_rgb(0xe7, 0xe7, 0xe7);
    pub const FG_DIM: egui::Color32 = egui::Color32::from_rgb(0x9d, 0x9d, 0x9d);
    pub const FG_BAR: egui::Color32 = FG;
    pub const FG_STATUS: egui::Color32 = FG;
    pub const BORDER: egui::Color32 = egui::Color32::from_rgb(0x2b, 0x2b, 0x2b);
    pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x37, 0x94, 0xd4);    // Dark Modern 蓝
    pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(0x04, 0x39, 0x5c);
    pub const RED: egui::Color32 = egui::Color32::from_rgb(0xf1, 0x4c, 0x4c);
    pub const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xcc, 0xa7, 0x00);
    pub const GREEN: egui::Color32 = egui::Color32::from_rgb(0x6a, 0x99, 0x55);
    pub const TAB_ACTIVE: egui::Color32 = BG_EDIT;
    pub const TAB_INACTIVE: egui::Color32 = BG_BASE;
    pub const SEL_BG: egui::Color32 = egui::Color32::from_rgb(0x26, 0x3c, 0x51);    // 用户气泡/选中
    pub const LINE_NUM: egui::Color32 = egui::Color32::from_rgb(0x6e, 0x7b, 0x8b);
    pub const LINE_CUR: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);
    pub const TERMINAL_BG: egui::Color32 = BG_EDIT;
    pub const TERMINAL_FG: egui::Color32 = FG;
}

/// 全局控件样式：扁平、深色、圆角小、悬停微亮（对标 Dark Modern）
fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(theme::FG);
    v.panel_fill = theme::BG_BASE;
    v.window_fill = theme::BG_PANEL;
    v.extreme_bg_color = theme::BG_EDIT;       // TextEdit 底 = 编辑器底，避免色差
    v.code_bg_color = theme::BG_EDIT;
    v.faint_bg_color = theme::BG_EDIT;
    v.window_stroke = egui::Stroke::new(1.0, theme::BORDER);
    v.window_rounding = egui::Rounding::same(6.0);
    v.menu_rounding = egui::Rounding::same(6.0);
    v.selection.bg_fill = theme::ACCENT_DIM;
    v.selection.stroke = egui::Stroke::new(1.0, theme::ACCENT);
    v.widgets.noninteractive.bg_fill = theme::BG_BASE;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme::FG);
    v.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    v.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme::FG_DIM);
    v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    v.widgets.hovered.bg_fill = theme::BG_HOVER;
    v.widgets.hovered.weak_bg_fill = theme::BG_HOVER;
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme::FG_BRIGHT);
    v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    v.widgets.active.bg_fill = theme::ACCENT_DIM;
    v.widgets.active.weak_bg_fill = theme::ACCENT_DIM;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme::FG_BRIGHT);
    v.widgets.open.bg_fill = theme::BG_HOVER;
    v.widgets.open.weak_bg_fill = theme::BG_HOVER;
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, theme::FG_BRIGHT);
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_shadow = egui::epaint::Shadow::NONE;
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(10.0, 3.0);
    style.spacing.menu_margin = egui::Margin::symmetric(4.0, 4.0);
    style.spacing.scroll = egui::style::ScrollStyle { bar_width: 8.0, ..Default::default() };
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
    style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(16.0));
    ctx.set_style(style);
}

// ── 语法高亮 ──
const KEYWORDS: &[&str] = &[
    "fn","let","mut","if","else","while","for","in","match","return",
    "break","continue","use","module","struct","enum","type","impl",
    "as","pub","true","false","None","Some","Ok","Err","import",
    "go","ui","render","Column","Row","TextInput","Button","List","ListItem","Text",
];

const TYPE_PAT: &[&str] = &["i32","i64","f64","bool","String","char","Vec","Map","Option","Result","Record"];

fn find_matching_bracket(text: &str, pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let (open, close, forward) = match bytes.get(pos) {
        Some(b'(') => (b'(', b')', true),
        Some(b')') => (b')', b'(', false),
        Some(b'{') => (b'{', b'}', true),
        Some(b'}') => (b'}', b'{', false),
        Some(b'[') => (b'[', b']', true),
        Some(b']') => (b']', b'[', false),
        _ => return None,
    };
    let mut depth = 0i32;
    if forward {
        for i in pos..bytes.len() {
            if bytes[i] == open { depth += 1; }
            else if bytes[i] == close { depth -= 1; if depth == 0 { return Some(i); } }
        }
    } else {
        let mut i = pos;
        loop {
            if bytes[i] == close { depth += 1; }
            else if bytes[i] == open { depth -= 1; if depth == 0 { return Some(i); } }
            if i == 0 { break; }
            i -= 1;
        }
    }
    None
}

fn highlight_layout(ui: &egui::Ui, text: &str, cursor_byte: Option<usize>, search: Option<&str>, surface: usize) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let default_style = egui::TextStyle::Monospace.resolve(ui.style());

    // 括号匹配高亮集合
    let mut match_set: std::collections::HashSet<usize> = std::collections::HashSet::new();
    if let Some(cb) = cursor_byte {
        for cand in [cb, cb.saturating_sub(1)] {
            if cand < text.len() && text.is_char_boundary(cand) {
                if let Some(m) = find_matching_bracket(text, cand) {
                    match_set.insert(cand);
                    match_set.insert(m);
                    break;
                }
            }
        }
    }

    let mut byte_acc = 0usize; // 当前行起始字节偏移
    for raw_line in text.split('\n') {
        // 剥离 \r（CRLF），但字节推进必须计入，保证括号匹配偏移正确
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let mut rest = line;
        let mut off = 0usize; // 行内偏移
        let mut line_has_content = false;
        while !rest.is_empty() {
            let trimmed = rest.trim_start();
            let indent_len = rest.len() - trimmed.len();
            if indent_len > 0 {
                job.append(&rest[..indent_len], 0.0, egui::TextFormat::simple(default_style.clone(), theme::FG));
                off += indent_len;
            }
            rest = trimmed;
            if rest.is_empty() { break; }

            // 注释
            if rest.starts_with("//") {
                job.append(rest, 0.0, egui::TextFormat::simple(default_style.clone(), egui::Color32::from_rgb(0x6a, 0x99, 0x55)));
                break;
            }
            // 字符串
            if rest.starts_with('"') {
                let end = rest[1..].find('"').map(|i| i + 2).unwrap_or(rest.len());
                job.append(&rest[..end], 0.0, egui::TextFormat::simple(default_style.clone(), egui::Color32::from_rgb(0xce, 0x91, 0x78)));
                off += end;
                rest = &rest[end..];
                line_has_content = true;
                continue;
            }
            let first = rest.chars().next().unwrap();
            let is_word = first.is_alphanumeric() || first == '_' || first == '#';
            if is_word {
                let word_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '#').unwrap_or(rest.len());
                let word = &rest[..word_end];
                let color = if KEYWORDS.contains(&word) || aine::veil::lang_has_word(surface, word) {
                    egui::Color32::from_rgb(0x56, 0x9c, 0xd6)
                } else if TYPE_PAT.contains(&word) {
                    egui::Color32::from_rgb(0x4e, 0xc9, 0xb0)
                } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    egui::Color32::from_rgb(0x4e, 0xc9, 0xb0)
                } else if word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    egui::Color32::from_rgb(0xb5, 0xce, 0xa8)
                } else {
                    theme::FG
                };
                let mut fmt = egui::TextFormat::simple(default_style.clone(), color);
                fmt.font_id = egui::FontId::monospace(13.0);
                // 搜索匹配高亮（全词相等）
                if let Some(q) = search {
                    if !q.is_empty() && word.eq_ignore_ascii_case(q) {
                        fmt.background = egui::Color32::from_rgb(0x4a, 0x3b, 0x10);
                        fmt.color = theme::FG_BRIGHT;
                    }
                }
                job.append(word, 0.0, fmt);
                off += word_end;
                rest = &rest[word_end..];
                line_has_content = true;
            } else {
                // 单个符号字符（检查括号匹配）
                let ch_len = first.len_utf8();
                let gpos = byte_acc + off;
                let mut fmt = egui::TextFormat::simple(default_style.clone(), theme::FG);
                fmt.font_id = egui::FontId::monospace(13.0);
                if match_set.contains(&gpos) {
                    fmt.background = egui::Color32::from_rgb(0x26, 0x5f, 0x8a);
                    fmt.color = egui::Color32::WHITE;
                }
                job.append(&rest[..ch_len], 0.0, fmt);
                off += ch_len;
                rest = &rest[ch_len..];
                line_has_content = true;
            }
        }
        if !line_has_content && line.is_empty() {
            job.append(" ", 0.0, egui::TextFormat::simple(default_style.clone(), theme::FG));
        }
        job.append("\n", 0.0, egui::TextFormat::simple(default_style.clone(), theme::FG));
        byte_acc += raw_line.len() + 1;
    }
    job
}

// ── 语言翻译 ──
const LANGS: &[&str] = &["EN", "中文", "日本語", "Deutsch", "Français", "Русский"];

// ── 统一命令注册表：菜单/快捷键/命令面板共用一表 ──
#[derive(Clone, Copy, PartialEq)]
enum Cmd {
    Save, Build, Run, Check, NewFile, QuickOpen, Search, ToggleTerminal, ToggleAiPanel,
    GotoDef, FindRefs, Format, ClearTerm, Lang(usize), FocusMode, RunTests,
    GotoLine, ToggleComment, DuplicateLine, RenameSymbol, Debug,
}

impl Cmd {
    fn all() -> &'static [Cmd] {
        &[
            Cmd::Save, Cmd::Build, Cmd::Run, Cmd::Check, Cmd::NewFile, Cmd::QuickOpen,
            Cmd::Search, Cmd::ToggleTerminal, Cmd::ToggleAiPanel, Cmd::GotoDef, Cmd::FindRefs,
            Cmd::Format, Cmd::ClearTerm, Cmd::Lang(0), Cmd::Lang(1), Cmd::Lang(2), Cmd::Lang(3),
            Cmd::Lang(4), Cmd::Lang(5), Cmd::FocusMode, Cmd::RunTests, Cmd::GotoLine,
            Cmd::ToggleComment, Cmd::DuplicateLine, Cmd::RenameSymbol,
        ]
    }
    fn label(&self, lang: usize) -> &'static str {
        let (zh, en) = match self {
            Cmd::Save => ("保存文件", "Save File"),
            Cmd::Build => ("构建", "Build"),
            Cmd::Run => ("运行", "Run"),
            Cmd::Check => ("检查", "Check"),
            Cmd::NewFile => ("新建文件", "New File"),
            Cmd::QuickOpen => ("快速打开文件", "Quick Open File"),
            Cmd::Search => ("搜索替换", "Search & Replace"),
            Cmd::ToggleTerminal => ("切换终端", "Toggle Terminal"),
            Cmd::ToggleAiPanel => ("切换 AI 面板", "Toggle AI Panel"),
            Cmd::GotoDef => ("转到定义", "Go to Definition"),
            Cmd::FindRefs => ("查找引用", "Find References"),
            Cmd::Format => ("格式化文档", "Format Document"),
            Cmd::ClearTerm => ("清空终端", "Clear Terminal"),
            Cmd::Lang(i) => {
                static NAMES: [[&str; 2]; 6] = [
                    ["EN", "EN"], ["中文", "Chinese"], ["日本語", "Japanese"],
                    ["Deutsch", "German"], ["Français", "French"], ["Русский", "Russian"],
                ];
                return NAMES[*i as usize][lang.min(1)];
            }
            Cmd::FocusMode => ("禅模式", "Focus Mode"),
            Cmd::RunTests => ("运行测试", "Run Tests"),
            Cmd::GotoLine => ("转到行", "Go to Line"),
            Cmd::ToggleComment => ("切换行注释", "Toggle Comment"),
            Cmd::DuplicateLine => ("复制当前行", "Duplicate Line"),
            Cmd::RenameSymbol => ("重命名符号", "Rename Symbol"),
            Cmd::Debug => ("调试运行", "Debug Run"),
        };
        if lang == 1 { zh } else { en }
    }
    fn shortcut(&self) -> &'static str {
        match self {
            Cmd::Save => "Ctrl+S", Cmd::Build => "F7", Cmd::Run => "F6", Cmd::Check => "F5",
            Cmd::QuickOpen => "Ctrl+P", Cmd::Search => "Ctrl+F", Cmd::ToggleTerminal => "Ctrl+`",
            Cmd::ToggleAiPanel => "Ctrl+I", Cmd::GotoDef => "F12", Cmd::FindRefs => "Shift+F12",
            Cmd::GotoLine => "Ctrl+G", Cmd::ToggleComment => "Ctrl+/", Cmd::FocusMode => "Ctrl+Alt+F",
            Cmd::RenameSymbol => "F2", Cmd::Debug => "F9",
            _ => "",
        }
    }
    fn execute(&self, app: &mut App) {
        match self {
            Cmd::Save => app.save(),
            Cmd::Build => app.build(),
            Cmd::Run => app.run_prog(),
            Cmd::Check => app.check(),
            Cmd::NewFile => { let n = app.files.len() + 1; app.new_file(&format!("file_{}", n)); }
            Cmd::QuickOpen => { app.show_quick_open = true; app.quick_open_filter.clear(); }
            Cmd::Search => app.show_search = true,
            Cmd::ToggleTerminal => { app.show_problems = !app.show_problems; app.bottom_tab = 2; }
            Cmd::ToggleAiPanel => app.show_ai_panel = !app.show_ai_panel,
            Cmd::GotoDef => app.goto_definition(),
            Cmd::FindRefs => app.find_references(),
            Cmd::Format => app.format_document(),
            Cmd::ClearTerm => app.terminal_text.clear(),
            Cmd::Lang(i) => { app.lang_idx = *i as usize; app.save_settings(); }
            Cmd::FocusMode => app.focus_mode = !app.focus_mode,
            Cmd::RunTests => { app.run_tests(); app.bottom_tab = 3; app.show_problems = true; }
            Cmd::GotoLine => app.show_goto = true,
            Cmd::ToggleComment => app.toggle_comment(),
            Cmd::DuplicateLine => app.duplicate_line(),
            Cmd::RenameSymbol => app.show_rename = true,
            Cmd::Debug => app.run_debugger(),
        }
    }
}

// ── 命令面板注册表 (Ctrl+K) ──
const COMMANDS: &[(&str, &str)] = &[
    ("Save File", "Ctrl+S"),
    ("Build", "F7"),
    ("Run", "F6"),
    ("Check", "F5"),
    ("New File", ""),
    ("Quick Open File", "Ctrl+P"),
    ("Search & Replace", "Ctrl+F"),
    ("Toggle Terminal", "Ctrl+`"),
    ("Toggle AI Panel", "Ctrl+I"),
    ("Go to Definition", "F12"),
    ("Find References", "Shift+F12"),
    ("Format Document", ""),
    ("Clear Terminal", ""),
    ("Language: EN", ""),
    ("Language: 中文", ""),
    ("Language: 日本語", ""),
    ("Language: Deutsch", ""),
    ("Language: Français", ""),
    ("Language: Русский", ""),
    ("Toggle Focus Mode (Zen)", "Ctrl+Alt+F"),
    ("Run Tests", ""),
];

/// UI 文案：6 语言完整表（EN / 简体中文 / 日本語 / Deutsch / Français / Русский）
const LANG_KEYS: &[&str] = &[
    "run", "check", "build", "problems", "explorer", "ai", "no_problems",
    "new_file", "save_file", "quick_open", "command_palette", "close_file", "delete_file",
    "toggle_problems", "file", "edit", "view", "help", "terminal", "output", "tests",
    "ask", "settings", "provider", "api_key", "model", "base_url", "search", "replace",
    "find_next", "replace_all", "toggle_comment", "duplicate_line", "goto_line_menu",
    "ai_panel", "focus_mode", "shortcuts_title", "new_file_tip", "refresh_tip",
    "confirm_ok", "cancel", "delete_title", "unsaved_title", "unsaved_msg",
    "goto_title", "line_label", "go", "no_file", "lines_unit", "demo_mode",
];
const LANG_TABLE: [[&str; 50]; 6] = [
    // 0 EN
    ["Run", "Check", "Build", "PROBLEMS", "EXPLORER", "AI ASSISTANT", "No problems detected.",
     "New File...", "Save", "Quick Open (Ctrl+P)", "Command Palette (Ctrl+K)", "Close File", "Delete File",
     "Toggle PROBLEMS panel", "File", "Edit", "View", "Help", "TERMINAL", "OUTPUT", "TESTS",
     "Ask AI...", "Settings", "Provider", "API Key", "Model", "Base URL", "Search", "Replace",
     "Next", "Replace All", "Toggle Line Comment  Ctrl+/", "Duplicate Line", "Go to Line...  Ctrl+G",
     "AI Panel  Ctrl+I", "Focus Mode  Ctrl+Alt+F", "Shortcuts:", "New File", "Refresh",
     "OK", "Cancel", "Delete File", "Unsaved Changes", "This file has unsaved changes. Close and discard them?",
     "Go to Line", "Line:", "Go", "no file", "lines", "Demo mode (no API key needed)"],
    // 1 简体中文
    ["运行", "检查", "构建", "问题", "资源管理器", "AI 助手", "未检测到问题。",
     "新建文件...", "保存", "快速打开 (Ctrl+P)", "命令面板 (Ctrl+K)", "关闭文件", "删除文件",
     "切换问题面板", "文件", "编辑", "查看", "帮助", "终端", "输出", "TESTS",
     "向 AI 提问...", "设置", "服务商", "API 密钥", "模型", "Base URL", "搜索", "替换",
     "下一个", "全部替换", "切换行注释  Ctrl+/", "复制当前行", "转到行...  Ctrl+G",
     "AI 面板  Ctrl+I", "禅模式  Ctrl+Alt+F", "快捷键：", "新建文件", "刷新文件树",
     "确定", "取消", "删除文件", "未保存的修改", "此文件有未保存的修改，关闭将丢弃。确定关闭？",
     "转到行", "行号:", "跳转", "无文件", "行", "演示模式（无需 API Key）"],
    // 2 日本語
    ["実行", "チェック", "ビルド", "問題", "エクスプローラー", "AI アシスタント", "問題は検出されませんでした。",
     "新規ファイル...", "保存", "クイックオープン (Ctrl+P)", "コマンドパレット (Ctrl+K)", "ファイルを閉じる", "ファイルを削除",
     "問題パネルの切り替え", "ファイル", "編集", "表示", "ヘルプ", "ターミナル", "出力", "TESTS",
     "AI に質問...", "設定", "プロバイダー", "API キー", "モデル", "Base URL", "検索", "置換",
     "次へ", "すべて置換", "行コメント切り替え  Ctrl+/", "行を複製", "指定行へ移動...  Ctrl+G",
     "AI パネル  Ctrl+I", "集中モード  Ctrl+Alt+F", "ショートカット:", "新規ファイル", "更新",
     "OK", "キャンセル", "ファイルを削除", "未保存の変更", "未保存の変更があります。閉じて破棄しますか？",
     "行へ移動", "行:", "移動", "ファイルなし", "行", "デモモード（API キー不要）"],
    // 3 Deutsch
    ["Ausführen", "Prüfen", "Bauen", "PROBLEME", "EXPLORER", "KI-ASSISTENT", "Keine Probleme gefunden.",
     "Neue Datei...", "Speichern", "Schnell öffnen (Ctrl+P)", "Befehlspalette (Ctrl+K)", "Datei schließen", "Datei löschen",
     "Problempanel umschalten", "Datei", "Bearbeiten", "Ansicht", "Hilfe", "TERMINAL", "AUSGABE", "TESTS",
     "KI fragen...", "Einstellungen", "Anbieter", "API-Schlüssel", "Modell", "Base URL", "Suchen", "Ersetzen",
     "Weiter", "Alle ersetzen", "Zeilenkommentar umschalten  Ctrl+/", "Zeile duplizieren", "Zu Zeile springen...  Ctrl+G",
     "KI-Panel  Ctrl+I", "Fokusmodus  Ctrl+Alt+F", "Tastenkürzel:", "Neue Datei", "Aktualisieren",
     "OK", "Abbrechen", "Datei löschen", "Ungespeicherte Änderungen", "Diese Datei hat ungespeicherte Änderungen. Schließen und verwerfen?",
     "Zu Zeile springen", "Zeile:", "Los", "keine Datei", "Zeilen", "Demomodus (kein API-Schlüssel nötig)"],
    // 4 Français
    ["Exécuter", "Vérifier", "Compiler", "PROBLÈMES", "EXPLORATEUR", "ASSISTANT IA", "Aucun problème détecté.",
     "Nouveau fichier...", "Enregistrer", "Ouverture rapide (Ctrl+P)", "Palette de commandes (Ctrl+K)", "Fermer le fichier", "Supprimer le fichier",
     "Basculer le panneau de problèmes", "Fichier", "Édition", "Affichage", "Aide", "TERMINAL", "SORTIE", "TESTS",
     "Demander à l'IA...", "Paramètres", "Fournisseur", "Clé API", "Modèle", "Base URL", "Rechercher", "Remplacer",
     "Suivant", "Tout remplacer", "Basculer le commentaire de ligne  Ctrl+/", "Dupliquer la ligne", "Aller à la ligne...  Ctrl+G",
     "Panneau IA  Ctrl+I", "Mode concentration  Ctrl+Alt+F", "Raccourcis :", "Nouveau fichier", "Actualiser",
     "OK", "Annuler", "Supprimer le fichier", "Modifications non enregistrées", "Ce fichier contient des modifications non enregistrées. Fermer et abandonner ?",
     "Aller à la ligne", "Ligne :", "Aller", "aucun fichier", "lignes", "Mode démo (sans clé API)"],
    // 5 Русский
    ["Запустить", "Проверить", "Собрать", "ПРОБЛЕМЫ", "ПРОВОДНИК", "ИИ-АССИСТЕНТ", "Проблем не обнаружено.",
     "Новый файл...", "Сохранить", "Быстрое открытие (Ctrl+P)", "Палитра команд (Ctrl+K)", "Закрыть файл", "Удалить файл",
     "Показать панель проблем", "Файл", "Правка", "Вид", "Справка", "ТЕРМИНАЛ", "ВЫВОД", "TESTS",
     "Спросить ИИ...", "Настройки", "Провайдер", "API-ключ", "Модель", "Base URL", "Поиск", "Заменить",
     "Далее", "Заменить всё", "Переключить комментарий строки  Ctrl+/", "Дублировать строку", "Перейти к строке...  Ctrl+G",
     "Панель ИИ  Ctrl+I", "Режим фокуса  Ctrl+Alt+F", "Горячие клавиши:", "Новый файл", "Обновить",
     "OK", "Отмена", "Удалить файл", "Несохранённые изменения", "В файле есть несохранённые изменения. Закрыть и потерять их?",
     "Перейти к строке", "Строка:", "Переход", "нет файла", "строк", "Демо-режим (без API-ключа)"],
];

fn tr(key: &'static str, lang: usize) -> &'static str {
    let lang = lang.min(5);
    LANG_KEYS.iter().position(|k| *k == key)
        .map(|i| LANG_TABLE[lang][i])
        .unwrap_or(key)
}

// ── 模型注册表（models.toml，T34 Capability+Router）──
#[derive(Clone, Default)]
struct ModelEntry {
    id: String,                // 厂商标识（deployment 名）
    provider: String,          // 展示用 provider_id
    protocol: String,          // openai_chat / openai_responses / anthropic_messages / gemini / provider_native
    base_url: String,
    model: String,             // provider 侧模型 ID
    api_key: String,           // 每厂商独立凭据（models.toml 本地保存）
    capabilities: Vec<String>, // chat / explain / fix / reason
}

// ── Catalog 模型目录（Umber catalog.json：9624 deployments，惰性解析）──
#[derive(Clone)]
struct CatEntry {
    provider: String,
    model_id: String,
}
fn catalog_entries() -> &'static Vec<CatEntry> {
    use std::sync::OnceLock;
    static CAT: OnceLock<Vec<CatEntry>> = OnceLock::new();
    CAT.get_or_init(|| {
        let Ok(text) = std::fs::read_to_string("catalog.json") else { return Vec::new(); };
        // 轻量提取 "deployments":[{"provider":"..","model_id":".."}...]（避免引入 serde）
        let mut out = Vec::new();
        if let Some(pos) = text.find("\"deployments\":") {
            let body = &text[pos..];
            let mut from = 0usize;
            while from < body.len() && out.len() < 20000 {
                let Some(po) = body[from..].find("\"provider\"") else { break };
                let seg = &body[from + po..];
                let Some(mo) = seg.find("\"model_id\"") else { break };
                let prov = json_string_field(seg, "provider");
                let mid = json_string_field(&seg[mo..], "model_id");
                if !prov.is_empty() && !mid.is_empty() {
                    out.push(CatEntry { provider: prov, model_id: mid });
                }
                from += po + mo + 10;
            }
        }
        out
    })
}

fn parse_models_toml(text: &str) -> Vec<ModelEntry> {
    let mut out: Vec<ModelEntry> = Vec::new();
    let mut cur: Option<ModelEntry> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("[[model]]") {
            if let Some(m) = cur.take() { out.push(m); }
            cur = Some(ModelEntry { id: String::new(), provider: "custom".into(), protocol: "openai_chat".into(), base_url: String::new(), model: String::new(), api_key: String::new(), capabilities: vec!["chat".into()] });
        } else if let Some(m) = cur.as_mut() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').to_string();
                match k.trim() {
                    "id" => m.id = v,
                    "provider" => m.provider = v,
                    "protocol" => m.protocol = v,
                    "base_url" => m.base_url = v,
                    "model" => m.model = v,
                    "model_id" => m.model = v,
                    "api_key" => m.api_key = v,
                    "capabilities" => {
                        m.capabilities = v.trim_start_matches('[').trim_end_matches(']')
                            .split(',').map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty()).collect();
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(m) = cur.take() { out.push(m); }
    out
}

// ── 结构化诊断 ──
#[derive(Clone)]
struct Diag {
    severity: String, // "error" | "warning"
    code: String,
    message: String,
    line: usize, // 1-based
    col: usize,
    quick_fix: String,
}

// ── AI 提出的变更（DiffSet 简化版：整文件替换）──
#[derive(Clone)]
struct PendingDiff {
    file: String,
    old_content: String,
    new_content: String,
    reason: String,
}

// ── 文件树缓存（避免每帧递归 read_dir 扫磁盘）──
#[derive(Default, Clone)]
struct FsDir {
    name: String,
    path: PathBuf,
    dirs: Vec<FsDir>,
    files: Vec<String>, // .aine 文件名
}

// ── 危险操作确认 ──
enum Confirm {
    DeleteFile(String), // 文件相对路径
    CloseTab(usize),    // 有未保存修改的标签
}

// ── 后台任务层：所有子进程调用走这里，UI 永不冻结 ──
pub enum TaskMsg {
    Checked { stderr: String, diags: Vec<Diag> }, // 检查完成
    Built { text: String, ok: bool },             // 构建输出（追加到终端）
    Ran { text: String },                         // 运行输出
    Git { output: String },                       // git 状态
    Term { text: String },                        // 终端命令输出
}

// ── Tab ──
struct Tab {
    name: String,
    content: String,           // 恒为 canonical（en）；表面只在显示层
    dirty: bool,
    cursor_line: usize,
    cursor_col: usize,
    cursor_byte: Option<usize>, // byte offset for bracket matching
    surface: usize,             // Language Veil: 0=zh 1=en 2=ja 3=de 4=fr 5=ru
}

// ── AI 设置 ──
#[derive(Clone)]
struct AiSettings {
    show: bool,
    provider: String,
    api_key: String,
    model: String,
    base_url: String,
    demo: bool, // 演示模式：Umber 内置假流，无需 API Key
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            show: false,
            provider: "OpenAICompatible".into(),
            api_key: String::new(),
            model: "deepseek-chat".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            demo: false,
        }
    }
}

// ── App ──
struct App {
    root: PathBuf,
    files: Vec<String>,
    tabs: Vec<Tab>,
    active_tab: usize,
    n_errors: usize,
    n_warnings: usize,
    lang_idx: usize,
    show_problems: bool,
    bottom_tab: usize, // 0=PROBLEMS 1=OUTPUT 2=TERMINAL
    output_text: String,
    terminal_text: String,
    terminal_input: String,
    terminal_cwd: PathBuf,
    show_search: bool,
    search_text: String,
    replace_text: String,
    show_quick_open: bool,
    quick_open_filter: String,
    show_command_bar: bool,
    command_filter: String,
    command_action: Option<usize>, // selected command to execute after UI closure
    show_ai_panel: bool,           // right AI panel visibility
    pending_cursor_chars: Option<usize>, // cursor position to apply to editor next frame
    diags: Vec<Diag>,              // structured diagnostics
    pending_diff: Option<PendingDiff>, // AI-proposed change awaiting approval
    ai_explaining: bool,           // AI is explaining/fixing (chat busy)
    fixing_file: Option<String>,   // ai/fix in flight: target file
    fixing_old: Option<String>,    // ai/fix in flight: original content
    fixing_reason: Option<String>, // ai/fix in flight: diagnostic summary
    sidebar_view: usize,           // 0=Explorer 1=Search 2=Git 3=Outline
    outline: Vec<(String, String, usize, usize)>, // (name, kind, line0, col0)
    proj_search: String,           // project-wide search query
    proj_results: Vec<(String, usize, String)>, // (file, line, text)
    test_output: String,           // aine test results
    breakpoints: Vec<usize>,       // 断点行号（1 基，当前文件）
    git_commit_msg: String,        // git commit 消息草稿
    debug_output: String,          // aine debug 报告
    models: Vec<ModelEntry>,       // models.toml routing table
    focus_mode: bool,              // 禅模式：隐藏所有面板只留编辑器
    confirm: Option<Confirm>,      // 危险操作确认（删除文件/关闭脏Tab）
    pending_selection: Option<(usize, usize)>, // 搜索/跳转的选区（字符索引）
    show_goto: bool,               // 转到行对话框
    goto_input: String,
    show_rename: bool,             // 重命名符号对话框
    rename_input: String,
    toasts: Vec<(String, std::time::Instant)>, // 右下角通知（4 秒过期）
    side_width: f32,               // 侧栏宽度（持久化）
    side_width_live: f32,
    lsp_stdin: Option<std::process::ChildStdin>, // aine lsp 子进程
    lsp_rx: Option<std::sync::mpsc::Receiver<(String, String, String)>>, // (uri, code, message) 逐条诊断
    lsp_res_rx: Option<std::sync::mpsc::Receiver<(i64, String)>>, // (id, 响应体)
    lsp_wait: Option<(i64, &'static str)>, // 在途请求 (id, kind)
    lsp_next_id: i64,
    show_completion: bool,          // 补全弹窗
    completions: Vec<String>,
    show_veil_preview: bool,       // Veil 双视图：canonical/表面 对照预览
    hover_idle: Option<std::time::Instant>,
    hover_sent_at: Option<std::time::Instant>,
    task_tx: Option<std::sync::mpsc::Sender<TaskMsg>>, // 后台任务通道（clone 给线程）
    task_rx: Option<std::sync::mpsc::Receiver<TaskMsg>>,
    task_busy: Option<&'static str>, // 状态栏显示的任务标签
    fs_root: Option<FsDir>,          // 文件树缓存
    fs_scanned_at: Option<std::time::Instant>,
    show_ai_settings: bool,
    ai_settings: AiSettings,
    ai_chat: Vec<(bool, String)>, // (is_user, message)
    ai_input: String,
    status: String,
    collapsed: std::collections::HashSet<PathBuf>, // collapsed folders in tree
    hover_info: Option<String>,       // hover tooltip text
    hover_pos: Option<egui::Pos2>,    // hover screen position
    ai_rx: Option<std::sync::mpsc::Receiver<AiMsg>>, // AI streaming channel (Umber)
    ai_streaming: bool,               // currently streaming
    ai_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>, // 流式取消标志
    ai_reasoning: String,          // 当前回复的思维链
    ai_usage: Option<(u64, u64)>,  // token 用量
    pending_task: &'static str,    // 本次请求的任务画像（路由用）
    settings_idx: usize,           // Settings 当前编辑的厂商索引
    show_model_picker: bool,       // catalog 模型选择器
    model_filter: String,
}

impl App {
    /// Recursively collect all .aine files under `dir`, storing paths relative to `base`
    fn collect_aine_files(base: &PathBuf, dir: &PathBuf, out: &mut Vec<String>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // skip target, .git, node_modules
                    let dn = entry.file_name().to_string_lossy().to_string();
                    if dn == "target" || dn == ".git" || dn == "node_modules" { continue; }
                    Self::collect_aine_files(base, &path, out);
                } else if path.to_string_lossy().ends_with(".aine") {
                    if let Ok(rel) = path.strip_prefix(base) {
                        out.push(rel.to_string_lossy().replace("\\", "/"));
                    }
                }
            }
        }
    }

    fn new() -> Self {
        let root = find_project_root();
        let examples_dir = root.join("examples");
        let mut files: Vec<String> = Vec::new();
        Self::collect_aine_files(&examples_dir, &examples_dir, &mut files);
        files.sort();
        let content = if files.is_empty() { String::new() }
            else { std::fs::read_to_string(examples_dir.join(&files[0])).unwrap_or_default() };
        let tab = Tab { name: files.first().cloned().unwrap_or_default(), content, dirty: false, cursor_line: 0, cursor_col: 0, cursor_byte: None, surface: 1 };
        let mut app = App {
            root, files, tabs: vec![tab], active_tab: 0,
            n_errors: 0, n_warnings: 0, lang_idx: 0,
            show_problems: true, bottom_tab: 0,
            output_text: String::new(), terminal_text: String::new(),
            terminal_input: String::new(), terminal_cwd: find_project_root(),
            show_search: false, search_text: String::new(), replace_text: String::new(),
            show_quick_open: false, quick_open_filter: String::new(),
            show_command_bar: false, command_filter: String::new(), command_action: None,
            show_ai_panel: true, pending_cursor_chars: None,
            diags: vec![], pending_diff: None, ai_explaining: false,
            fixing_file: None, fixing_old: None, fixing_reason: None,
            sidebar_view: 0, proj_search: String::new(), proj_results: vec![],
            outline: vec![],
            test_output: String::new(),
            breakpoints: vec![], debug_output: String::new(), git_commit_msg: String::new(),
            models: Vec::new(),
            focus_mode: false,
            confirm: None, pending_selection: None, show_goto: false, goto_input: String::new(),
            show_rename: false, rename_input: String::new(),
            toasts: vec![], side_width: 200.0, side_width_live: 200.0,
            lsp_stdin: None, lsp_rx: None,
            lsp_res_rx: None, lsp_wait: None, lsp_next_id: 10,
            show_completion: false, completions: vec![],
            show_veil_preview: false,
            hover_idle: None, hover_sent_at: None,
            task_tx: None, task_rx: None, task_busy: None,
            fs_root: None, fs_scanned_at: None,
            show_ai_settings: false, ai_settings: AiSettings::default(),
            ai_chat: vec![], ai_input: String::new(),
            status: "Ready".into(),
            collapsed: std::collections::HashSet::new(),
            hover_info: None, hover_pos: None,
            ai_rx: None, ai_streaming: false,
            ai_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ai_reasoning: String::new(), ai_usage: None,
            pending_task: "chat", settings_idx: 0,
            show_model_picker: false, model_filter: String::new(),
        };
        let (ttx, trx) = std::sync::mpsc::channel();
        app.task_tx = Some(ttx);
        app.task_rx = Some(trx);
        app.load_settings();
        app.load_models();
        app.lsp_start();
        if let (Some(t0), Some(t0c)) = (app.tabs.first(), app.tabs.first()) {
            let (n, c) = (t0.name.clone(), t0c.content.clone());
            app.lsp_notify("textDocument/didOpen", &n, &c);
        }
        app.check();
        app
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn settings_path(&self) -> PathBuf {
        self.root.join(".aine_studio").join("settings")
    }

    fn load_settings(&mut self) {
        let path = self.settings_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    match k.trim() {
                        "provider" => self.ai_settings.provider = v.trim().to_string(),
                        "api_key" => self.ai_settings.api_key = v.trim().to_string(),
                        "model" => self.ai_settings.model = v.trim().to_string(),
                        "base_url" => self.ai_settings.base_url = v.trim().to_string(),
                        "demo" => self.ai_settings.demo = v.trim() == "true",
                        "lang_idx" => {
                            if let Ok(idx) = v.trim().parse() { self.lang_idx = idx; }
                        }
                        "side_width" => {
                            if let Ok(w) = v.trim().parse() { self.side_width = w; self.side_width_live = w; }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// 多厂商部署表写回 models.toml（T39 Model Center 持久化）
    fn save_models_toml(&self) {
        let mut out = String::from("# Aine Studio 多厂商部署表（T34 Router / T39 Model Center）\n");
        for m in &self.models {
            out.push_str(&format!(
                "\n[[model]]\nid = \"{}\"\nprovider = \"{}\"\nprotocol = \"{}\"\nbase_url = \"{}\"\nmodel = \"{}\"\napi_key = \"{}\"\ncapabilities = [{}]\n",
                m.id, m.provider, m.protocol, m.base_url, m.model, m.api_key,
                m.capabilities.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
            ));
        }
        let _ = std::fs::write(self.root.join("models.toml"), out);
    }

    fn save_settings(&self) {
        let path = self.settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = format!(
            "provider={}\napi_key={}\nmodel={}\nbase_url={}\nlang_idx={}\nside_width={}\n",
            self.ai_settings.provider,
            self.ai_settings.api_key,
            self.ai_settings.model,
            self.ai_settings.base_url,
            self.lang_idx,
            self.side_width_live,
        );
        let _ = std::fs::write(&path, content);
    }

    /// 读取 models.toml（T34 静态路由表）；不存在则写入默认
    fn load_models(&mut self) {
        let path = self.root.join("models.toml");
        if !path.exists() {
            let default = "# Aine Studio 多厂商部署表（T34 Router / T39 Model Center）\n# protocol: openai_chat / openai_responses / anthropic_messages / gemini / provider_native\n# capabilities: chat / explain / fix / reason\n# 每个厂商独立 api_key；发送时全部注册为 Umber Deployment，按能力跨厂商路由\n\n[[model]]\nid = \"deepseek\"\nprovider = \"deepseek\"\nprotocol = \"openai_chat\"\nbase_url = \"https://api.deepseek.com/v1\"\nmodel = \"deepseek-chat\"\napi_key = \"\"\ncapabilities = [\"chat\", \"explain\", \"fix\"]\n\n[[model]]\nid = \"anthropic\"\nprovider = \"anthropic\"\nprotocol = \"anthropic_messages\"\nbase_url = \"https://api.anthropic.com\"\nmodel = \"claude-sonnet-4\"\napi_key = \"\"\ncapabilities = [\"chat\", \"explain\", \"fix\", \"reason\"]\n";
            let _ = std::fs::write(&path, default);
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            self.models = parse_models_toml(&text);
        }
    }

    /// T34 Router：按任务能力画像选模型（仅限当前 provider，避免 key 不匹配）
    fn route_model(&self, task: &str) -> Option<&ModelEntry> {
        let provider = &self.ai_settings.provider;
        self.models.iter()
            .find(|m| m.provider == *provider && m.capabilities.iter().any(|c| c == task))
            .or_else(|| self.models.iter().find(|m| m.provider == *provider && m.id == self.ai_settings.model))
            .or_else(|| self.models.iter().find(|m| m.provider == *provider))
    }

    /// 按任务画像路由（跨厂商）后发送
    fn send_routed(&mut self, task: &'static str) {
        self.pending_task = task;
        self.send_ai_message();
    }

    fn new_file(&mut self, name: &str) {
        if name.is_empty() { return; }
        let fname = if name.ends_with(".aine") { name.to_string() } else { format!("{}.aine", name) };
        let path = self.root.join("examples").join(&fname);
        if path.exists() { return; }
        let _ = std::fs::write(&path, "// New Aine file\n");
        self.fs_scanned_at = None; // 触发文件树重扫
        self.files.push(fname.clone());
        self.files.sort();
        if let Some(idx) = self.files.iter().position(|f| *f == fname) {
            self.open_file(idx);
        }
    }

    fn open_file(&mut self, idx: usize) {
        if idx >= self.files.len() { return; }
        let rel_path = self.files[idx].clone();
        // 已打开则切换
        if let Some(tab_idx) = self.tabs.iter().position(|t| t.name == rel_path) {
            self.active_tab = tab_idx;
            return;
        }
        let path = self.root.join("examples").join(&rel_path);
        // 编码检测：BOM / 非法 UTF-8 提示（暂不自动转换）
        if let Ok(bytes) = std::fs::read(&path) {
            let bom_utf8 = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
            let bom_utf16le = bytes.starts_with(&[0xFF, 0xFE]);
            let bom_utf16be = bytes.starts_with(&[0xFE, 0xFF]);
            if bom_utf16le || bom_utf16be {
                self.toast(format!("{} 是 UTF-16 编码，暂不支持", rel_path));
                return;
            }
            if !bom_utf8 && std::str::from_utf8(&bytes).is_err() {
                self.toast(format!("{} 可能是 GBK/非UTF-8 编码，中文将显示乱码", rel_path));
            }
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            self.lsp_notify("textDocument/didOpen", &rel_path, &content);
            self.tabs.push(Tab { name: rel_path, content, dirty: false, cursor_line: 0, cursor_col: 0, cursor_byte: None, surface: 1 });
            self.active_tab = self.tabs.len() - 1;
            self.check();
        }
    }

    /// 真删除文件（从磁盘移除 + 从文件列表/标签移除）
    fn delete_file(&mut self, rel: &str) {
        let path = self.root.join("examples").join(rel);
        let _ = std::fs::remove_file(&path);
        self.files.retain(|f| *f != rel);
        if let Some(ti) = self.tabs.iter().position(|t| t.name == rel) {
            self.close_tab(ti);
        }
        self.status = format!("已删除 {}", rel);
        self.fs_scanned_at = None; // 触发文件树重扫
    }

    /// 请求关闭标签（脏标签先确认）
    fn request_close_tab(&mut self, idx: usize) {
        let dirty = self.tabs.get(idx).map(|t| t.dirty).unwrap_or(false);
        if dirty {
            self.confirm = Some(Confirm::CloseTab(idx));
        } else {
            self.close_tab(idx);
        }
    }

    /// 跳到指定行（1-based，越界则钳制）
    fn jump_to_line(&mut self, line: usize) {
        let Some(t) = self.active_tab() else { return };
        let content = t.content.clone();
        let byte = content.split('\n').take(line.saturating_sub(1))
            .map(|l| l.chars().count() + 1).sum::<usize>()
            .saturating_sub(if line > 1 { 1 } else { 0 });
        let char_idx = content[..byte.min(content.len())].chars().count();
        self.pending_cursor_chars = Some(char_idx);
    }

    /// 查找下一个：从光标向后找，包尾环绕；命中即选中（跳转+高亮）
    fn find_next(&mut self) {
        let q = self.search_text.trim().to_string();
        let Some(t) = self.active_tab() else { return };
        if q.is_empty() { return; }
        let content = t.content.clone();
        let start = (t.cursor_byte.unwrap_or(0) + q.len()).min(content.len());
        let hit = content[start..].find(&q)
            .map(|i| start + i)
            .or_else(|| content.find(&q));
        if let Some(byte) = hit {
            let s_char = content[..byte].chars().count();
            let e_char = s_char + q.chars().count();
            self.pending_selection = Some((s_char, e_char));
            self.status = format!("找到于 {}", content[..byte].matches('\n').count() + 1);
        } else {
            self.status = "未找到".into();
        }
    }

    fn search_count(&self) -> usize {
        let Some(t) = self.active_tab() else { return 0 };
        let q = self.search_text.trim();
        if q.is_empty() { 0 } else { t.content.matches(q).count() }
    }

    /// 切换当前行注释（//）
    fn toggle_comment(&mut self) {
        let Some(t) = self.active_tab_mut() else { return };
        let line_no = t.cursor_line;
        let lines: Vec<String> = t.content.split('\n').map(|s| s.to_string()).collect();
        if line_no >= lines.len() { return; }
        let trimmed = lines[line_no].trim_start();
        let indent_len = lines[line_no].len() - trimmed.len();
        let mut new_line = if trimmed.starts_with("//") {
            let rest = trimmed.trim_start_matches('/').trim_start();
            format!("{}{}", &lines[line_no][..indent_len], rest)
        } else {
            format!("{}// {}", &lines[line_no][..indent_len], trimmed)
        };
        new_line.truncate(4000);
        t.content = lines.iter().enumerate()
            .map(|(i, l)| if i == line_no { new_line.clone() } else { l.clone() })
            .collect::<Vec<_>>().join("\n");
        t.dirty = true;
    }

    /// 复制当前行到下一行
    fn duplicate_line(&mut self) {
        let Some(t) = self.active_tab_mut() else { return };
        let lines: Vec<String> = t.content.split('\n').map(|s| s.to_string()).collect();
        if t.cursor_line >= lines.len() { return; }
        let cur = lines[t.cursor_line].clone();
        let mut out = Vec::with_capacity(lines.len() + 1);
        for (i, l) in lines.iter().enumerate() {
            out.push(l.clone());
            if i == t.cursor_line { out.push(cur.clone()); }
        }
        t.content = out.join("\n");
        t.dirty = true;
    }

    fn close_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len().saturating_sub(1);
            }
        }
    }

    fn save(&mut self) {
        let dirty = self.active_tab().map(|t| t.dirty).unwrap_or(false);
        let name = self.active_tab().map(|t| t.name.clone()).unwrap_or_default();
        let content = self.active_tab().map(|t| t.content.clone()).unwrap_or_default();
        if dirty {
            let path = self.root.join("examples").join(&name);
            let _ = std::fs::write(&path, &content);
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.dirty = false;
            }
            self.status = format!("Saved {}", name);
            self.toast(format!("已保存 {}", name));
            // LSP：保存即 didChange（结构化诊断推送回填）
            self.lsp_notify("textDocument/didChange", &name, &content);
            self.check();
        }
    }

    fn build(&mut self) {
        let Some(tab_name) = self.active_tab().map(|t| t.name.clone()) else { return };
        self.save();
        let path = self.root.join("examples").join(&tab_name);
        let aine = self.root.join("target").join("debug").join("aine.exe");
        let name = tab_name.clone();
        self.spawn_task("构建中", move |tx| {
            let out = std::process::Command::new(&aine).args(["build", &path.to_string_lossy()]).output();
            let text = match out {
                Ok(o) => {
                    let mut t = format!("$ aine build {}\n", name);
                    t.push_str(&String::from_utf8_lossy(&o.stdout));
                    t.push_str(&String::from_utf8_lossy(&o.stderr));
                    t.push_str(if o.status.success() { "\n构建完成。\n" } else { "\n构建失败。\n" });
                    t
                }
                Err(e) => format!("构建错误: {}\n", e),
            };
            let ok_flag = text.contains("构建完成");
            let _ = tx.send(TaskMsg::Built { text, ok: ok_flag });
        });
    }

    fn run_prog(&mut self) {
        let Some(tab) = self.active_tab() else { return };
        let stem = tab.name.trim_end_matches(".aine").replace('/', "_");
        let exe = self.root.join(format!("{}_gen.exe", stem));
        let label = stem.to_string();
        self.spawn_task("运行中", move |tx| {
            let text = if exe.exists() {
                match std::process::Command::new(&exe).output() {
                    Ok(o) => {
                        let mut t = format!("$ {}\n", label);
                        t.push_str(&String::from_utf8_lossy(&o.stdout));
                        t.push_str(&String::from_utf8_lossy(&o.stderr));
                        t
                    }
                    Err(e) => format!("运行错误: {}\n", e),
                }
            } else {
                format!("{} 尚未构建，请先 Build (F7)。\n", label)
            };
            let _ = tx.send(TaskMsg::Ran { text });
        });
    }

    /// 通知（右下角 toast，4 秒自动消失）
    fn toast(&mut self, msg: impl Into<String>) {
        self.toasts.push((msg.into(), std::time::Instant::now()));
        if self.toasts.len() > 5 { self.toasts.remove(0); }
    }

    fn render_toasts(&mut self, ctx: &egui::Context) {
        self.toasts.retain(|(_, t)| t.elapsed().as_secs() < 4);
        if self.toasts.is_empty() { return; }
        let screen = ctx.screen_rect();
        let n = self.toasts.len();
        for (k, (msg, _)) in self.toasts.iter().enumerate() {
            let w = 300.0;
            let h = 36.0;
            let pos = egui::pos2(screen.right() - w - 12.0, screen.bottom() - 40.0 - (n as f32 - 1.0 - k as f32) * (h + 6.0));
            let id = egui::Id::new(("toast", k, msg.len()));
            egui::Area::new(id)
                .fixed_pos(pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(theme::BG_PANEL)
                        .stroke(egui::Stroke::new(1.0, theme::ACCENT))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.set_width(w - 20.0);
                            ui.label(egui::RichText::new(msg.as_str()).color(theme::FG).size(12.0));
                        });
                });
        }
    }

    /// 启动 aine lsp 子进程并握手（诊断推送通道）
    fn lsp_start(&mut self) {
        if self.lsp_stdin.is_some() { return; }
        let aine = self.root.join("target").join("debug").join("aine.exe");
        let mut child = match std::process::Command::new(&aine)
            .arg("lsp").stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()).spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();
        self.lsp_res_rx = Some(rx2);
        let init = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"capabilities\":{}}}";
        let inited = "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}";
        let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", init.len(), init);
        let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", inited.len(), inited);
        self.lsp_stdin = Some(stdin);
        self.lsp_rx = Some(rx);
        std::thread::spawn(move || {
            use std::io::Read;
            let mut rdr = stdout;
            loop {
                let mut header = String::new();
                let mut byte = [0u8; 1];
                loop {
                    match rdr.read(&mut byte) { Ok(1) => header.push(byte[0] as char), _ => return }
                    if header.ends_with("\r\n\r\n") { break; }
                    if header.len() > 200 { return; }
                }
                let len: usize = header.trim().split("\r\n")
                    .find_map(|l| l.strip_prefix("Content-Length: ").and_then(|v| v.trim().parse().ok()))
                    .unwrap_or(0);
                if len == 0 { return; }
                let mut buf = vec![0u8; len];
                if rdr.read_exact(&mut buf).is_err() { return; }
                let body = String::from_utf8_lossy(&buf).to_string();
                if body.contains("\"id\":") && body.contains("\"result\"") {
                    // 请求响应：提取 id 后整包转交
                    let id = json_num_field(&body, "id").unwrap_or(0);
                    let _ = tx2.send((id as i64, body));
                    continue;
                }
                if !body.contains("publishDiagnostics") { continue; }
                let uri = json_string_field(&body, "uri");
                for item in body.split("}, {").chain(body.split("},{")) {
                    let line = json_num_field(item, "line").unwrap_or(0);
                    let chara = json_num_field(item, "character").unwrap_or(0);
                    let sev = json_num_field(item, "severity").unwrap_or(1);
                    let msg = json_string_field(item, "message");
                    if msg.is_empty() { continue; }
                    let code = {
                        let inner = msg.trim_start_matches('[');
                        inner.split(']').next().unwrap_or("").to_string()
                    };
                    let severity = if sev == 1 { "error" } else { "warning" };
                    let _ = tx.send((uri.clone(), code, format!("{}|{}|{}|{}", line, chara, severity, msg)));
                }
            }
        });
    }

    /// didOpen / didChange（全文同步）
    fn lsp_notify(&mut self, method: &str, rel: &str, text: &str) {
        if self.lsp_stdin.is_none() { self.lsp_start(); }
        let Some(stdin) = self.lsp_stdin.as_mut() else { return };
        let abs = self.root.join("examples").join(rel);
        let uri = format!("file:///{}", abs.to_string_lossy().replace("\\", "/"));
        let body_text = json_escape(text);
        let params = if method == "textDocument/didOpen" {
            format!("{{\"textDocument\":{{\"uri\":\"{}\",\"languageId\":\"aine\",\"version\":1,\"text\":\"{}\"}}}}", uri, body_text)
        } else {
            format!("{{\"textDocument\":{{\"uri\":\"{}\",\"version\":2}},\"contentChanges\":[{{\"text\":\"{}\"}}]}}", uri, body_text)
        };
        let msg = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{}\",\"params\":{}}}", method, params);
        let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", msg.len(), msg);
    }

    /// 发送请求并挂起等待响应（响应在 poll_lsp 中按 id 匹配）
    fn lsp_request(&mut self, tag: &'static str, method: &str, params: String) {
        let Some(stdin) = self.lsp_stdin.as_mut() else { return };
        self.lsp_next_id += 1;
        let id = self.lsp_next_id;
        let msg = format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"{}\",\"params\":{}}}", id, method, params);
        let _ = write!(stdin, "Content-Length: {}\r\n\r\n{}", msg.len(), msg);
        self.lsp_wait = Some((id, tag));
    }

    /// 请求光标处 definition（LSP 精确跳转）
    fn lsp_goto_definition(&mut self) {
        let Some(t) = self.active_tab() else { return };
        let (line, col) = (t.cursor_line, t.cursor_col);
        let uri = format!("file:///{}", self.root.join("examples").join(&t.name).to_string_lossy().replace("\\", "/"));
        let params = format!("{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}", uri, line, col);
        self.lsp_request("definition", "textDocument/definition", params);
    }

    /// 请求光标处 hover（LSP 精确类型提示，替代指针估算）
    fn lsp_hover(&mut self) {
        let Some(t) = self.active_tab() else { return };
        let (line, col) = (t.cursor_line, t.cursor_col);
        let uri = format!("file:///{}", self.root.join("examples").join(&t.name).to_string_lossy().replace("\\", "/"));
        let params = format!("{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}", uri, line, col);
        self.lsp_request("hover", "textDocument/hover", params);
    }

    /// 请求光标处补全
    fn lsp_completion(&mut self) {
        let Some(t) = self.active_tab() else { return };
        let (line, col) = (t.cursor_line, t.cursor_col);
        let uri = format!("file:///{}", self.root.join("examples").join(&t.name).to_string_lossy().replace("\\", "/"));
        let params = format!("{{\"textDocument\":{{\"uri\":\"{}\"}},\"position\":{{\"line\":{},\"character\":{}}}}}", uri, line, col);
        self.lsp_request("completion", "textDocument/completion", params);
    }

    /// 帧首轮询 LSP 诊断（应用到当前打开的对应文件）
    fn poll_lsp(&mut self) {
        let Some(rx) = self.lsp_rx.as_ref() else { return };
        // 按 uri 聚合本轮收到的诊断
        let mut by_uri: std::collections::HashMap<String, Vec<Diag>> = std::collections::HashMap::new();
        loop {
            match rx.try_recv() {
                Ok((uri, code, payload)) => {
                    let mut parts = payload.splitn(4, '|');
                    let _line = parts.next().and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                    let _char = parts.next().and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                    let severity = parts.next().unwrap_or("error").to_string();
                    let msg = parts.next().unwrap_or("").to_string();
                    by_uri.entry(uri.clone()).or_default().push(Diag {
                        severity, code, message: msg, line: _line + 1, col: _char + 1, quick_fix: String::new(),
                    });
                }
                Err(_) => break,
            }
        }
        // 请求响应（hover/definition/completion 按 id 匹配）
        if let (Some((wid, tag)), Some(rx2)) = (self.lsp_wait, self.lsp_res_rx.as_ref()) {
            if let Ok((rid, body)) = rx2.try_recv() {
                if rid == wid {
                    self.lsp_wait = None;
                    match tag {
                        "definition" => {
                            // 首个 range.start.line/character → 跳转
                            let dl = json_num_field(&body, "line").unwrap_or(0) as usize;
                            let dc = json_num_field(&body, "character").unwrap_or(0) as usize;
                            if let Some(t) = self.active_tab() {
                                let content = t.content.clone();
                                let char_idx = content.split('\n').take(dl)
                                    .map(|l| l.chars().count() + 1).sum::<usize>()
                                    .saturating_sub(1) + dc;
                                self.pending_cursor_chars = Some(char_idx);
                            }
                            self.status = format!("跳转到 {} 行 {} 列", dl + 1, dc + 1);
                        }
                        "hover" => {
                            let v = json_string_field(&body, "value");
                            if !v.is_empty() {
                                self.hover_info = Some(v);
                                self.hover_sent_at = Some(std::time::Instant::now());
                            }
                        }
                        "completion" => {
                            self.completions.clear();
                            let mut rest = body.as_str();
                            while let Some(pos) = rest.find("\"label\":\"") {
                                let tail = &rest[pos + 9..];
                                let end = tail.find('"').unwrap_or(0);
                                if end == 0 { break; }
                                let label = &tail[..end];
                                if !label.is_empty() && !self.completions.contains(&label.to_string()) {
                                    self.completions.push(label.to_string());
                                }
                                rest = &tail[end..];
                                if self.completions.len() >= 60 { break; }
                            }
                            if !self.completions.is_empty() { self.show_completion = true; }
                        }
                        _ => {}
                    }
                }
            }
        }
        if by_uri.is_empty() { return; }
        // 应用到对应 tab（uri 末段与 tab 名匹配）
        for (uri, diags) in by_uri {
            let fname = uri.rsplit('/').next().unwrap_or("").to_string();
            if let Some(ti) = self.tabs.iter().position(|t| t.name == fname) {
                let n_err = diags.iter().filter(|d| d.severity == "error").count();
                let n_warn = diags.iter().filter(|d| d.severity == "warning").count();
                if ti == self.active_tab {
                    self.diags = diags;
                    self.n_errors = n_err;
                    self.n_warnings = n_warn;
                }
            }
        }
        if self.lsp_rx.is_some() { /* 保持通道 */ }
    }

    /// 后台执行命令，完成后经任务通道回传（UI 永不阻塞）
    fn spawn_task<F>(&mut self, label: &'static str, f: F)
    where F: FnOnce(std::sync::mpsc::Sender<TaskMsg>) + Send + 'static {
        let Some(tx) = self.task_tx.clone() else { return };
        self.task_busy = Some(label);
        std::thread::spawn(move || f(tx));
    }

    /// 帧首轮询后台任务结果
    fn poll_tasks(&mut self) {
        let mut msgs = Vec::new();
        {
            let Some(rx) = self.task_rx.as_ref() else { return };
            loop {
                match rx.try_recv() {
                    Ok(msg) => msgs.push(msg),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
        }
        if !msgs.is_empty() {
            for msg in msgs { self.apply_task(msg); }
            self.task_busy = None;
        }
    }

    fn apply_task(&mut self, msg: TaskMsg) {
        match msg {
            TaskMsg::Checked { stderr, diags } => {
                self.diags = diags;
                self.n_errors = self.diags.iter().filter(|d| d.severity == "error").count();
                self.n_warnings = self.diags.iter().filter(|d| d.severity == "warning").count();
                self.output_text = stderr;
            }
            TaskMsg::Built { text, ok } => {
                self.terminal_text.push_str(&text);
                self.bottom_tab = 2;
                self.toast(if ok { "构建完成" } else { "构建失败" });
            }
            TaskMsg::Ran { text } | TaskMsg::Term { text } => {
                self.terminal_text.push_str(&text);
                self.bottom_tab = 2;
            }
            TaskMsg::Git { output } => {
                self.output_text = output;
            }
        }
    }

    fn check(&mut self) {
        let Some(tab_name) = self.active_tab().map(|t| t.name.clone()) else { return };
        self.save(); // 检查磁盘上的最新内容
        let path = self.root.join("examples").join(&tab_name);
        let aine = self.root.join("target").join("debug").join("aine.exe");
        self.spawn_task("检查中", move |tx| {
            let out = std::process::Command::new(&aine).args(["check", &path.to_string_lossy()]).output();
            let (stderr, diags) = match out {
                Ok(o) => {
                    let s = String::from_utf8_lossy(&o.stderr).to_string();
                    let diags = parse_diagnostics(&s);
                    (s, diags)
                }
                Err(e) => (format!("aine check 启动失败: {}", e), Vec::new()),
            };
            let _ = tx.send(TaskMsg::Checked { stderr, diags });
        });
    }

    /// T34 Router：跨全部已配置厂商按任务能力选路（key 为空的跳过）
    fn route_entry_for(&self, task: &str) -> Option<ModelEntry> {
        let keyed: Vec<ModelEntry> = self.models.iter()
            .filter(|m| !m.api_key.trim().is_empty() && !m.id.is_empty())
            .cloned().collect();
        if keyed.is_empty() { return None; }
        keyed.iter().find(|m| m.capabilities.iter().any(|c| c == task)).cloned()
            .or_else(|| keyed.first().cloned())
    }

    fn send_ai_message(&mut self) {
        if self.ai_input.is_empty() { return; }
        let user_msg = self.ai_input.clone();
        self.ai_chat.push((true, user_msg.clone()));
        self.ai_input.clear();

        let task = self.pending_task;
        self.pending_task = "chat";
        let demo = self.ai_settings.demo;
        let route = self.route_entry_for(task);
        if !demo && route.is_none() {
            self.ai_chat.push((false, "尚未配置厂商：打开 Settings 添加厂商并填 API Key，或勾选演示模式体验流式。".into()));
            return;
        }
        let history = self.ai_chat.clone();
        // 系统上下文：当前文件名 + 内容（截断）
        let file_ctx = self.active_tab().map(|t| {
            (t.name.clone(), t.content.chars().take(6000).collect::<String>())
        });
        self.ai_cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        let cancel = self.ai_cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.ai_rx = Some(rx);
        self.ai_streaming = true;
        self.ai_chat.push((false, String::new())); // placeholder for AI response

        let route = route.unwrap_or_default();
        std::thread::spawn(move || {
            umber_chat(&route, demo, &history, &file_ctx, &cancel, &tx);
        });
    }

    fn poll_ai_response(&mut self) {
        let Some(rx) = self.ai_rx.take() else { return };
        loop {
            match rx.try_recv() {
                Ok(AiMsg::Delta(d)) => {
                    // 真流式：增量追加并继续排空本帧队列
                    if let Some(last) = self.ai_chat.last_mut() {
                        if !last.0 {
                            if last.1.starts_with("Thinking") { last.1.clear(); }
                            last.1.push_str(&d);
                        }
                    }
                }
                Ok(AiMsg::Reasoning(d)) => {
                    self.ai_reasoning.push_str(&d);
                }
                Ok(AiMsg::Usage(i, o)) => {
                    self.ai_usage = Some((i, o));
                }
                Ok(AiMsg::Done(text)) => {
                    if let Some(last) = self.ai_chat.last_mut() {
                        if !last.0 && !text.is_empty() { last.1 = text; }
                    }
                    self.ai_streaming = false;
                    self.ai_rx = Some(rx);
                    self.try_extract_fix();
                    return;
                }
                Ok(AiMsg::Error(e)) => {
                    if let Some(last) = self.ai_chat.last_mut() {
                        if !last.0 { last.1 = e; }
                    }
                    self.ai_streaming = false;
                    self.ai_rx = Some(rx);
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.ai_rx = Some(rx);
                    if self.ai_streaming {
                        if let Some(last) = self.ai_chat.last_mut() {
                            if !last.0 && last.1.is_empty() {
                                let dots = ".".repeat((std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as usize / 300) % 4);
                                last.1 = format!("Thinking{}", dots);
                            }
                        }
                    }
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ai_streaming = false;
                    return;
                }
            }
        }
    }

    /// ai/explain：把诊断发给 AI 解释（结果进聊天面板）
    fn ai_explain_diag(&mut self, idx: usize) {
        let Some(d) = self.diags.get(idx).cloned() else { return };
        let Some(tab) = self.active_tab() else { return };
        let src_line = tab.content.lines().nth(d.line.saturating_sub(1)).unwrap_or("").to_string();
        let msg = format!(
            "请解释这个 Aine 编译{}并给出修复建议：\n[{}{}] {}\n出错位置: 第{}行第{}列\n该行源码: {}\n（简要中文回答）",
            if d.severity == "error" { "错误" } else { "警告" },
            d.severity, d.code, d.message, d.line, d.col, src_line
        );
        self.ai_input = msg;
        self.ai_explaining = true;
        self.show_ai_panel = true;
        self.send_routed("explain");
        self.ai_explaining = false;
    }

    /// ai/fix：让 AI 修复诊断 → 产出 PendingDiff 等待审批（T36）
    fn ai_fix_diag(&mut self, idx: usize) {
        let Some(d) = self.diags.get(idx).cloned() else { return };
        let Some(tab) = self.active_tab() else { return };
        let file = tab.name.clone();
        let content = tab.content.clone();
        let src_line = content.lines().nth(d.line.saturating_sub(1)).unwrap_or("").to_string();
        let prompt = format!(
            "修复这个 Aine 编译错误。\n错误: [{}{}] {}\n位置: 第{}行第{}列\n该行源码: {}\n\n完整文件内容:\n```aine\n{}\n```\n\n请输出修复后的完整文件内容，用 ```aine 和 ``` 包裹，不要输出其他解释。",
            d.severity, d.code, d.message, d.line, d.col, src_line, content
        );
        self.ai_input = prompt;
        self.ai_explaining = true;
        self.show_ai_panel = true;
        self.send_routed("fix");
        self.ai_explaining = false;
        // 记录待修复文件，AI 回复到达后尝试解析出 diff
        self.fixing_file = Some(file);
        self.fixing_old = Some(content);
        self.fixing_reason = Some(format!("[{}{}] {}", d.severity, d.code, d.message));
    }

    /// AI 回复到达后：若在修复模式，解析代码块 → PendingDiff（T36→T37）
    fn try_extract_fix(&mut self) {
        let (Some(file), Some(old), Some(reason)) =
            (&self.fixing_file, &self.fixing_old, &self.fixing_reason) else { return };
        let Some(last) = self.ai_chat.last() else { return };
        if last.0 { return } // 还不是 AI 回复
        let text = &last.1;
        if self.ai_streaming { return } // 还在流式输出
        // 提取 ```aine ... ``` 代码块
        let new_content = extract_code_block(text);
        if let Some(nc) = new_content {
            if nc != *old {
                self.pending_diff = Some(PendingDiff {
                    file: file.clone(),
                    old_content: old.clone(),
                    new_content: nc,
                    reason: reason.clone(),
                });
            }
        }
        // 无论成功与否，结束修复模式
        self.fixing_file = None;
        self.fixing_old = None;
        self.fixing_reason = None;
    }

    /// 审批通过 → 应用 diff → 自动 check 反馈（T38 闭环）
    fn apply_pending_diff(&mut self) {
        if let Some(pd) = self.pending_diff.take() {
            if let Some(ti) = self.tabs.iter().position(|t| t.name == pd.file) {
                self.tabs[ti].content = pd.new_content;
                self.tabs[ti].dirty = true;
                self.active_tab = ti;
                self.save();
                self.check();
                if self.n_errors == 0 {
                    self.ai_chat.push((false, "✅ 修复已应用，编译通过（0 errors）".into()));
                } else {
                    self.ai_chat.push((false, format!("⚠️ 修复已应用，但仍有 {} 个错误，可继续 [Fix]", self.n_errors)));
                }
            }
        }
    }

    /// 大纲：当前文件符号（函数/struct/enum 等）
    fn refresh_outline(&mut self) {
        if let Some(t) = self.active_tab() {
            let (content, name) = (t.content.clone(), t.name.clone());
            let path = self.root.join("examples").join(&name);
            let ps = path.to_string_lossy().to_string();
            self.outline = aine::lsp::ide_symbols(&content, &ps);
        }
    }

    /// 全工程搜索（当前打开文件集 = examples 下所有 .aine）
    fn project_search(&mut self) {
        self.proj_results.clear();
        let q = self.proj_search.trim().to_string();
        if q.is_empty() { return; }
        let files = self.files.clone();
        for f in files {
            let path = self.root.join("examples").join(&f);
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(&q) {
                        self.proj_results.push((f.clone(), i + 1, line.trim().to_string()));
                        if self.proj_results.len() >= 200 { return; }
                    }
                }
            }
        }
    }

    /// Git 状态刷新（git status --porcelain + diff --stat）
    fn git_refresh(&mut self) {
        let root = self.root.clone();
        self.spawn_task("Git", move |tx| {
            let mut out = String::from("$ git status --porcelain\n");
            match std::process::Command::new("git").args(["status", "--porcelain"]).current_dir(&root).output() {
                Ok(o) => {
                    out.push_str(&String::from_utf8_lossy(&o.stdout));
                    out.push_str("\n$ git diff --stat\n");
                    match std::process::Command::new("git").args(["diff", "--stat"]).current_dir(&root).output() {
                        Ok(o2) => out.push_str(&String::from_utf8_lossy(&o2.stdout)),
                        Err(e) => out.push_str(&format!("(diff error: {})", e)),
                    }
                }
                Err(e) => out.push_str(&format!("(git not available: {})", e)),
            }
            let _ = tx.send(TaskMsg::Git { output: out });
        });
    }

    /// git stage 全部变更
    fn git_stage_all(&mut self) {
        let root = self.root.clone();
        self.spawn_task("Git", move |tx| {
            let out = std::process::Command::new("git").args(["add", "-A"]).current_dir(&root).output();
            let text = match out {
                Ok(o) if o.status.success() => "staged all changes\n".to_string(),
                Ok(o) => format!("stage error: {}\n", String::from_utf8_lossy(&o.stderr)),
                Err(e) => format!("git error: {}\n", e),
            };
            let _ = tx.send(TaskMsg::Term { text });
        });
        self.toast("git add -A");
    }

    /// git commit（UI 输入消息，后台执行）
    fn git_commit(&mut self, msg: &str) {
        let root = self.root.clone();
        let m = msg.trim().to_string();
        if m.is_empty() { self.toast("commit 消息为空"); return; }
        self.spawn_task("Git", move |tx| {
            let out = std::process::Command::new("git").args(["commit", "-m", &m]).current_dir(&root).output();
            let text = match out {
                Ok(o) => {
                    let mut t = String::from_utf8_lossy(&o.stdout).to_string();
                    t.push_str(&String::from_utf8_lossy(&o.stderr));
                    t
                }
                Err(e) => format!("git error: {}\n", e),
            };
            let _ = tx.send(TaskMsg::Term { text });
        });
        self.toast("git commit");
    }

    /// 调试：aine debug 当前文件 + 断点行
    fn run_debugger(&mut self) {
        let Some(tab_name) = self.active_tab().map(|t| t.name.clone()) else { return };
        if self.breakpoints.is_empty() {
            self.toast("先右键点击行号设断点");
            return;
        }
        {
            self.save();
            let path = self.root.join("examples").join(&tab_name);
            let aine = self.root.join("target").join("debug").join("aine.exe");
            let bps = self.breakpoints.clone();
            self.spawn_task("调试中", move |tx| {
                let mut cmd_args = vec!["debug".to_string(), path.to_string_lossy().to_string()];
                for b in &bps { cmd_args.push(b.to_string()); }
                let out = std::process::Command::new(&aine).args(&cmd_args).output();
                let text = match out {
                    Ok(o) => {
                        let mut t = String::from_utf8_lossy(&o.stdout).to_string();
                        t.push_str(&String::from_utf8_lossy(&o.stderr));
                        t
                    }
                    Err(e) => format!("debug error: {}\n", e),
                };
                let _ = tx.send(TaskMsg::Ran { text });
            });
        }
    }

    /// 运行当前文件的 #[test] 函数（aine test）
    fn run_tests(&mut self) {
        if let Some(tab) = self.active_tab() {
            let path = self.root.join("examples").join(&tab.name);
            let aine = self.root.join("target").join("debug").join("aine.exe");
            let out = Command::new(&aine).args(["test", &path.to_string_lossy()]).output();
            self.test_output = match out {
                Ok(o) => {
                    let mut s = format!("$ aine test {}\n", tab.name);
                    s.push_str(&String::from_utf8_lossy(&o.stdout));
                    let err = String::from_utf8_lossy(&o.stderr);
                    if !err.is_empty() { s.push_str(&err); }
                    s
                }
                Err(e) => format!("test error: {}", e),
            };
        }
    }

    /// 当前文件路径（绝对）
    fn active_path(&self) -> Option<PathBuf> {
        self.active_tab().map(|t| self.root.join("examples").join(&t.name))
    }

    /// F12 / 命令：跳到光标处符号定义（移动 egui 编辑器光标）
    fn goto_definition(&mut self) {
        let (content, byte) = {
            let t = match self.active_tab() { Some(t) => t, None => return };
            (t.content.clone(), t.cursor_byte.unwrap_or(0))
        };
        let path = match self.active_path() {
            Some(p) => p.to_string_lossy().to_string(), None => return,
        };
        if let Some((def_line, def_col)) = aine::lsp::ide_definition(&content, &path, byte) {
            // 转 egui 字符索引并设置编辑器光标
            let byte_off = content.lines().take(def_line).map(|l| l.len() + 1).sum::<usize>()
                .saturating_sub(if def_line > 0 { 1 } else { 0 })
                + def_col.min(content.lines().nth(def_line).map(|l| l.len()).unwrap_or(0));
            let char_idx = content[..byte_off.min(content.len())].chars().count();
            self.status = format!("Definition: line {}", def_line + 1);
            self.pending_cursor_chars = Some(char_idx);
        } else {
            self.status = "No definition found at cursor".into();
        }
    }

    /// Shift+F12 / 命令：查找所有引用 → 显示在搜索面板
    fn find_references(&mut self) {
        let (content, byte) = {
            let t = match self.active_tab() { Some(t) => t, None => return };
            (t.content.clone(), t.cursor_byte.unwrap_or(0))
        };
        let path = match self.active_path() {
            Some(p) => p.to_string_lossy().to_string(), None => return,
        };
        match aine::lsp::ide_references(&content, &path, byte) {
            Some(refs) => {
                let name_end = content[byte.min(content.len())..]
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| byte + i).unwrap_or(content.len());
                let name_start = content[..byte.min(content.len())].rfind(|c: char| !c.is_alphanumeric() && c != '_').map(|i| i + 1).unwrap_or(0);
                let name = &content[name_start.min(name_end)..name_end.max(name_start)];
                let mut out = format!("{} references to `{}`:\n", refs.len(), name);
                for (l, c) in refs.iter().take(100) {
                    out.push_str(&format!("  Ln {}, Col {}\n", l + 1, c + 1));
                }
                self.output_text = out;
                self.bottom_tab = 1;
                self.show_problems = true;
                self.status = format!("Found {} references", refs.len());
            }
            None => { self.status = "No symbol at cursor".into(); }
        }
    }

    /// 命令：格式化当前文档（aine fmt）
    fn format_document(&mut self) {
        let path = match self.active_path() {
            Some(p) => p, None => return,
        };
        self.save();
        let aine = self.root.join("target").join("debug").join("aine.exe");
        if let Ok(out) = std::process::Command::new(&aine).arg("fmt").arg(&path).output() {
            if out.status.success() {
                let formatted = String::from_utf8_lossy(&out.stdout).to_string();
                if !formatted.trim().is_empty() {
                    if let Some(t) = self.active_tab_mut() {
                        t.content = formatted;
                        t.dirty = true;
                    }
                }
                self.status = "Formatted".into();
            } else {
                self.status = "Format failed (code has errors?)".into();
            }
        }
    }

    fn do_replace_all(&mut self, search: &str, replace: &str) {
        if let Some(tab) = self.active_tab_mut() {
            if !search.is_empty() {
                tab.content = tab.content.replace(search, replace);
                tab.dirty = true;
            }
        }
    }
}

// ── UI ──
impl App {
    /// 扫描目录树进缓存（2 秒 TTL，新建/删除文件时强制重扫）
    fn rescan_fs(&mut self) {
        let examples = self.root.join("examples");
        let node = Self::scan_dir(&examples, "examples");
        let mut files = Vec::new();
        Self::collect_files(&node, "", &mut files);
        files.sort();
        self.files = files;
        self.fs_root = Some(node);
        self.fs_scanned_at = Some(std::time::Instant::now());
    }

    fn scan_dir(dir: &PathBuf, name: &str) -> FsDir {
        let mut node = FsDir { name: name.into(), path: dir.clone(), ..Default::default() };
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                let p = e.path();
                if p.is_dir() {
                    if n == "target" || n == ".git" || n == "node_modules" { continue; }
                    node.dirs.push(Self::scan_dir(&p, &n));
                    node.dirs.sort_by(|a, b| a.name.cmp(&b.name));
                } else if n.ends_with(".aine") {
                    node.files.push(n);
                    node.files.sort();
                }
            }
        }
        node
    }

    fn collect_files(node: &FsDir, prefix: &str, out: &mut Vec<String>) {
        for f in &node.files { out.push(format!("{}{}", prefix, f)); }
        for d in &node.dirs { Self::collect_files(d, &format!("{}{}/", prefix, d.name), out); }
    }

    /// 渲染缓存的文件树（无磁盘 IO）
    fn render_fs(&mut self, ui: &mut egui::Ui, node: &FsDir, prefix: &str, depth: usize) {
        let node = node.clone(); // 断开借用以便回调 open_file
        for d in &node.dirs {
            let is_collapsed = self.collapsed.contains(&d.path);
            let arrow = if is_collapsed { "▶" } else { "▼" };
            let indent = 8 + depth * 14;
            ui.horizontal(|ui| {
                ui.add_space(indent as f32);
                if ui.add(egui::Button::new(
                    egui::RichText::new(format!("{}  {}", arrow, d.name))
                        .color(theme::FG).size(12.0)
                ).frame(false).min_size(egui::vec2(ui.available_width(), 20.0))).clicked() {
                    if is_collapsed { self.collapsed.remove(&d.path); }
                    else { self.collapsed.insert(d.path.clone()); }
                }
            });
            if !is_collapsed {
                self.render_fs(ui, d, &format!("{}{}/", prefix, d.name), depth + 1);
            }
        }
        for f in &node.files {
            let rel = format!("{}{}", prefix, f);
            let indent = 8 + depth * 14;
            let file_idx = self.files.iter().position(|x| *x == rel);
            let active_name = self.active_tab().map(|t| t.name.clone()).unwrap_or_default();
            let is_sel = active_name == rel;
            let is_open = self.tabs.iter().any(|t| t.name == rel);
            let c = if is_sel { theme::ACCENT } else if is_open { theme::FG } else { theme::FG_DIM };
            let icon = if is_sel { "●" } else if is_open { "○" } else { " " };
            let fname = f.clone();
            if ui.horizontal(|ui| {
                ui.add_space(indent as f32);
                ui.add(egui::Button::new(
                    egui::RichText::new(format!("{}  {}", icon, fname)).color(c).size(12.0)
                ).frame(false).min_size(egui::vec2(ui.available_width(), 20.0)))
            }).inner.clicked() {
                if let Some(idx) = file_idx {
                    self.open_file(idx);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll AI streaming response + background tasks
        self.poll_ai_response();
        if self.ai_streaming { ctx.request_repaint(); }
        self.poll_tasks();
        if self.task_busy.is_some() { ctx.request_repaint(); }
        self.poll_lsp();
        // hover：光标静止 800ms 后经 LSP 请求精确类型提示
        {
            let cur = self.active_tab().map(|t| (t.cursor_line, t.cursor_col, t.content.len()));
            match (cur, self.hover_idle) {
                (Some(_), None) => { self.hover_idle = Some(std::time::Instant::now()); self.hover_sent_at = None; }
                (Some((l, c, n)), Some(t0)) => {
                    let moved = self.hover_sent_at.is_none() && {
                        // 变化检测：cursor 与上一帧不同则重置
                        false
                    };
                    let _ = (l, c, n);
                    if self.hover_sent_at.is_none()
                        && self.lsp_wait.is_none()
                        && t0.elapsed().as_millis() > 800
                    {
                        self.hover_sent_at = Some(std::time::Instant::now());
                        self.lsp_hover();
                    }
                }
                _ => {}
            }
        }

        // 提前 clone 避免 borrow 冲突
        let root = self.root.clone();
        let lang = self.lang_idx;
        let mut show_problems = self.show_problems;
        let mut search_text = self.search_text.clone();
        let mut replace_text = self.replace_text.clone();
        let mut show_search = self.show_search;
        let mut show_quick_open = self.show_quick_open;
        let mut quick_filter = self.quick_open_filter.clone();
        let mut show_command_bar = self.show_command_bar;
        let mut command_filter = self.command_filter.clone();
        let mut command_action: Option<Cmd> = None;
        let show_ai_panel = self.show_ai_panel;
        let mut explain_idx: Option<usize> = None;
        let mut fix_idx: Option<usize> = None;
        let mut apply_diff = false;
        let mut reject_diff = false;
        let mut search_now = false;
        let mut git_now = false;
        let mut test_now = false;
        let mut find_next_now = false;
        let mut show_goto = self.show_goto;
        let mut goto_input = self.goto_input.clone();
        let mut show_rename = self.show_rename;
        let mut rename_input = self.rename_input.clone();
        let mut show_completion = self.show_completion;
        let mut completions = self.completions.clone();
        let mut commit_msg = self.git_commit_msg.clone();
        let mut dropped_check = false;
        let mut confirm_yes = false;
        let mut cancel_confirm = false;
        let mut show_model_picker = self.show_model_picker;
        let mut model_filter = self.model_filter.clone();
        let mut save_models_flag = false;
        let focus = self.focus_mode;
        let old_preview = self.pending_diff.as_ref().map(|d| d.old_content.clone()).unwrap_or_default();
        let new_preview = self.pending_diff.as_ref().map(|d| d.new_content.clone()).unwrap_or_default();
        let mut show_ai_settings = self.show_ai_settings;
        let mut bottom_tab = self.bottom_tab;
        let n_err = self.n_errors;
        let n_warn = self.n_warnings;
        let active_idx = self.active_tab;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) { self.check(); }
            if i.key_pressed(egui::Key::F6) { self.run_prog(); }
            if i.key_pressed(egui::Key::F7) { self.build(); }
            if i.key_pressed(egui::Key::F12) {
                if self.lsp_stdin.is_some() { self.lsp_goto_definition(); }
                else { self.goto_definition(); }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Space) { self.lsp_completion(); }
            if i.modifiers.shift && i.key_pressed(egui::Key::F12) { self.find_references(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Backtick) { self.show_problems = !self.show_problems; self.bottom_tab = 2; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) { self.show_ai_panel = !self.show_ai_panel; }
            if i.modifiers.ctrl && i.modifiers.alt && i.key_pressed(egui::Key::F) { self.focus_mode = !self.focus_mode; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Slash) { self.toggle_comment(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) { show_goto = true; }
            if i.key_pressed(egui::Key::F9) { app_or_self_debug(self); }
            if i.key_pressed(egui::Key::F2) {
                if let Some(t) = self.active_tab() {
                    let b = t.cursor_byte.unwrap_or(0).min(t.content.len());
                    let is_w = |c: char| c.is_alphanumeric() || c == '_';
                    let st = t.content[..b].rfind(|c: char| !is_w(c)).map(|x| x + 1).unwrap_or(0);
                    let en = t.content[b..].find(|c: char| !is_w(c)).map(|x| b + x).unwrap_or(t.content.len());
                    self.rename_input = t.content[st..en].to_string();
                }
                show_rename = true;
            }
            if self.focus_mode && i.key_pressed(egui::Key::Escape) { self.focus_mode = false; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) { self.save(); }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) { self.show_search = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::P) { self.show_quick_open = true; }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::K) { self.show_command_bar = true; }
            if i.key_pressed(egui::Key::Escape) {
                self.show_search = false;
                self.show_quick_open = false;
                self.show_command_bar = false;
                self.show_goto = false;
                self.confirm = None;
                self.show_completion = false;
            }
        });

        // 保存搜索替换结果
        let (search_in, replace_in) = (search_text.clone(), replace_text.clone());
        self.search_text = search_in.clone();
        self.replace_text = replace_in.clone();

        let lang = self.lang_idx;

        // ── 顶栏 ──
        egui::TopBottomPanel::top("titlebar")
            .exact_height(28.0)
            .frame(egui::Frame::none().fill(theme::BG_BAR))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Aine Studio").color(theme::ACCENT).size(13.0));
                    ui.add_space(12.0);

                    // File 菜单
                    ui.menu_button(tr("file", lang), |ui| {
                        if ui.button(tr("new_file", lang)).clicked() {
                            let n = self.files.len() + 1;
                            self.new_file(&format!("file_{}", n));
                            ui.close_menu();
                        }
                        if ui.button(tr("save_file", lang)).clicked() { self.save(); ui.close_menu(); }
                        ui.separator();
                        if ui.button(tr("quick_open", lang)).clicked() { self.show_quick_open = true; ui.close_menu(); }
                        if ui.button(tr("command_palette", lang)).clicked() { self.show_command_bar = true; ui.close_menu(); }
                        ui.separator();
                        if ui.button(tr("close_file", lang)).clicked() { self.request_close_tab(self.active_tab); ui.close_menu(); }
                        if ui.button(tr("delete_file", lang)).clicked() {
                            if let Some(tab) = self.active_tab() {
                                self.confirm = Some(Confirm::DeleteFile(tab.name.clone()));
                            }
                            ui.close_menu();
                        }
                    });

                    // Edit 菜单（真实动作；撤销/重做为编辑器内建）
                    ui.menu_button(tr("edit", lang), |ui| {
                        ui.label(egui::RichText::new(if lang == 1 { "撤销 Ctrl+Z / 重做 Ctrl+Y（编辑器内建）" } else { "Undo Ctrl+Z / Redo Ctrl+Y (built-in)" }).color(theme::FG_DIM).size(11.0));
                        ui.separator();
                        if ui.button(tr("toggle_comment", lang)).clicked() {
                            self.toggle_comment(); ui.close_menu();
                        }
                        if ui.button(tr("duplicate_line", lang)).clicked() {
                            self.duplicate_line(); ui.close_menu();
                        }
                        if ui.button(tr("goto_line_menu", lang)).clicked() {
                            show_goto = true; ui.close_menu();
                        }
                        if ui.button(if lang == 1 { "重命名符号...  F2" } else { "Rename Symbol...  F2" }).clicked() {
                            show_rename = true; ui.close_menu();
                        }
                        if ui.button(tr("find_next", lang)).clicked() {
                            find_next_now = true; ui.close_menu();
                        }
                    });

                    // View 菜单
                    ui.menu_button(tr("view", lang), |ui| {
                        if ui.button(tr("toggle_problems", lang)).clicked() {
                            show_problems = !show_problems;
                            ui.close_menu();
                        }
                        if ui.button(tr("ai_panel", lang)).clicked() {
                            self.show_ai_panel = !self.show_ai_panel;
                            ui.close_menu();
                        }
                        if ui.button(tr("focus_mode", lang)).clicked() {
                            self.focus_mode = !self.focus_mode;
                            ui.close_menu();
                        }
                        if ui.button(if lang == 1 { "Veil 双视图预览" } else { "Veil dual-view preview" }).clicked() {
                            self.show_veil_preview = !self.show_veil_preview;
                            ui.close_menu();
                        }
                        ui.separator();
                        // Language Veil：编辑器表面语言（仅显示层，保存恒 canonical）
                        ui.label(egui::RichText::new(if lang == 1 { "Language Veil 表面：" } else { "Language Veil surface:" }).color(theme::FG_DIM).size(11.0));
                        for (vi, vname) in aine::veil::SURFACE_NAMES.iter().enumerate() {
                            let cur = self.active_tab().map(|t| t.surface).unwrap_or(1);
                            let mark = if cur == vi { "●" } else { "○" };
                            if ui.button(egui::RichText::new(format!("{} {}", mark, vname)).size(12.0)).clicked() {
                                if let Some(t) = self.active_tab_mut() { t.surface = vi; }
                                ui.close_menu();
                            }
                        }
                    });

                    // Help 菜单
                    ui.menu_button(tr("help", lang), |ui| {
                        ui.label(egui::RichText::new("Aine Studio v0.2").color(theme::FG_DIM).size(11.0));
                        ui.separator();
                        ui.label(egui::RichText::new(tr("shortcuts_title", lang)).color(theme::FG).size(11.0));
                        for (k, d) in [
                            ("Ctrl+S", if lang == 1 { "保存" } else { "Save" }),
                            ("F5", if lang == 1 { "检查" } else { "Check" }),
                            ("F7", if lang == 1 { "构建" } else { "Build" }),
                            ("F6", if lang == 1 { "运行" } else { "Run" }),
                            ("Ctrl+P", if lang == 1 { "快速打开" } else { "Quick Open" }),
                            ("Ctrl+K", if lang == 1 { "命令面板" } else { "Commands" }),
                            ("Ctrl+F", if lang == 1 { "搜索替换" } else { "Search & Replace" }),
                            ("F12", if lang == 1 { "转到定义" } else { "Go to Definition" }),
                            ("Shift+F12", if lang == 1 { "查找引用" } else { "Find References" }),
                            ("Ctrl+/", if lang == 1 { "注释当前行" } else { "Comment line" }),
                            ("Ctrl+`", if lang == 1 { "终端" } else { "Terminal" }),
                            ("Ctrl+I", if lang == 1 { "AI 面板" } else { "AI Panel" }),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(k).color(theme::ACCENT).size(11.0).monospace());
                                ui.label(egui::RichText::new(d).color(theme::FG_DIM).size(11.0));
                            });
                        }
                    });

                    ui.add_space(8.0);

                    // Run/Build/Check/Save 按钮（扁平，悬停微亮）
                    let btn = |ui: &mut egui::Ui, label: &str| -> bool {
                        ui.add(egui::Button::new(
                            egui::RichText::new(label).color(theme::FG).size(12.0)
                        ).min_size(egui::vec2(56.0, 22.0))).clicked()
                    };
                    if btn(ui, tr("run", lang)) { self.run_prog(); }
                    if btn(ui, tr("build", lang)) { self.build(); }
                    if btn(ui, tr("check", lang)) { self.check(); }
                    if btn(ui, tr("save_file", lang)) { self.save(); }

                    // AI 设置按钮
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(
                            egui::RichText::new("AI").color(theme::FG).size(12.0)
                        ).min_size(egui::vec2(36.0, 22.0))).clicked() {
                            self.show_ai_panel = !self.show_ai_panel;
                        }
                        // 语言
                        ui.menu_button(LANGS[lang], |ui| {
                            // 6 语言完整翻译（LANG_TABLE）
                            for (i, name) in LANGS.iter().enumerate() {
                                let c = if i == lang { theme::ACCENT } else { theme::FG };
                                if ui.button(egui::RichText::new(*name).color(c).size(12.0)).clicked() {
                                    self.lang_idx = i;
                                    self.save_settings();
                                    ui.close_menu();
                                }
                            }
                        });
                    });
                });
            });

        // ── AI 设置弹窗 ──
        let mut close_ai_settings = false;
        let mut save_settings_flag = false;
        if show_ai_settings {
            if self.settings_idx >= self.models.len() { self.settings_idx = 0; }
            egui::Window::new(tr("settings", lang))
                .default_width(430.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(if lang == 1 { "厂商部署（Umber 多 Deployment 并存）" } else { "Vendor deployments (multi-deployment)" }).color(theme::FG_DIM).size(11.0));
                    let ids: Vec<String> = self.models.iter().map(|m| m.id.clone()).collect();
                    egui::ComboBox::from_id_source("vendor_sel")
                        .selected_text(ids.get(self.settings_idx).map(|s| s.as_str()).unwrap_or("-"))
                        .show_ui(ui, |ui| {
                            for (i, id) in ids.iter().enumerate() {
                                ui.selectable_value(&mut self.settings_idx, i, id);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.small_button(if lang == 1 { "＋ 新增厂商" } else { "+ Add vendor" }).clicked() {
                            let mut n = self.models.len() + 1;
                            while self.models.iter().any(|m| m.id == format!("vendor{}", n)) { n += 1; }
                            self.models.push(ModelEntry {
                                id: format!("vendor{}", n),
                                provider: "custom".into(), protocol: "openai_chat".into(),
                                base_url: String::new(), model: String::new(), api_key: String::new(),
                                capabilities: vec!["chat".into()],
                            });
                            self.settings_idx = self.models.len() - 1;
                        }
                        let del = ui.small_button(if lang == 1 { "删除" } else { "Delete" }).clicked();
                        if del && !self.models.is_empty() {
                            self.models.remove(self.settings_idx);
                            self.settings_idx = 0;
                        }
                    });
                    ui.separator();
                    if let Some(m) = self.models.get_mut(self.settings_idx) {
                        let mut caps = m.capabilities.join(",");
                        egui::Grid::new("vendor_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                            ui.label("id");
                            ui.text_edit_singleline(&mut m.id);
                            ui.end_row();
                            ui.label(tr("provider", lang));
                            ui.text_edit_singleline(&mut m.provider);
                            ui.end_row();
                            ui.label("protocol");
                            egui::ComboBox::from_id_source("proto_sel")
                                .selected_text(&m.protocol)
                                .show_ui(ui, |ui| {
                                    for pr in ["openai_chat", "openai_responses", "anthropic_messages", "gemini", "provider_native"] {
                                        ui.selectable_value(&mut m.protocol, pr.to_string(), pr);
                                    }
                                });
                            ui.end_row();
                            ui.label("Base URL");
                            ui.add(egui::TextEdit::singleline(&mut m.base_url).desired_width(240.0));
                            ui.end_row();
                            ui.label(tr("model", lang));
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut m.model).desired_width(150.0));
                                if ui.small_button(if lang == 1 { "从目录选择…" } else { "Catalog..." }).clicked() {
                                    show_model_picker = true;
                                    model_filter.clear();
                                }
                            });
                            ui.end_row();
                            ui.label(tr("api_key", lang));
                            ui.add(egui::TextEdit::singleline(&mut m.api_key).password(true).desired_width(240.0));
                            ui.end_row();
                            ui.label("capabilities");
                            ui.text_edit_singleline(&mut caps);
                            ui.end_row();
                        });
                        m.capabilities = caps.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.ai_settings.demo, tr("demo_mode", lang));
                    // Umber runtime_status（部署/目录条目）
                    if let Some(api) = umber::global() {
                        unsafe {
                            let mut ev = umber::UmerEvent { sequence: 0, json: std::ptr::null_mut(), json_len: 0 };
                            if (api.status_fn)(api.rt, &mut ev) == umber::OK && !ev.json.is_null() {
                                let js = std::str::from_utf8(std::slice::from_raw_parts(ev.json as *const u8, ev.json_len)).unwrap_or("").to_string();
                                (api.string_free)(ev.json);
                                ui.label(egui::RichText::new(js).color(theme::FG_DIM).size(10.0).monospace());
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("umber_ffi.dll not loaded").color(theme::RED).size(10.0));
                    }
                    ui.add_space(6.0);
                    if ui.button(tr("confirm_ok", lang)).clicked() {
                        show_ai_settings = false;
                        save_settings_flag = true;
                        save_models_flag = true;
                    }
                });
        }

        // ── catalog 模型选择器（Umber 目录，惰性解析）──
        if show_model_picker && !self.models.is_empty() {
            egui::Window::new(if lang == 1 { "模型目录（Umber catalog）" } else { "Model Catalog (Umber)" })
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(if lang == 1 { "筛选:" } else { "Filter:" });
                        ui.add(egui::TextEdit::singleline(&mut model_filter).desired_width(220.0));
                        ui.label(egui::RichText::new(format!("{} deployments", catalog_entries().len())).color(theme::FG_DIM).size(10.0));
                    });
                    ui.separator();
                    let flt = model_filter.to_lowercase();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let mut shown = 0;
                        for c in catalog_entries().iter() {
                            if shown >= 200 { ui.label("..."); break; }
                            let full = format!("{}/{}", c.provider, c.model_id);
                            if !flt.is_empty() && !full.to_lowercase().contains(&flt) { continue; }
                            if ui.add(egui::Button::new(egui::RichText::new(&full).size(11.0).monospace())
                                .frame(false).min_size(egui::vec2(ui.available_width(), 16.0))).clicked() {
                                if let Some(m) = self.models.get_mut(self.settings_idx) { m.model = c.model_id.clone(); }
                                show_model_picker = false;
                            }
                            shown += 1;
                        }
                    });
                });
        }

        if close_ai_settings { self.show_ai_settings = false; }
        if save_settings_flag { self.save_settings(); }

        // ── 状态栏 ──
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(24.0)
            .frame(egui::Frame::none().fill(theme::BG_STATUS))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    // 真实文件名 + 脏标记
                    let (tab_name, dirty, eol, n_lines) = self.active_tab()
                        .map(|t| (t.name.clone(), t.dirty,
                            if t.content.contains("\r\n") { "CRLF" } else { "LF" },
                            t.content.split('\n').count()))
                        .unwrap_or((tr("no_file", lang).to_string(), false, "-", 0));
                    ui.label(egui::RichText::new(format!("{}{}", tab_name, if dirty { " ●" } else { "" }))
                        .color(if dirty { theme::YELLOW } else { theme::FG_STATUS }).size(12.0));
                    ui.add_space(16.0);
                    let (w_color, w_text) = if n_err > 0 { (theme::RED, format!("x {}  ! {}", n_err, n_warn)) }
                        else if n_warn > 0 { (theme::YELLOW, format!("⚠ {}", n_warn)) }
                        else { (theme::GREEN, format!("√ {}", if lang == 1 { "无问题" } else { "OK" })) };
                    ui.label(egui::RichText::new(w_text).color(w_color).size(12.0));
                    // 后台任务忙碌指示
                    if let Some(task) = self.task_busy {
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(format!("⏳ {}", task)).color(theme::ACCENT).size(12.0));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(
                            aine::veil::SURFACE_NAMES[self.active_tab().map(|t| t.surface).unwrap_or(1)]
                        ).color(theme::FG_STATUS).size(12.0));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(eol).color(theme::FG_STATUS).size(12.0));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(format!("{} {}", n_lines, tr("lines_unit", lang))).color(theme::FG_STATUS).size(12.0));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("UTF-8").color(theme::FG_STATUS).size(12.0));
                        ui.add_space(16.0);
                        let (l, c) = self.active_tab()
                            .map(|t| (t.cursor_line, t.cursor_col))
                            .unwrap_or((0, 0));
                        ui.label(egui::RichText::new(format!("Ln {}, Col {}", l + 1, c + 1)).color(theme::FG_STATUS).size(12.0));
                    });
                });
            });

        // ── 活动栏（图标居中、悬停提示、右侧 1px 边框 — 对标 VSCode Activity Bar）──
        if !focus {
        egui::SidePanel::left("activity_bar")
            .exact_width(44.0)
            .frame(egui::Frame::none()
                .fill(theme::BG_BAR)
                .stroke(egui::Stroke::new(1.0, theme::BORDER)))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                let tips = [
                    ("📁", tr("explorer", lang)),
                    ("🔍", tr("search", lang)),
                    ("🌿", "Git"),
                    ("📋", if lang == 1 { "大纲" } else { "Outline" }),
                ];
                for (i, (icon, tip)) in tips.iter().enumerate() {
                    let is_sel = self.sidebar_view == i;
                    // 居中：留出 (44-36)/2 = 4px
                    ui.add_space(4.0);
                    let btn = egui::Button::new(egui::RichText::new(*icon).size(17.0))
                        .min_size(egui::vec2(36.0, 34.0));
                    let resp = ui.add(btn).on_hover_text(*tip);
                    if resp.clicked() {
                        self.sidebar_view = i;
                        if i == 2 { self.git_refresh(); }
                        if i == 3 { self.refresh_outline(); }
                    }
                    if is_sel {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(resp.rect.left_top(), egui::vec2(3.0, resp.rect.height())),
                            1.0, theme::ACCENT,
                        );
                    }
                    ui.add_space(4.0);
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.add_space(4.0);
                let ai_resp = ui.add(egui::Button::new(egui::RichText::new("🤖").size(17.0))
                    .min_size(egui::vec2(36.0, 34.0)))
                    .on_hover_text(format!("AI ({})", "Ctrl+I"));
                if ai_resp.clicked() {
                    self.show_ai_panel = !self.show_ai_panel;
                }
            });
        }

        // ── 左侧边栏（视图随活动栏切换）──
        if !focus {
        let side_view = self.sidebar_view;
        let proj_q = self.proj_search.clone();
        let proj_results = self.proj_results.clone();
        egui::SidePanel::left("sidebar")
            .default_width(self.side_width)
            .resizable(true)
            .frame(egui::Frame::none().fill(theme::BG_SIDE))
            .show(ctx, |ui| {
                match side_view {
                    0 => { // Explorer（缓存渲染 + 工具条）
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(
                                format!("{}", tr("explorer", lang))
                            ).color(theme::FG_DIM).size(11.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("📄").size(12.0))
                                    .frame(false)).on_hover_text(tr("new_file_tip", lang)).clicked() {
                                    let n = self.files.len() + 1;
                                    self.new_file(&format!("file_{}", n));
                                    self.fs_scanned_at = None;
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("🔄").size(12.0))
                                    .frame(false)).on_hover_text(tr("refresh_tip", lang)).clicked() {
                                    self.rescan_fs();
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        // 缓存 2 秒 TTL；新建/删除文件时强制重扫
                        let stale = self.fs_root.is_none()
                            || self.fs_scanned_at.map(|t| t.elapsed().as_millis() > 2000).unwrap_or(true);
                        if stale { self.rescan_fs(); }
                        if let Some(root_node) = self.fs_root.clone() {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.add_space(4.0);
                                self.render_fs(ui, &root_node, "", 0);
                            });
                        }
                    }
                    1 => { // Search（全工程搜索）
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("  SEARCH").color(theme::FG_DIM).size(11.0).strong());
                        ui.add_space(4.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut self.proj_search)
                                .hint_text("Search workspace...").desired_width(ui.available_width() - 8.0));
                            if resp.changed() || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                search_now = true;
                            }
                        });
                        ui.label(egui::RichText::new(
                            format!("  {} results", proj_results.len())
                        ).color(theme::FG_DIM).size(10.0));
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut last_file = String::new();
                            for (f, line, text) in &proj_results {
                                if *f != last_file {
                                    ui.label(egui::RichText::new(format!("  {}", f)).color(theme::FG).size(11.0).strong());
                                    last_file = f.clone();
                                }
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(format!("    {}: {}", line, text)).color(theme::FG_DIM).size(10.0).monospace()
                                ).frame(false).min_size(egui::vec2(ui.available_width(), 16.0))).clicked() {
                                    if let Some(idx) = self.files.iter().position(|x| x == f) {
                                        self.open_file(idx);
                                        self.jump_to_line(*line);
                                    }
                                }
                            }
                        });
                        let _ = proj_q;
                    }
                    3 => { // Outline（符号大纲，点击跳行）
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(
                                if lang == 1 { "大纲" } else { "OUTLINE" }
                            ).color(theme::FG_DIM).size(11.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🔄").size(12.0))
                                    .frame(false)).clicked() { self.refresh_outline(); }
                            });
                        });
                        ui.separator();
                        if self.outline.is_empty() { self.refresh_outline(); }
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let entries = self.outline.clone();
                            for (name, kind, line, _col) in entries {
                                let icon = if kind.contains("Fn") { "ƒ" } else if kind.contains("Struct") { "S" } else if kind.contains("Enum") { "E" } else { "·" };
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(format!("{} {}  {}", icon, name, kind))
                                        .color(theme::FG).size(11.0)
                                ).frame(false).min_size(egui::vec2(ui.available_width(), 18.0))).clicked() {
                                    self.jump_to_line(line + 1);
                                }
                            }
                        });
                    }
                    2 => { // Git
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("  SOURCE CONTROL").color(theme::FG_DIM).size(11.0).strong());
                        ui.add_space(4.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            if ui.small_button("Refresh").clicked() { git_now = true; }
                            if ui.small_button("Diff").clicked() {
                                git_now = true;
                                self.bottom_tab = 1;
                                self.show_problems = true;
                            }
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            if ui.small_button("+ Stage All").clicked() { self.git_stage_all(); git_now = true; }
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            ui.label("msg:");
                            ui.add(egui::TextEdit::singleline(&mut commit_msg).desired_width(ui.available_width() - 70.0)
                                .hint_text("commit message..."));
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            let can = !commit_msg.trim().is_empty();
                            if ui.add_enabled(can, egui::Button::new(egui::RichText::new("Commit").color(theme::FG_BRIGHT))).clicked() {
                                let m = commit_msg.trim().to_string();
                                self.git_commit(&m);
                                commit_msg.clear();
                                git_now = true;
                            }
                        });
                        ui.separator();
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(egui::RichText::new(&self.output_text).color(theme::FG).size(10.0).monospace());
                        });
                    }
                    _ => {}
                }
            });
        }
        if search_now { self.project_search(); }
        if git_now { self.git_refresh(); }

        // ── 右侧 AI 面板（Ctrl+I 切换）──
        if show_ai_panel && !focus {
        egui::SidePanel::right("ai_panel")
            .default_width(260.0)
            .resizable(true)
            .frame(egui::Frame::none().fill(theme::BG_SIDE))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(
                    format!("  {}", tr("ai", lang))
                ).color(theme::FG_DIM).size(11.0).strong());
                ui.add_space(4.0);
                ui.separator();

                // AI 设置入口
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if ui.small_button("Settings").clicked() {
                        show_ai_settings = !show_ai_settings;
                    }
                    if !self.ai_settings.api_key.is_empty() {
                        ui.label(egui::RichText::new(
                            format!("{} / {}", self.ai_settings.provider, self.ai_settings.model)
                        ).color(theme::FG_DIM).size(10.0));
                    }
                });
                ui.separator();

                // 聊天记录（占满剩余高度，输入框沉底 — 对标主流 AI 助手布局）
                let msg_area = ui.available_height() - 64.0;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_height(msg_area.max(60.0));
                        ui.add_space(8.0);
                        if self.ai_chat.is_empty() {
                            egui::Frame::none()
                                .fill(theme::SEL_BG)
                                .rounding(egui::Rounding::same(6.0))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(
                                        if lang == 1 { "你好！我是 Aine AI 助手。" }
                                        else { "Hi! I'm the Aine AI assistant." }
                                    ).color(theme::FG).size(12.0));
                                });
                        }
                        for (is_user, msg) in &self.ai_chat {
                            ui.add_space(4.0);
                            let bg = if *is_user { theme::SEL_BG } else { theme::BG_PANEL };
                            egui::Frame::none()
                                .fill(bg)
                                .rounding(egui::Rounding::same(6.0))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    if *is_user {
                                        ui.label(egui::RichText::new(msg).color(theme::FG).size(12.0));
                                    } else {
                                        render_ai_markdown(ui, msg);
                                    }
                                });
                        }
                        // 思维链（灰色斜体）与 token 用量
                        if !self.ai_reasoning.is_empty() {
                            ui.label(egui::RichText::new("— reasoning —").color(theme::FG_DIM).size(10.0));
                            ui.label(egui::RichText::new(&self.ai_reasoning).color(theme::FG_DIM).size(11.0).italics());
                        }
                        if let Some((i, o)) = self.ai_usage {
                            ui.label(egui::RichText::new(format!("tokens {} / {}", i, o)).color(theme::FG_DIM).size(10.0));
                        }
                        // 流式输出时自动跟随到底部
                        if self.ai_streaming {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });

                // 输入框（固定在面板底部）
                ui.separator();
                ui.horizontal(|ui| {
                    // 流式进行中 → 停止按钮
                    if self.ai_streaming {
                        if ui.small_button("■").on_hover_text("Stop").clicked() {
                            self.ai_cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_input)
                            .hint_text(tr("ask", lang))
                            .desired_width(ui.available_width() - if self.ai_streaming { 60.0 } else { 36.0 })
                    );
                    if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.ai_input.is_empty() && !self.ai_streaming {
                            self.send_ai_message();
                        }
                    }
                });
            });
        }

        // ── 最小imap（代码缩略图，点击跳行）──
        if show_problems && !focus && !self.tabs.is_empty() {
            egui::SidePanel::right("minimap")
                .exact_width(70.0)
                .frame(egui::Frame::none().fill(theme::BG_EDIT))
                .show(ctx, |ui| {
                    let content = self.active_tab().map(|t| t.content.clone()).unwrap_or_default();
                    let cur_line = self.active_tab().map(|t| t.cursor_line).unwrap_or(0);
                    let line_h = 2.0f32;
                    let total_h = content.lines().count() as f32 * line_h;
                    let resp = ui.allocate_response(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Sense::click(),
                    );
                    let painter = ui.painter_at(resp.rect);
                    painter.rect_filled(resp.rect, 0.0, theme::BG_EDIT);
                    let w = resp.rect.width() - 8.0;
                    for (i, line) in content.lines().enumerate().take(3000) {
                        let t = line.trim_start();
                        if t.is_empty() { continue; }
                        let len = (t.len().min(120) as f32 / 120.0) * w;
                        let color = if t.starts_with("//") {
                            egui::Color32::from_rgb(0x3f, 0x5f, 0x3f)
                        } else if t.starts_with('"') {
                            egui::Color32::from_rgb(0x6f, 0x5f, 0x4f)
                        } else {
                            egui::Color32::from_rgb(0x5a, 0x5a, 0x5a)
                        };
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(resp.rect.left() + 4.0, resp.rect.top() + i as f32 * line_h),
                                egui::vec2(len, 1.2),
                            ), 0.0, color);
                    }
                    // 光标行指示条
                    let cur_y = resp.rect.top() + cur_line as f32 * line_h;
                    if cur_y < resp.rect.bottom() {
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(resp.rect.left(), cur_y),
                                egui::vec2(resp.rect.width(), line_h),
                            ), 0.0, egui::Color32::from_rgb(0x3a, 0x5a, 0x7a));
                    }
                    // 点击 → 跳到对应行
                    if resp.clicked() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let line = ((pos.y - resp.rect.top()) / line_h) as usize;
                            let char_idx = content.split('\n').take(line).map(|l| l.chars().count() + 1).sum();
                            self.pending_cursor_chars = Some(char_idx);
                        }
                    }
                    let _ = total_h;
                });
        }

        // ── Quick Open (Ctrl+P) ──
        if show_quick_open {
            egui::Window::new("Quick Open")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut quick_filter);
                    ui.separator();
                    let filter = quick_filter.to_lowercase();
                    let files_clone = self.files.clone();
                    for (i, f) in files_clone.iter().enumerate() {
                        if filter.is_empty() || f.to_lowercase().contains(&filter) {
                            if ui.button(f).clicked() {
                                self.open_file(i);
                                show_quick_open = false;
                                quick_filter.clear();
                            }
                        }
                    }
                });
        }

        // ── Command Bar (Ctrl+K) ──
        if show_command_bar {
            egui::Window::new("Commands")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut command_filter);
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        let filter = command_filter.to_lowercase();
                        // 模糊匹配打分：子序列命中为基础，连续/词首命中加分
                        fn fuzzy_score(name: &str, pat: &str) -> Option<i32> {
                            if pat.is_empty() { return Some(0); }
                            let name_l = name.to_lowercase();
                            let pat_l = pat.to_lowercase();
                            let mut score = 0;
                            let mut ni = 0usize;
                            let mut prev_hit = false;
                            for pc in pat_l.chars() {
                                let mut found = false;
                                while ni < name_l.len() {
                                    let nc = name_l[ni..].chars().next().unwrap();
                                    if nc == pc {
                                        score += if prev_hit { 3 } else { 1 };
                                        // 词首命中加分
                                        if ni == 0 || !name_l[ni-1..ni].chars().next().unwrap().is_alphanumeric() {
                                            score += 2;
                                        }
                                        prev_hit = true;
                                        ni += 1;
                                        found = true;
                                        break;
                                    }
                                    prev_hit = false;
                                    ni += 1;
                                }
                                if !found { return None; }
                            }
                            Some(score - (name_l.len() as i32 - pat_l.len() as i32) / 8)
                        }
                        let mut scored: Vec<(i32, &Cmd)> = Vec::new();
                        for c in Cmd::all() {
                            let name = c.label(lang);
                            match if filter.is_empty() { Some(0) } else { fuzzy_score(name, &filter) } {
                                Some(sc) => scored.push((sc, c)),
                                None => {}
                            }
                        }
                        scored.sort_by(|a, b| b.0.cmp(&a.0));
                        for (_sc, c) in scored.iter().take(30) {
                            let name = c.label(lang);
                            ui.horizontal(|ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new(name).size(12.0))
                                        .frame(false)
                                        .min_size(egui::vec2(250.0, 18.0))
                                ).clicked() {
                                    command_action = Some(**c);
                                    show_command_bar = false;
                                    command_filter.clear();
                                }
                                ui.label(egui::RichText::new(c.shortcut()).color(theme::FG_DIM).size(11.0));
                            });
                        }
                    });
                });
        }

        // ── 确认对话框（删除文件 / 丢弃未保存修改）──
        let confirm_ctx = self.confirm.as_ref().map(|c| match c {
            Confirm::DeleteFile(f) => (
                if lang == 1 { "删除文件" } else { "Delete File" },
                format!("{} {}", if lang == 1 { "确定从磁盘永久删除" } else { "Permanently delete" }, f),
                true,
            ),
            Confirm::CloseTab(_) => (
                if lang == 1 { "未保存的修改" } else { "Unsaved Changes" },
                if lang == 1 { "此文件有未保存的修改，关闭将丢弃。确定关闭？".to_string() } else { "This file has unsaved changes. Close and discard them?".to_string() },
                false,
            ),
        });
        if let Some((title, msg, danger)) = confirm_ctx {
            egui::Window::new(title)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&msg).color(theme::FG).size(13.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let ok_text = tr("confirm_ok", lang);
                        let cancel_text = tr("cancel", lang);
                        let ok_btn = egui::Button::new(egui::RichText::new(ok_text).color(theme::FG_BRIGHT));
                        let ok_btn = if danger { ok_btn.fill(egui::Color32::from_rgb(0x5a, 0x1d, 0x1d)) } else { ok_btn };
                        if ui.add(ok_btn).clicked() {
                            confirm_yes = true;
                        }
                        if ui.button(cancel_text).clicked() {
                            cancel_confirm = true;
                        }
                    });
                });
        }
        if cancel_confirm { self.confirm = None; }

        // ── 转到行 (Ctrl+G) ──
        if show_goto {
            egui::Window::new(tr("goto_title", lang))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
                .default_width(240.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("line_label", lang));
                        let resp = ui.add(egui::TextEdit::singleline(&mut goto_input)
                            .desired_width(100.0).code_editor());
                        let go = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if go || ui.button(tr("go", lang)).clicked() {
                            if let Ok(line) = goto_input.trim().parse::<usize>() {
                                if line >= 1 {
                                    self.jump_to_line(line);
                                    show_goto = false;
                                    goto_input.clear();
                                }
                            }
                        }
                    });
                });
        }

        // ── 补全弹窗（LSP completion）──
        if show_completion && !completions.is_empty() {
            egui::Window::new("Completion")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Ctrl+Space — LSP completions").color(theme::FG_DIM).size(10.0));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for c in &completions {
                            if ui.add(egui::Button::new(egui::RichText::new(c).size(12.0).monospace())
                                .frame(false).min_size(egui::vec2(ui.available_width(), 18.0))).clicked() {
                                // 在光标处插入
                                if let Some(t) = self.active_tab_mut() {
                                    let b = t.cursor_byte.unwrap_or(0).min(t.content.len());
                                    t.content.insert_str(b, c);
                                    t.dirty = true;
                                    dropped_check = true;
                                }
                                show_completion = false;
                            }
                        }
                    });
                    if ui.button("Esc close").clicked() { show_completion = false; }
                });
        }

        // ── 重命名符号 (F2，ide_rename 后端) ──
        if show_rename {
            egui::Window::new(if lang == 1 { "重命名符号" } else { "Rename Symbol" })
                .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(if lang == 1 { "新名称:" } else { "New name:" });
                        let resp = ui.add(egui::TextEdit::singleline(&mut rename_input)
                            .desired_width(140.0));
                        let go = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if go || ui.button(tr("confirm_ok", lang)).clicked() {
                            let new_name = rename_input.trim().to_string();
                            if !new_name.is_empty() {
                                let (content, byte) = {
                                    let t = self.active_tab().unwrap();
                                    (t.content.clone(), t.cursor_byte.unwrap_or(0))
                                };
                                let path = self.active_path().unwrap_or_default();
                                let ps = path.to_string_lossy().to_string();
                                if let Some(new_text) = aine::lsp::ide_rename(&content, &ps, byte, &new_name) {
                                    if let Some(t) = self.active_tab_mut() {
                                        t.content = new_text;
                                        t.dirty = true;
                                    }
                                    self.check();
                                }
                            }
                            show_rename = false;
                            rename_input.clear();
                        }
                    });
                });
        }

        // ── Diff 审批 UI（T37：AI 提出变更 → 用户 Accept/Reject）──
        if let Some(pd) = &self.pending_diff {
            egui::Window::new(format!("AI 修复审批 — {}", pd.file))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(720.0)
                .default_height(480.0)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(format!("原因: {}", pd.reason)).color(theme::YELLOW).size(12.0));
                    ui.separator();
                    ui.label(egui::RichText::new("变更预览（→ 建议的新内容）:").color(theme::FG_DIM).size(11.0));
                    egui::ScrollArea::vertical().max_height(ui.available_height() - 60.0).show(ui, |ui| {
                        ui.columns(2, |cols| {
                            cols[0].label(egui::RichText::new("当前").color(theme::RED).size(11.0));
                            cols[1].label(egui::RichText::new("AI 建议").color(egui::Color32::from_rgb(0x6a, 0x99, 0x55)).size(11.0));
                            cols[0].add(egui::TextEdit::multiline(&mut old_preview.clone())
                                .font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                            cols[1].add(egui::TextEdit::multiline(&mut new_preview.clone())
                                .font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                        });
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(
                            egui::RichText::new("✓ Accept（应用并重新编译）").color(egui::Color32::WHITE)
                        ).fill(egui::Color32::from_rgb(0x2e, 0x7d, 0x32))).clicked() {
                            apply_diff = true;
                        }
                        if ui.button("✗ Reject（放弃）").clicked() {
                            reject_diff = true;
                        }
                    });
                });
        }
        if apply_diff { self.apply_pending_diff(); }
        if reject_diff {
            self.pending_diff = None;
            self.ai_chat.push((false, "已拒绝此修复建议。".into()));
        }

        // ── 命令执行（UI 闭包外）──
        if let Some(cmd) = command_action {
            cmd.execute(self);
        }

        // ── 中央面板 ──
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_EDIT))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Tab 栏（多标签：活动 Tab = 编辑器底色 + 顶部 accent 条，对标 VS Code）
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        let tab_count = self.tabs.len();
                        for i in 0..tab_count {
                            let (name, dirty) = if let Some(t) = self.tabs.get(i) {
                                (t.name.clone(), t.dirty)
                            } else { continue; };
                            let is_active = i == self.active_tab;
                            let label = if dirty { format!(" {} ●", name) } else { format!("  {}", name) };
                            let bg = if is_active { theme::TAB_ACTIVE } else { theme::TAB_INACTIVE };
                            let fg = if is_active { theme::FG_BRIGHT } else { theme::FG_DIM };
                            let tab_resp = egui::Frame::none()
                                .fill(bg)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_space(8.0);
                                        if ui.add(egui::Button::new(
                                            egui::RichText::new(label).color(fg).size(12.0)
                                        ).frame(false)).clicked() {
                                            self.active_tab = i;
                                        }
                                        if ui.add(egui::Button::new(
                                            egui::RichText::new("✕").color(theme::FG_DIM).size(10.0)
                                        ).frame(false).small()).clicked() {
                                            self.request_close_tab(i);
                                        }
                                        ui.add_space(4.0);
                                    });
                                });
                            if is_active {
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        tab_resp.response.rect.left_top(),
                                        egui::vec2(tab_resp.response.rect.width(), 2.0),
                                    ), 2.0, theme::ACCENT);
                            }
                        }
                    });
                    ui.separator();

                    // Veil 双视图预览：显示层 ↔ canonical 对照
                    if self.show_veil_preview {
                        if let Some(t) = self.tabs.get(self.active_tab) {
                            let cname = aine::veil::SURFACE_NAMES[1];
                            let sname = aine::veil::SURFACE_NAMES[t.surface];
                            let preview = if t.surface == 1 {
                                aine::veil::render_to(&t.content, 0)
                            } else {
                                t.content.clone()
                            };
                            let (l1, l2) = if t.surface == 1 { (sname, cname) } else { (cname, sname) };
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(format!(
                                    "Veil 预览  上：编辑器({})   下：{}(canonical)",
                                    l1, l2
                                )).color(theme::FG_DIM).size(10.0));
                            });
                            let mut preview_ro = preview;
                            ui.push_id("veil_preview", |ui| {
                                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut preview_ro)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(ui.available_width())
                                            .interactive(false),
                                    );
                                });
                            });
                        }
                    }

                    // 搜索替换栏（计数/下一个/全部替换/高亮）
                    if show_search {
                        let mut st = self.search_text.clone();
                        let mut rt = self.replace_text.clone();
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(tr("search", lang));
                            let s_resp = ui.add(egui::TextEdit::singleline(&mut st)
                                .hint_text(if lang == 1 { "回车查找下一个" } else { "Enter to find next" })
                                .desired_width(160.0));
                            if s_resp.changed() { st = st.clone(); }
                            let hits = if st.trim().is_empty() { 0 } else {
                                self.active_tab().map(|t| t.content.matches(&st).count()).unwrap_or(0)
                            };
                            ui.label(egui::RichText::new(format!("{} {}", hits, if lang == 1 { "个匹配" } else { "matches" }))
                                .color(theme::FG_DIM).size(11.0));
                            ui.separator();
                            ui.label(tr("replace", lang));
                            ui.add(egui::TextEdit::singleline(&mut rt).desired_width(140.0));
                            if ui.button(tr("find_next", lang)).clicked() {
                                self.search_text = st.clone();
                                find_next_now = true;
                            }
                            if ui.button(tr("replace_all", lang)).clicked() {
                                self.do_replace_all(&st, &rt);
                                rt = rt.clone();
                            }
                            if ui.button("✕").clicked() { show_search = false; }
                        });
                        self.search_text = st;
                        self.replace_text = rt;
                        ui.separator();
                    }

                    // 面包屑
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("examples").color(theme::FG_DIM).size(11.0));
                        ui.label(egui::RichText::new(" > ").color(theme::FG_DIM).size(11.0));
                        if let Some(tab) = self.active_tab() {
                            ui.label(egui::RichText::new(&tab.name).color(theme::FG_DIM).size(11.0));
                        }
                    });
                    ui.separator();

                    // 编辑器 + 行号
                    let editor_h = ui.available_height() - 30.0;
                    if self.active_tab < self.tabs.len() {
                        let line_count = self.tabs[self.active_tab].content.lines().count().max(1);
                        let cur_line = self.tabs[self.active_tab].cursor_line;

                        egui::ScrollArea::vertical()
                            .max_height(editor_h)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // 行号列
                                    ui.vertical(|ui| {
                                        for i in 0..line_count {
                                            let c = if i == cur_line { theme::ACCENT } else { theme::LINE_NUM };
                                            let is_bp = self.breakpoints.contains(&(i + 1));
                                            let btxt = if is_bp { format!("{:>3} ●", i) } else { format!("{:>4}", i + 1) };
                                            let bcol = if is_bp { theme::RED } else { c };
                                            let r = ui.add(egui::Button::new(
                                                egui::RichText::new(btxt)
                                                    .color(bcol).size(13.0).monospace()
                                            ).frame(false).min_size(egui::vec2(46.0, 16.0)));
                                            if r.clicked() {
                                                self.jump_to_line(i + 1);
                                            }
                                            if r.secondary_clicked() {
                                                if is_bp { self.breakpoints.retain(|b| *b != i + 1); }
                                                else { self.breakpoints.push(i + 1); self.breakpoints.sort(); }
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);

                                    // 编辑器（语法高亮 + 括号匹配）
                                    let (content, cur, cur_byte, cur_surface) = if let Some(t) = self.tabs.get(self.active_tab) {
                                        (t.content.clone(), t.cursor_line, t.cursor_byte, t.surface)
                                    } else { (String::new(), 0, None, 1usize) };
                                    // Language Veil：显示层 = render_to(canonical)
                                    let veiled = aine::veil::render_to(&content, cur_surface);
                                    let mut display = veiled;
                                    // 搜索高亮词（Ctrl+F 打开且非空时）
                                    let search_hi = if show_search && !search_text.trim().is_empty() {
                                        Some(search_text.trim().to_string())
                                    } else { None };
                                    let mut layouter = move |ui: &egui::Ui, text: &str, _w: f32|
                                        -> std::sync::Arc<egui::Galley> {
                                        let job = highlight_layout(ui, text, cur_byte, search_hi.as_deref(), cur_surface);
                                        ui.fonts(|f| f.layout_job(job))
                                    };
                                    let editor_id = egui::Id::new("main_editor");
                                    // 应用 pending 选区（搜索命中高亮）/ 光标（跳转）
                                    if let Some((s_char, e_char)) = self.pending_selection.take() {
                                        if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), editor_id) {
                                            let cs = egui::text::CCursor { index: s_char, prefer_next_row: false };
                                            let ce = egui::text::CCursor { index: e_char, prefer_next_row: true };
                                            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(cs, ce)));
                                            state.store(ui.ctx(), editor_id);
                                        }
                                    } else if let Some(char_idx) = self.pending_cursor_chars.take() {
                                        if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), editor_id) {
                                            let cc = egui::text::CCursor { index: char_idx, prefer_next_row: true };
                                            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(cc.clone(), cc)));
                                            state.store(ui.ctx(), editor_id);
                                        }
                                    }
                                    let output = egui::TextEdit::multiline(&mut display)
                                        .id(editor_id)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(ui.available_width())
                                        .code_editor()
                                        .lock_focus(true)
                                        .layouter(&mut layouter)
                                        .show(ui);
                                    let resp = output.response.clone();
                                    // ── 真实光标追踪（下一帧用于行号/状态栏/括号匹配）──
                                    // egui CCursor.index 是字符索引，转字节偏移
                                    if let Some(cr) = output.cursor_range {
                                        let char_idx = cr.primary.ccursor.index;
                                        let byte = display.char_indices().nth(char_idx)
                                            .map(|(b, _)| b).unwrap_or(display.len());
                                        let mut line = 0usize;
                                        let mut col = 0usize;
                                        let mut acc = 0usize;
                                        for l in display.split('\n') {
                                            if byte <= acc + l.len() { col = byte - acc; break; }
                                            acc += l.len() + 1;
                                            line += 1;
                                        }
                                        if let Some(t) = self.tabs.get_mut(self.active_tab) {
                                            t.cursor_line = line;
                                            t.cursor_col = col;
                                            t.cursor_byte = Some(byte);
                                        }
                                    }
                                    // ── Hover & Go-to-Definition ──
                                    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                                    if let Some(ppos) = pointer_pos {
                                        if resp.rect.contains(ppos) {
                                            // monospace font metrics
                                            let line_h = 16.0f32;
                                            let char_w = 7.8f32; // ~13px monospace
                                            let rel_y = ppos.y - resp.rect.top();
                                            let rel_x = ppos.x - resp.rect.left();
                                            let line = (rel_y / line_h).floor() as usize;
                                            let col = (rel_x / char_w).floor() as usize;
                                            // Compute byte offset
                                            let mut byte_off = 0usize;
                                            for (li, l) in display.split('\n').enumerate() {
                                                if li == line {
                                                    byte_off += col.min(l.len());
                                                    break;
                                                }
                                                byte_off += l.len() + 1;
                                            }
                                            let file_path = self.root.join("examples").join(
                                                self.active_tab().map(|t| t.name.clone()).unwrap_or_default()
                                            );
                                            let path_str = file_path.to_string_lossy().to_string();
                                            // Ctrl+Click → Go to Definition
                                            if ui.input(|i| i.pointer.primary_clicked() && i.modifiers.ctrl) {
                                                if let Some((def_line, _def_col)) = aine::lsp::ide_definition(&display, &path_str, byte_off) {
                                                    // Move cursor to definition line
                                                    if let Some(t) = self.tabs.get_mut(self.active_tab) {
                                                        t.cursor_line = def_line;
                                                    }
                                                }
                                            }
                                            // Hover tooltip
                                            if let Some(info) = aine::lsp::ide_hover(&display, &path_str, byte_off) {
                                                self.hover_info = Some(info);
                                                self.hover_pos = Some(ppos);
                                            } else {
                                                self.hover_info = None;
                                            }
                                        } else {
                                            self.hover_info = None;
                                        }
                                    } else {
                                        self.hover_info = None;
                                    }
                                    // Draw hover tooltip
                                    if let (Some(ref info), Some(ref pos)) = (&self.hover_info, &self.hover_pos) {
                                        let tip_id = egui::Id::new("hover_tip");
                                        egui::Area::new(tip_id)
                                            .fixed_pos(*pos + egui::vec2(16.0, 16.0))
                                            .show(ctx, |ui| {
                                                egui::Frame::none()
                                                    .fill(egui::Color32::from_rgb(0x2d, 0x2d, 0x2d))
                                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x55, 0x55, 0x55)))
                                                    .rounding(4.0)
                                                    .inner_margin(6.0)
                                                    .show(ui, |ui| {
                                                        ui.label(egui::RichText::new(info.as_str())
                                                            .color(theme::FG).size(12.0).monospace());
                                                    });
                                            });
                                    }
                                    // ── Auto-indent on Enter ──
                                    if resp.changed() {
                                        if let Some(t) = self.tabs.get_mut(self.active_tab) {
                                            // Detect if a newline was just added
                                            let old_lines: Vec<&str> = t.content.split('\n').collect();
                                            let new_lines: Vec<&str> = display.split('\n').collect();
                                            if new_lines.len() > old_lines.len() {
                                                // Find the new line
                                                for (i, nl) in new_lines.iter().enumerate() {
                                                    if i >= old_lines.len() || *nl != old_lines[i] {
                                                        // This is the new line - compute indent
                                                        if i > 0 {
                                                            let prev = new_lines[i - 1];
                                                            let trimmed = prev.trim_end();
                                                            let base_indent: String = prev.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
                                                            let extra = if trimmed.ends_with('{') || trimmed.ends_with(':') || trimmed.ends_with("=>") {
                                                                "  ".to_string()
                                                            } else { String::new() };
                                                            let indent = format!("{}{}", base_indent, extra);
                                                            if !indent.is_empty() && nl.trim().is_empty() {
                                                                // Insert indent into the new line
                                                                let mut rebuilt = new_lines[..i].join("\n");
                                                                rebuilt.push('\n');
                                                                rebuilt.push_str(&indent);
                                                                if i + 1 < new_lines.len() {
                                                                    rebuilt.push('\n');
                                                                    rebuilt.push_str(&new_lines[i + 1..].join("\n").as_str());
                                                                }
                                                                // Skip lines that are just whitespace (the typed newline)
                                                                for rest_line in &new_lines[i + 1..] {
                                                                    if rest_line.trim().is_empty() {
                                                                        rebuilt = rebuilt.trim_end_matches('\n').to_string();
                                                                    }
                                                                }
                                                                display = rebuilt;
                                                            }
                                                        }
                                                        break;
                                                    }
                                                }
                                            }
                                            // 编辑发生在表面层 → 回写 canonical
                                            t.content = aine::veil::surface_to_canonical(&display, cur_surface);
                                            t.dirty = true;
                                        }
                                    }
                                });
                            });
                    }

                    // 底部面板（PROBLEMS / OUTPUT / TERMINAL / TESTS）
                    if show_problems && !focus {
                        ui.separator();
                        // Tab 切换
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            for (i, name) in [tr("problems", lang), tr("output", lang), tr("terminal", lang), tr("tests", lang)].iter().enumerate() {
                                let is_sel = bottom_tab == i;
                                let c = if is_sel { theme::FG } else { theme::FG_DIM };
                                if ui.add(egui::Button::new(
                                    egui::RichText::new(*name).color(c).size(10.0).strong()
                                ).frame(false)).clicked() {
                                    bottom_tab = i;
                                    if i == 3 { test_now = true; }
                                }
                                ui.add_space(12.0);
                            }
                        });
                        ui.separator();

                        // 内容区
                        match bottom_tab {
                            0 => { // PROBLEMS
                                let n_diags = self.diags.len();
                                if n_diags == 0 {
                                    ui.label(egui::RichText::new(format!("  {}", tr("no_problems", lang))).color(theme::FG_DIM).size(11.0));
                                } else {
                                    ui.label(egui::RichText::new(
                                        format!("  {} errors, {} warnings", n_err, n_warn)
                                    ).color(if n_err > 0 { theme::RED } else { theme::YELLOW }).size(11.0));
                                    let diags = self.diags.clone();
                                    for (i, d) in diags.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.add_space(8.0);
                                            let icon = if d.severity == "error" { "⛔" } else { "⚠️" };
                                            let color = if d.severity == "error" { theme::RED } else { theme::YELLOW };
                                            ui.label(egui::RichText::new(icon).size(11.0));
                                            // 行可点击 → 跳转到出错行
                                            if ui.add(egui::Button::new(
                                                egui::RichText::new(
                                                    format!("[{}] {} (Ln {}, Col {})", d.code, d.message, d.line, d.col)
                                                ).color(color).size(11.0)
                                            ).frame(false)).clicked() {
                                                self.jump_to_line(d.line);
                                            }
                                            if ui.small_button("Explain").clicked() {
                                                explain_idx = Some(i);
                                            }
                                            if ui.small_button("Fix").clicked() {
                                                fix_idx = Some(i);
                                            }
                                        });
                                        if !d.quick_fix.is_empty() {
                                            ui.label(egui::RichText::new(format!("      💡 {}", d.quick_fix))
                                                .color(theme::FG_DIM).size(10.0));
                                        }
                                    }
                                }
                            }
                            1 => { // OUTPUT
                                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new(
                                        format!("{}", self.output_text)
                                    ).color(theme::FG).size(11.0).monospace());
                                });
                            }
                            2 => { // TERMINAL
                                egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                    // Terminal output
                                    for line in self.terminal_text.lines() {
                                        ui.label(egui::RichText::new(line).color(theme::TERMINAL_FG).size(11.0).monospace());
                                    }
                                });
                                // Terminal input
                                let cwd_display = self.terminal_cwd.file_name().unwrap_or_default().to_string_lossy().to_string();
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(format!("{}$", cwd_display)).color(theme::ACCENT).size(11.0).monospace());
                                    let resp = ui.text_edit_singleline(&mut self.terminal_input);
                                    if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        let cmd = self.terminal_input.trim().to_string();
                                        if !cmd.is_empty() {
                                            self.terminal_text.push_str(&format!("{}$ {}\n", cwd_display, cmd));
                                            // Handle cd command
                                            if cmd.starts_with("cd ") {
                                                let target = cmd[3..].trim();
                                                let new_dir = if target == ".." {
                                                    self.terminal_cwd.parent().map(|p| p.to_path_buf())
                                                } else if target.starts_with('/') || target.contains(':') {
                                                    Some(std::path::PathBuf::from(target))
                                                } else {
                                                    Some(self.terminal_cwd.join(target))
                                                };
                                                if let Some(d) = new_dir {
                                                    if d.is_dir() {
                                                        self.terminal_cwd = d;
                                                    } else {
                                                        self.terminal_text.push_str(&format!("cd: no such directory: {}\n", target));
                                                    }
                                                }
                                            } else {
                                                // 后台执行命令（UI 不冻结）
                                                let cwd = self.terminal_cwd.clone();
                                                self.spawn_task("终端", move |tx| {
                                                    let out = Command::new("cmd")
                                                        .args(["/C", &cmd])
                                                        .current_dir(&cwd)
                                                        .output();
                                                    let mut text = String::new();
                                                    match out {
                                                        Ok(o) => {
                                                            text.push_str(&String::from_utf8_lossy(&o.stdout));
                                                            text.push_str(&String::from_utf8_lossy(&o.stderr));
                                                            if !o.status.success() {
                                                                text.push_str(&format!("exit code: {}\n", o.status.code().unwrap_or(-1)));
                                                            }
                                                        }
                                                        Err(e) => text.push_str(&format!("error: {}\n", e)),
                                                    }
                                                    let _ = tx.send(TaskMsg::Term { text });
                                                });
                                            }
                                            self.terminal_input.clear();
                                        }
                                    }
                                });
                            }
                            3 => { // TESTS（aine test 聚合，T43）
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    if ui.small_button("▶ Run Tests").clicked() { test_now = true; }
                                    ui.label(egui::RichText::new("runs #[test] functions in current file").color(theme::FG_DIM).size(10.0));
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                    let pass = self.test_output.contains("passed") || self.test_output.contains("ok");
                                    let fail = self.test_output.contains("FAILED") || self.test_output.contains("failed");
                                    let color = if fail { theme::RED } else if pass { egui::Color32::from_rgb(0x6a, 0x99, 0x55) } else { theme::FG };
                                    let _ = color;
                                    for line in self.test_output.lines() {
                                        let lc = if line.contains("FAILED") || line.contains("failed") { theme::RED }
                                            else if line.contains("ok") || line.contains("passed") { egui::Color32::from_rgb(0x6a, 0x99, 0x55) }
                                            else { theme::TERMINAL_FG };
                                        ui.label(egui::RichText::new(line).color(lc).size(11.0).monospace());
                                    }
                                });
                            }
                            _ => {}
                        }
                    }
                });
            });

        // ── CentralPanel 内 UI 触发的动作（帧末统一执行，避免同帧丢失）──
        if let Some(i) = explain_idx.take() { self.ai_explain_diag(i); }
        if let Some(i) = fix_idx.take() { self.ai_fix_diag(i); }
        if test_now { self.run_tests(); }
        if dropped_check { self.check(); }
        if find_next_now { self.find_next(); }
        if confirm_yes {
            if let Some(c) = self.confirm.take() {
                match c {
                    Confirm::DeleteFile(f) => self.delete_file(&f),
                    Confirm::CloseTab(i) => self.close_tab(i),
                }
            }
        }

        // ── 局部 UI 状态回写 ──
        self.show_problems = show_problems;
        self.bottom_tab = bottom_tab;
        self.show_quick_open = show_quick_open;
        self.quick_open_filter = quick_filter;
        self.show_command_bar = show_command_bar;
        self.command_filter = command_filter;
        self.show_goto = show_goto;
        self.goto_input = goto_input;
        self.show_rename = show_rename;
        self.rename_input = rename_input;
        self.show_completion = show_completion;
        self.completions = completions;
        self.git_commit_msg = commit_msg;
        self.show_ai_settings = show_ai_settings;
        self.show_search = show_search;
        self.show_model_picker = show_model_picker;
        self.model_filter = model_filter;
        if save_models_flag { self.save_models_toml(); }
        self.render_toasts(ctx);
    }
}

// ── AI API Functions ──

/// 解析 aine check 输出为结构化诊断列表
fn parse_diagnostics(text: &str) -> Vec<Diag> {
    let mut out: Vec<Diag> = Vec::new();
    for line in text.lines() {
        // error[F1001]: message 或 warning[W2001]: message
        if let Some(rest) = line.strip_prefix("error[").or_else(|| line.strip_prefix("warning[")) {
            let severity = if line.starts_with("error[") { "error" } else { "warning" };
            if let Some(close) = rest.find(']') {
                let code = rest[..close].to_string();
                let after = &rest[close + 1..];
                let message = after.strip_prefix(": ").unwrap_or(after).trim().to_string();
                out.push(Diag { severity: severity.into(), code, message, line: 0, col: 0, quick_fix: String::new() });
                continue;
            }
        }
        // --> file:line:col 归属到上一个诊断
        if let Some(rest) = line.trim().strip_prefix("-->") {
            let parts: Vec<&str> = rest.trim().rsplitn(3, ':').collect();
            if parts.len() == 3 {
                if let (Ok(l), Ok(c)) = (parts[1].parse::<usize>(), parts[0].parse::<usize>()) {
                    if let Some(last) = out.last_mut() {
                        last.line = l;
                        last.col = c;
                    }
                }
            }
            continue;
        }
        // Quick Fix 提示
        if let Some(rest) = line.trim().strip_prefix("= Quick Fix: ") {
            if let Some(last) = out.last_mut() {
                last.quick_fix = rest.trim().to_string();
            }
        }
    }
    // 过滤没有位置的（解析中断行）
    out.retain(|d| d.line > 0);
    out
}

/// 从 AI 回复中提取 ```aine ... ```（或 ```）代码块
fn extract_code_block(text: &str) -> Option<String> {
    let fence = "```";
    let mut rest = text;
    while let Some(start) = rest.find(fence) {
        let after_open = &rest[start + fence.len()..];
        // 跳过语言标记行
        let body_start = after_open.find('\n').map(|i| i + 1).unwrap_or(0);
        let body = &after_open[body_start..];
        if let Some(end) = body.find(fence) {
            let candidate = body[..end].trim_start_matches('\n').trim_end().to_string();
            if !candidate.is_empty() {
                return Some(candidate);
            }
        }
        rest = &after_open[body_start..];
    }
    None
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn extract_json_str<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{}\":\"", key);
    if let Some(start) = json.find(&pat) {
        let val_start = start + pat.len();
        if let Some(end) = json[val_start..].find('"') {
            return &json[val_start..val_start + end];
        }
    }
    ""
}

// __UMBER_SECTION__

// ── Umber 运行时接入（动态加载 umber_ffi.dll，统一多 Provider 真流式）──
mod umber {
    use std::ffi::{c_char, c_void};
    use std::sync::OnceLock;

    pub const OK: i32 = 0;
    pub const EVENT: i32 = 1;
    pub const CLOSED: i32 = 2;
    pub const WOULD_BLOCK: i32 = 3;

    #[repr(C)]
    pub struct UmerEvent {
        pub sequence: u64,
        pub json: *mut c_char,
        pub json_len: usize,
    }

    type AbiVersion = unsafe extern "C" fn() -> u32;
    type RuntimeInit = unsafe extern "C" fn() -> *mut c_void;
    type SetDeployment = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> i32;
    type SetCredential = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> i32;
    type LoadCatalog = unsafe extern "C" fn(*mut c_void, *const c_char) -> i32;
    type StreamOpen = unsafe extern "C" fn(*mut c_void, *const c_char, usize, *mut *mut c_void) -> i32;
    type StreamNext = unsafe extern "C" fn(*mut c_void, u32, *mut UmerEvent) -> i32;
    type StreamCancel = unsafe extern "C" fn(*mut c_void);
    type Status = unsafe extern "C" fn(*mut c_void, *mut UmerEvent) -> i32;
    type StreamClose = unsafe extern "C" fn(*mut c_void);
    type StringFree = unsafe extern "C" fn(*mut c_char);

    /// Runtime 句柄 + 函数指针集。Umber 句柄内部加锁、任意线程可用（见接口文档 §3.4）。
    pub struct Api {
        pub rt: *mut c_void,
        pub set_deployment: SetDeployment,
        pub set_credential: SetCredential,
        pub load_catalog: LoadCatalog,
        pub stream_open: StreamOpen,
        pub stream_next: StreamNext,
        pub stream_cancel: StreamCancel,
        pub status_fn: Status,
        pub stream_close: StreamClose,
        pub string_free: StringFree,
    }
    unsafe impl Send for Api {}
    unsafe impl Sync for Api {}

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(filename: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> isize;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn lookup(module: isize, name: &str) -> Option<isize> {
        let cname = std::ffi::CString::new(name).ok()?;
        Some(GetProcAddress(module, cname.as_ptr() as *const u8))
    }

    /// 进程级单例：加载 DLL → 校验 ABI major → runtime_init → 可选加载 catalog
    pub fn global() -> Option<&'static Api> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(|| unsafe {
            // 候选：exe 同目录 → Umber 集成包绝对路径
            let mut hmodule: isize = 0;
            for path in [
                "umber_ffi.dll",
                "E:\\个人项目\\Umber API\\软件实际调用所需文件及接口文档\\lib\\umber_ffi.dll",
            ] {
                let wide = to_wide(path);
                hmodule = LoadLibraryW(wide.as_ptr());
                if hmodule != 0 { break; }
            }
            if hmodule == 0 { return None; }
            let abi: AbiVersion = std::mem::transmute(lookup(hmodule, "runtime_abi_version")?);
            if (abi() >> 16) != 0 { return None; }
            let init: RuntimeInit = std::mem::transmute(lookup(hmodule, "runtime_init")?);
            let rt = init();
            if rt.is_null() { return None; }
            macro_rules! sym {
                ($n:literal, $t:ty) => {
                    match lookup(hmodule, $n) {
                        Some(p) => std::mem::transmute::<isize, $t>(p),
                        None => return None,
                    }
                };
            }
            let api = Api {
                rt,
                set_deployment: sym!("runtime_set_deployment", SetDeployment),
                set_credential: sym!("runtime_set_credential", SetCredential),
                load_catalog: sym!("runtime_load_catalog", LoadCatalog),
                stream_open: sym!("runtime_stream_open", StreamOpen),
                stream_next: sym!("runtime_stream_next", StreamNext),
                stream_cancel: sym!("runtime_stream_cancel", StreamCancel),
                status_fn: sym!("runtime_status", Status),
                stream_close: sym!("runtime_stream_close", StreamClose),
                string_free: sym!("runtime_string_free", StringFree),
            };
            // 离线模型目录（可选，缺了就当 unknown，不阻塞）
            if let Ok(cpath) = std::ffi::CString::new("catalog.json") {
                let _ = (api.load_catalog)(rt, cpath.as_ptr());
            }
            Some(api)
        })
        .as_ref()
    }
}

/// AI 流消息：Delta 正文增量 / Reasoning 思维链 / Usage 用量 / Done 完成 / Error 失败
pub enum AiMsg {
    Delta(String),
    Reasoning(String),
    Usage(u64, u64),
    Done(String),
    Error(String),
}

/// 从事件 JSON 里取数字字段
fn json_num_field(json: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{}\":", key);
    let from = json.find(&pat)? + pat.len();
    let rest = json[from..].trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

/// 从事件 JSON 里取字符串字段（处理转义；找不到返回空串）
fn json_string_field(json: &str, key: &str) -> String {
    let pat = format!("\"{}\":", key);
    let mut from = 0usize;
    while let Some(rel) = json[from..].find(&pat) {
        let after = &json[from + rel + pat.len()..];
        let trimmed = after.trim_start();
        if trimmed.starts_with('"') {
            return json_unescape(&trimmed[1..]);
        }
        from += rel + pat.len();
    }
    String::new()
}

/// 解析 JSON 字符串字面量直到未转义的收尾引号
fn json_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(cp) { out.push(ch); }
                    }
                }
                Some(other) => out.push(other),
                None => break,
            },
            other => out.push(other),
        }
    }
    out
}

/// Umber 真流式对话：Deployment → 凭据 → 系统提示(带文件上下文) → 拉流 → Delta/Done/Error，可取消
fn umber_chat(
    route: &ModelEntry,
    demo: bool,
    history: &[(bool, String)],
    file_ctx: &Option<(String, String)>,
    cancel: &std::sync::atomic::AtomicBool,
    tx: &std::sync::mpsc::Sender<AiMsg>,
) {
    let Some(api) = umber::global() else {
        let _ = tx.send(AiMsg::Error("未找到 umber_ffi.dll（Umber 运行时不可用，请把 DLL 放到 exe 同目录）".into()));
        return;
    };
    unsafe {
        let (req_model, use_demo) = if demo {
            ("demo".to_string(), true)
        } else {
            (format!("aine/{}", route.id), false)
        };
        if !use_demo {
            // 1) Deployment：每厂商独立注册（多厂商并存），5 种协议原样透传
            let dep_id = format!("aine/{}", route.id);
            let endpoint = route.base_url.trim_end_matches('/').trim_end_matches("/chat/completions").trim_end_matches("/messages");
            let config = format!(
                "{{\"id\":\"{}\",\"provider_id\":\"{}\",\"protocol\":\"{}\",\"endpoint_url\":\"{}\",\"model_id\":\"{}\",\"credential_ref\":\"aine/{}\"}}",
                dep_id, route.provider, route.protocol, endpoint, route.model, route.id
            );
            let cdep = std::ffi::CString::new(config).unwrap();
            if (api.set_deployment)(api.rt, cdep.as_ptr(), cdep.as_bytes().len()) != umber::OK {
                let _ = tx.send(AiMsg::Error(format!("Deployment 配置不合法（厂商 {}，检查 protocol / Base URL / model）", route.id)));
                return;
            }
            // 2) 凭据只在内存（Umber 不落盘），每厂商独立 ref
            let cref = std::ffi::CString::new(format!("aine/{}", route.id)).unwrap();
            let secret = std::ffi::CString::new(route.api_key.trim()).unwrap();
            let _ = (api.set_credential)(api.rt, cref.as_ptr(), secret.as_ptr());
        }

        // 3) 请求：系统提示（角色 + 当前文件上下文）+ 历史
        let mut msgs = String::from("[");
        let mut first = true;
        let sys = match file_ctx {
            Some((name, content)) => format!(
                "You are the Aine language programming assistant inside Aine Studio IDE. Answer concisely in the user's language. The user is editing `examples/{name}`:
```aine
{content}
```",
                name = name, content = content
            ),
            None => "You are the Aine language programming assistant inside Aine Studio IDE. Answer concisely in the user's language.".to_string(),
        };
        msgs.push_str(&format!("{{\"role\":\"system\",\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}]}}", json_escape(&sys)));
        for (is_user, msg) in history.iter().take(history.len().saturating_sub(1)) {
            if msg.is_empty() { continue; }
            let role = if *is_user { "user" } else { "assistant" };
            msgs.push_str(&format!(
                ",{{\"role\":\"{}\",\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}]}}",
                role, json_escape(msg)
            ));
        }
        msgs.push(']');
        let req = format!("{{\"model\":\"{}\",\"messages\":{}}}", req_model, msgs);
        let creq = std::ffi::CString::new(req).unwrap();

        // 4) 拉流
        let mut stream: *mut std::ffi::c_void = std::ptr::null_mut();
        let rc = (api.stream_open)(api.rt, creq.as_ptr(), creq.as_bytes().len(), &mut stream);
        if rc != umber::OK || stream.is_null() {
            let why = match rc {
                -5 => "Deployment 未配置或缺 API Key",
                -1 => "参数为空",
                -3 => "请求 JSON 不合法",
                _ => "打开流失败",
            };
            let _ = tx.send(AiMsg::Error(format!("Umber stream_open: {} (code {})", why, rc)));
            return;
        }
        let mut acc = String::new();
        loop {
            let mut ev = umber::UmerEvent { sequence: 0, json: std::ptr::null_mut(), json_len: 0 };
            let st = (api.stream_next)(stream, 1000, &mut ev);
            if st == umber::EVENT {
                let json = if ev.json.is_null() { String::new() } else {
                    std::str::from_utf8(std::slice::from_raw_parts(ev.json as *const u8, ev.json_len))
                        .unwrap_or("").to_string()
                };
                (api.string_free)(ev.json);
                match json_string_field(&json, "type").as_str() {
                    "text_delta" => {
                        let delta = json_string_field(&json, "delta");
                        if !delta.is_empty() {
                            acc.push_str(&delta);
                            let _ = tx.send(AiMsg::Delta(delta));
                        }
                    }
                    "reasoning_delta" => {
                        // 思维链（deepseek-reasoner 等推理模型）
                        let delta = json_string_field(&json, "delta");
                        if !delta.is_empty() {
                            let _ = tx.send(AiMsg::Reasoning(delta));
                        }
                    }
                    "usage_updated" => {
                        // token 用量（字段名宽松匹配）
                        let pin = json_num_field(&json, "prompt_tokens")
                            .or_else(|| json_num_field(&json, "input_tokens"))
                            .or_else(|| json_num_field(&json, "input"));
                        let pout = json_num_field(&json, "completion_tokens")
                            .or_else(|| json_num_field(&json, "output_tokens"))
                            .or_else(|| json_num_field(&json, "output"));
                        if pin.is_some() || pout.is_some() {
                            let _ = tx.send(AiMsg::Usage(pin.unwrap_or(0), pout.unwrap_or(0)));
                        }
                    }
                    "failed" => {
                        let mut msg = json_string_field(&json, "message");
                        if msg.is_empty() { msg = json.clone(); }
                        let _ = tx.send(AiMsg::Error(msg));
                        break;
                    }
                    "completed" | "cancelled" => break,
                    _ => {} // started / reasoning_* / usage_updated / toolcall_* 暂不呈现
                }
            } else if st == umber::CLOSED {
                break;
            } else if st == umber::WOULD_BLOCK {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = (api.stream_cancel)(stream);
                    if !acc.is_empty() { acc.push_str("
（已取消）"); }
                    let _ = tx.send(AiMsg::Done(acc));
                    (api.stream_close)(stream);
                    return;
                }
                continue;
            } else {
                let _ = tx.send(AiMsg::Error(format!("Umber stream_next 错误码 {}", st)));
                break;
            }
        }
        (api.stream_close)(stream);
        if !acc.is_empty() {
            let _ = tx.send(AiMsg::Done(acc));
        }
    }
}

/// AI 回复的轻量 Markdown 渲染：``` 代码块 → 等宽深底框，其余按行纯文本
fn render_ai_markdown(ui: &mut egui::Ui, text: &str) {
    let mut in_code = false;
    let mut code_buf = String::new();
    for line in text.split('\n') {
        if line.trim_start().starts_with("```") {
            if in_code {
                let buf = code_buf.clone();
                egui::Frame::none()
                    .fill(theme::BG_INPUT)
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(buf.trim_end()).color(theme::FG_BRIGHT).size(11.0).monospace());
                    });
                code_buf.clear();
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
        } else {
            ui.label(egui::RichText::new(line).color(theme::FG).size(12.0));
        }
    }
    if !code_buf.is_empty() {
        let buf = code_buf.clone();
        egui::Frame::none()
            .fill(theme::BG_INPUT)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(buf).color(theme::FG_BRIGHT).size(11.0).monospace());
            });
    }
}

fn app_or_self_debug(app: &mut App) {
    app.run_debugger();
}

fn main() -> eframe::Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Aine Studio"),
        // wgpu(DXGI/D3D12) 渲染：本机 OpenGL/WGL present 路径异常时的替代
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("Aine Studio", options, Box::new(|cc| {
        load_cjk_fonts(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);
        Box::new(App::new())
    }))
}
