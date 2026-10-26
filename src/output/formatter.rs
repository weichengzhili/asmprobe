use std::collections::BTreeMap;

use crate::config::OutputFormat;
use crate::domain::{AsmLine, CompileOutput, CompilerInfo, CompilerSource, Language, Severity};

pub fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

pub fn source_str(s: CompilerSource) -> &'static str {
    match s {
        CompilerSource::Local => "local",
        CompilerSource::Remote => "godbolt",
    }
}

fn asm_to_text(out: &CompileOutput) -> String {
    out.asm
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_compile(out: &CompileOutput, format: OutputFormat, _color: bool) -> String {
    match format {
        OutputFormat::Json => {
            let diags = out
                .diagnostics
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "severity": severity_str(d.severity),
                        "message": d.message,
                        "line": d.line,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "source": out.source,
                "asm": out.asm,
                "exit_code": out.exit_code,
                "diagnostics": diags,
            })
            .to_string()
        }
        OutputFormat::Text | OutputFormat::Html => {
            let mut s = String::new();
            for d in &out.diagnostics {
                s.push_str(&format!("[{}] {}\n", severity_str(d.severity), d.message));
            }
            let asm = asm_to_text(out);
            s.push_str(&asm);
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s
        }
    }
}

pub fn render_compilers(list: &[CompilerInfo], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => {
            let arr = list
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "name": c.name,
                        "version": c.version,
                        "source": source_str(c.source),
                        "is_default": c.is_default,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!(arr).to_string()
        }
        OutputFormat::Text | OutputFormat::Html => {
            let mut s = String::new();
            for c in list {
                let ver = c.version.as_deref().unwrap_or("-");
                let def = if c.is_default { " (default)" } else { "" };
                s.push_str(&format!(
                    "{:<20} {:<10} {:<8}{}\n",
                    c.id,
                    ver,
                    source_str(c.source),
                    def
                ));
            }
            s
        }
    }
}

pub fn render_languages(format: OutputFormat) -> String {
    let langs: Vec<Language> = Language::all().to_vec();
    match format {
        OutputFormat::Json => {
            let arr = langs
                .iter()
                .map(|l| {
                    serde_json::json!({
                        "id": l.godbolt_id(),
                        "name": l.display_name(),
                        "extension": l.extension(),
                        "default_args": l.default_args(),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!(arr).to_string()
        }
        OutputFormat::Text | OutputFormat::Html => {
            let mut s = String::new();
            for l in &langs {
                let args = l.default_args().join(" ");
                s.push_str(&format!(
                    "{:<6} ext={:<4} default_args=[{}]\n",
                    l.display_name(),
                    l.extension(),
                    args
                ));
            }
            s
        }
    }
}

/// 生成源码（左）/ 汇编（右）对照 HTML 页面（单文件、无外部依赖）。
pub fn render_compare_html(out: &CompileOutput, name: &str) -> String {
    let src_lines: Vec<&str> = out.source.lines().collect();

    let mut buckets: BTreeMap<u32, Vec<&AsmLine>> = BTreeMap::new();
    let mut unattributed: Vec<&AsmLine> = Vec::new();
    for a in &out.asm {
        match a.source.as_ref().and_then(|s| s.line) {
            Some(l) => buckets.entry(l).or_default().push(a),
            None => unattributed.push(a),
        }
    }

    let mut body = String::new();
    if !unattributed.is_empty() {
        body.push_str(&render_row(0, "", &unattributed, true));
    }
    for (i, src) in src_lines.iter().enumerate() {
        let line_no = (i + 1) as u32;
        let asm_for: &[&AsmLine] = match buckets.get(&line_no) {
            Some(v) => v.as_slice(),
            None => &[],
        };
        body.push_str(&render_row(line_no, src, asm_for, false));
    }

    let mut html = String::from(
        "<!DOCTYPE html><html lang=\"zh\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
    );
    html.push_str("<title>asmprobe: ");
    html.push_str(&escape(name));
    html.push_str("</title><style>");
    html.push_str(CSS);
    html.push_str("</style></head><body>");
    html.push_str("<h1>asmprobe — 源码 / 汇编对照 · ");
    html.push_str(&escape(name));
    html.push_str(
        "</h1><div class=\"legend\">左：源码　右：汇编（括号内为对应源码行号）</div>\
         <div class=\"container\">",
    );
    html.push_str(&body);
    html.push_str("</div></body></html>");
    html
}

fn render_row(line_no: u32, src: &str, asm: &[&AsmLine], unattributed: bool) -> String {
    let left_no = if unattributed {
        String::new()
    } else {
        line_no.to_string()
    };
    let left = escape(src);
    let right: String = asm
        .iter()
        .map(|a| {
            let badge = a
                .source
                .as_ref()
                .and_then(|s| s.line)
                .map(|l| l.to_string())
                .unwrap_or_default();
            format!(
                "<div class=\"asm\"><span class=\"badge\">{}</span>{}</div>",
                badge,
                escape(&a.text)
            )
        })
        .collect();
    format!(
        "<div class=\"row\"><div class=\"ln\">{left_no}</div>\
         <div class=\"src\">{left}</div><div class=\"asmcol\">{right}</div></div>",
        left_no = left_no,
        left = left,
        right = right
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const CSS: &str = "\
* { box-sizing: border-box; }\
body { margin:0; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; \
background:#0f1117; color:#d6deeb; }\
h1 { font-size:15px; padding:12px 16px; margin:0; background:#161a23; border-bottom:1px solid #2a2f3a; }\
.legend { padding:6px 16px; color:#7f8aa3; font-size:12px; background:#12151c; }\
.container { display:flex; flex-direction:column; }\
.row { display:flex; border-bottom:1px solid #1c212b; align-items:stretch; }\
.row:hover { background:#171c27; }\
.ln { width:44px; flex:none; text-align:right; padding:2px 8px; color:#5b6678; \
user-select:none; border-right:1px solid #1c212b; }\
.src { width:50%; flex:none; padding:2px 12px; white-space:pre-wrap; word-break:break-all; \
border-right:1px solid #2a2f3a; }\
.asmcol { width:50%; padding:2px 12px; white-space:pre-wrap; word-break:break-all; }\
.asm { white-space:pre-wrap; }\
.badge { display:inline-block; min-width:26px; color:#5b6678; margin-right:8px; user-select:none; }";
