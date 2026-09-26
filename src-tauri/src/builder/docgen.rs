//! API 文档生成：扫描 {libs,game}/{common,server,client}/api/*.lua，
//! 解析头部说明 + 成员绑定/函数（含上方注释块）；纯转发文件（仅一行
//! `return require('...')`）顺链解析目标模块 main 与同目录 *.d.lua 签名。
//! 输出 Markdown 到 .bgd/doc/api_generated/<code_set>/<side>/<module>.md + 总索引。
//!
//! 规范化反向校验：模块缺头部说明 / 成员零注释时输出 warn 日志——
//! 生成过程同时是 api/main 写法规范度的检查器。

use super::{code_set_dir, LogFn};
use crate::config::BgdConfig;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// 单个成员（M.xxx / function M.xxx / 大写表类成员如 Sound.new）
#[derive(Default)]
struct MemberDoc {
    table: String,
    name: String,
    sig: String,
    doc: Vec<String>,
}

/// 一个 lua 文件解析出的文档结构
#[derive(Default)]
struct LuaDoc {
    header: Vec<String>,
    members: Vec<MemberDoc>,
    /// 纯转发目标（require 路径）；非纯转发为 None
    forward: Option<String>,
    /// @bgd:no-doc 标记（内部件不进文档）
    no_doc: bool,
}

/// 注释行剥离 "--" 前缀（--- 文档注释的多余横杠一并剥除）；非注释行返回 None
fn strip_comment(line: &str) -> Option<String> {
    let t = line.trim_start();
    t.strip_prefix("--")
        .map(|s| s.trim_start_matches('-').trim().to_string())
}

/// 分割线（-- ==== / -- ---- 类，剥完只剩 - 或 =）
fn is_divider(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '='))
}

/// EmmyLua 注解行（---@class / ---@param 等）
fn is_annotation(s: &str) -> bool {
    s.starts_with('@')
}

/// 过滤注释行：keep_annotations=false 时丢弃 @注解 与分割线
fn keep_comment(s: &str, keep_annotations: bool) -> bool {
    if is_divider(s) {
        return false;
    }
    if !keep_annotations && is_annotation(s) {
        return false;
    }
    true
}

/// 解析 function/M 绑定行，返回 (table, name, sig)
fn parse_member_line(line: &str) -> Option<(String, String, String)> {
    let t = line.trim();
    // function X.y(...) / function X:y(...)
    if let Some(rest) = t.strip_prefix("function") {
        let rest = rest.trim_start();
        let name_end = rest.find('(')?;
        let name_part = rest[..name_end].trim();
        let (tab, name) = name_part
            .rsplit_once(['.', ':'])
            .map(|(a, b)| (a.to_string(), b.to_string()))?;
        if tab.is_empty() || name.is_empty() {
            return None;
        }
        return Some((tab, name, t.to_string()));
    }
    // X.y = ...（仅限 M 或大写开头表，排除 core.xxx/override.xxx 等内部注入）
    let eq = t.find('=')?;
    let lhs = t[..eq].trim();
    let (tab, name) = lhs
        .rsplit_once('.')
        .map(|(a, b)| (a.to_string(), b.to_string()))?;
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let ok_table = tab == "M" || tab.chars().next().is_some_and(|c| c.is_uppercase());
    if !ok_table {
        return None;
    }
    Some((tab.to_string(), name.to_string(), t.to_string()))
}

/// 解析 return require('...')（纯转发检测用）
fn parse_forward(line: &str) -> Option<String> {
    let t = line.trim();
    let rest = t.strip_prefix("return")?.trim_start();
    let rest = rest.strip_prefix("require")?.trim_start();
    let rest = rest.strip_prefix('(')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

/// 跳过标记命中判定：注释文本 c 与配置标记 skip 都剥掉 "--" 前缀后包含匹配
/// （strip_comment 已剥掉注释行的 "-- "，配置里的 "-- @bgd:xxx" 需对齐）。
fn hit_skip(c: &str, skip: &str) -> bool {
    if skip.is_empty() {
        return false;
    }
    let marker = skip.trim_start_matches('-').trim();
    !marker.is_empty() && c.contains(marker)
}

/// 解析一个 lua 文件为文档结构。
/// keep_annotations=true 用于 .d.lua（保留 ---@param/@return 等注解行）。
/// skip_annotation=doc 跳过标记（cfg.doc_skip_annotation；空串 = 禁用）。
fn parse_lua(path: &Path, keep_annotations: bool, skip_annotation: &str) -> Result<LuaDoc> {
    let content = fs::read_to_string(path)?;
    let mut doc = LuaDoc::default();
    let mut lines = content.lines().peekable();

    // 头部：文件首个代码行之前的连续注释/空行
    let mut header: Vec<String> = Vec::new();
    let mut no_doc = false;
    while let Some(&line) = lines.peek() {
        let t = line.trim();
        if t.is_empty() {
            lines.next();
            continue;
        }
        match strip_comment(line) {
            Some(c) => {
                // 跳过标记在 keep_comment 前判定（@ 注解行会被滤掉，提前拦）
                if hit_skip(&c, skip_annotation) {
                    no_doc = true;
                }
                if keep_comment(&c, keep_annotations) {
                    header.push(c);
                }
                lines.next();
            }
            None => break,
        }
    }
    // 去头尾空行
    while header.first().is_some_and(|s| s.is_empty()) {
        header.remove(0);
    }
    while header.last().is_some_and(|s| s.is_empty()) {
        header.pop();
    }
    doc.header = header;
    doc.no_doc = no_doc;
    if no_doc {
        return Ok(doc); // 标记 no-doc：成员不解析
    }

    // 成员扫描：pending 存连续注释块，空行清零
    // 成员文档保留 @param/@return 等注解行（签名即文档），只滤分割线
    let mut pending: Vec<String> = Vec::new();
    let mut code_lines = 0usize;
    let mut forward: Option<String> = None;
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            pending.clear();
            continue;
        }
        if let Some(c) = strip_comment(line) {
            if hit_skip(&c, skip_annotation) {
                no_doc = true;
            }
            if !is_divider(&c) {
                pending.push(c);
            }
            continue;
        }
        code_lines += 1;
        if let Some(f) = parse_forward(line) {
            forward = Some(f);
        }
        if let Some((tab, name, sig)) = parse_member_line(line) {
            doc.members.push(MemberDoc {
                table: tab,
                name,
                sig,
                doc: std::mem::take(&mut pending),
            });
        } else {
            pending.clear();
        }
    }
    // 纯转发：全文只有一个代码行且是 return require
    doc.forward = if code_lines == 1 { forward } else { None };
    doc.no_doc = doc.no_doc || no_doc;
    Ok(doc)
}

/// require 路径（libs.xxx.yyy / src.xxx.yyy）解析为 code set 下的实际文件
fn resolve_require(root_prefix: &str, src_root: &Path, req: &str) -> Option<PathBuf> {
    let mut segs = req.split('.');
    if segs.next()? != root_prefix {
        return None;
    }
    let rel: Vec<&str> = segs.collect();
    if rel.is_empty() {
        return None;
    }
    let p = src_root.join(rel.join("/")).with_extension("lua");
    p.is_file().then_some(p)
}

/// 成员概览表里的首行说明（跳过空行取首行）
fn first_doc_line(doc: &[String]) -> String {
    doc.iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default()
        .replace('|', "\\|")
}

/// 单元格转义（| 会破坏表格）
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// 成员文档结构化渲染：
/// 叙述行直出；@param/@return 归集成表格；其余 @注解（@class/@field 等）并入叙述。
fn render_member_doc(doc: &[String]) -> String {
    let mut narrative: Vec<&str> = Vec::new();
    let mut params: Vec<(String, String, String)> = Vec::new(); // (名, 类型, 说明)
    let mut returns: Vec<(String, String)> = Vec::new(); // (类型, 说明)

    for line in doc {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("@param") {
            let rest = rest.trim_start();
            // 形态：name[?] type 说明…（type 可能含空格如 '"a"'|'"b"'，说明可缺省）
            let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let name = &rest[..name_end];
            let after = rest[name_end..].trim_start();
            // 类型与说明的切分：最后一个「空白 + 中文字符/大写字母开头的词」之前都算类型
            // 简化：类型无空格则直接取首段，否则按已知注解形态尽量切
            let (ty, desc) = split_type_desc(after);
            params.push((cell(name), cell(&ty), cell(&desc)));
        } else if let Some(rest) = t.strip_prefix("@return") {
            let (ty, desc) = split_type_desc(rest.trim_start());
            returns.push((cell(&ty), cell(&desc)));
        } else {
            narrative.push(line.as_str());
        }
    }

    let mut out = String::new();
    // 去头尾空行的叙述
    while narrative.first().is_some_and(|s| s.trim().is_empty()) {
        narrative.remove(0);
    }
    while narrative.last().is_some_and(|s| s.trim().is_empty()) {
        narrative.pop();
    }
    if !narrative.is_empty() {
        for l in &narrative {
            out.push_str(l);
            out.push('\n');
        }
        out.push('\n');
    }
    if !params.is_empty() {
        out.push_str("| 参数 | 类型 | 说明 |\n| --- | --- | --- |\n");
        for (n, ty, d) in &params {
            out.push_str(&format!("| `{n}` | `{ty}` | {d} |\n"));
        }
        out.push('\n');
    }
    if !returns.is_empty() {
        out.push_str("| 返回值 | 说明 |\n| --- | --- |\n");
        for (ty, d) in &returns {
            out.push_str(&format!("| `{ty}` | {d} |\n"));
        }
        out.push('\n');
    }
    out
}

/// 切分「类型 说明」：类型段内无空白则首段即类型；否则从右往左找第一个
/// 以中文/中括号/反引号/小写字母开头的说明段（Lua 类型惯用小写，保守取首段）。
fn split_type_desc(s: &str) -> (String, String) {
    let s = s.trim();
    if s.is_empty() {
        return (String::new(), String::new());
    }
    match s.find(char::is_whitespace) {
        None => (s.to_string(), String::new()),
        Some(i) => {
            let ty = &s[..i];
            let desc = s[i..].trim_start();
            (ty.to_string(), desc.to_string())
        }
    }
}

/// 生成一个模块的 Markdown
fn render_module_md(
    code_set: &str,
    side: &str,
    module: &str,
    api_rel: &str,
    main: &LuaDoc,
    d_sigs: &[(String, LuaDoc)],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# bgd_api.{side}.{module}\n\n"));
    out.push_str(&format!("> 来源：`{api_rel}`（{code_set}）\n\n"));

    if !main.header.is_empty() {
        for l in &main.header {
            out.push_str(l);
            out.push('\n');
        }
        out.push('\n');
    } else {
        out.push_str("_（缺模块头部说明）_\n\n");
    }

    if !main.members.is_empty() {
        out.push_str("## 成员一览\n\n| 成员 | 说明 |\n| --- | --- |\n");
        for m in &main.members {
            out.push_str(&format!(
                "| `{}.{}` | {} |\n",
                m.table,
                m.name,
                first_doc_line(&m.doc)
            ));
        }
        out.push_str("\n## 成员详情\n\n");
        for m in &main.members {
            out.push_str(&format!("### `{}.{}`\n\n", m.table, m.name));
            if !m.doc.is_empty() {
                out.push_str(&render_member_doc(&m.doc));
            }
            out.push_str(&format!("```lua\n{}\n```\n\n", m.sig));
        }
    }

    for (d_name, d_doc) in d_sigs {
        if d_doc.members.is_empty() {
            continue;
        }
        out.push_str(&format!("## 类型签名（{d_name}）\n\n```lua\n"));
        for m in &d_doc.members {
            for l in &m.doc {
                out.push_str(&format!("---{l}\n"));
            }
            out.push_str(&format!("{}\n\n", m.sig));
        }
        out.push_str("```\n");
    }
    out
}

/// 扫描全部 api 目录生成文档，返回生成的文件清单。
/// out_dir=None 时用配置 cfg.api_generated_dir（相对项目根）。
pub fn generate_api_docs(
    bgd_root: &Path,
    cfg: &BgdConfig,
    out_dir: Option<&Path>,
    log: &LogFn,
) -> Result<Vec<PathBuf>> {
    let out_root = match out_dir {
        Some(p) if p.is_absolute() => p.to_path_buf(),
        Some(p) => bgd_root.join(p),
        None => cfg.abs(bgd_root, &cfg.api_generated_dir),
    };
    if out_root.is_dir() {
        fs::remove_dir_all(&out_root)?;
    }
    fs::create_dir_all(&out_root)?;

    let mut generated: Vec<PathBuf> = Vec::new();
    let mut index: Vec<(String, String, String, String)> = Vec::new(); // (code_set, side, module, 首行说明)

    for code_set in ["libs", "game"] {
        let (dir, _) = code_set_dir(code_set, cfg);
        // 输出目录用磁盘目录名（libs/src），与 .bgd 下 code set 目录一一对应
        let out_name = Path::new(dir)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let src_root = cfg.abs(bgd_root, dir);
        let root_prefix = Path::new(dir)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        for side in ["common", "server", "client"] {
            let api_dir = src_root.join(side).join("api");
            if !api_dir.is_dir() {
                continue;
            }
            let mut modules: Vec<PathBuf> = fs::read_dir(&api_dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|e| e == "lua")
                        && p.file_name().is_some_and(|n| n != "init.lua")
                })
                .collect();
            modules.sort();

            for api_file in modules {
                let module = api_file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let api_rel = format!("{root_prefix}/{side}/api/{module}.lua");
                let mut main_doc = parse_lua(&api_file, false, &cfg.doc_skip_annotation)?;

                // 跳过标记命中：内部件不进文档
                if main_doc.no_doc {
                    log(&format!("[skip] no-doc: {api_rel}"));
                    continue;
                }
                let mut d_sigs: Vec<(String, LuaDoc)> = Vec::new();

                // 纯转发：顺链解析目标 main + 同目录 *.d.lua
                if let Some(req) = main_doc.forward.clone() {
                    if let Some(target) = resolve_require(&root_prefix, &src_root, &req) {
                        let mut tdoc = parse_lua(&target, false, &cfg.doc_skip_annotation)?;
                        // 头部与成员：转发文件自身有头则用转发文件的，否则用目标的
                        if main_doc.header.is_empty() {
                            main_doc.header = tdoc.header.clone();
                        }
                        main_doc.members.append(&mut tdoc.members);
                        if let Some(dir) = target.parent() {
                            let mut dls: Vec<PathBuf> = fs::read_dir(dir)?
                                .filter_map(|e| e.ok().map(|e| e.path()))
                                .filter(|p| {
                                    p.file_name()
                                        .and_then(|n| n.to_str())
                                        .is_some_and(|n| n.ends_with(".d.lua"))
                                })
                                .collect();
                            dls.sort();
                            for dl in dls {
                                let name = dl
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();
                                d_sigs.push((name, parse_lua(&dl, true, &cfg.doc_skip_annotation)?));
                            }
                        }
                    }
                }

                // 规范化反向校验
                if main_doc.header.is_empty() {
                    log(&format!("[warn] api 模块缺头部说明: {api_rel}"));
                }
                let naked = main_doc.members.iter().filter(|m| m.doc.is_empty()).count();
                if !main_doc.members.is_empty() && naked > 0 {
                    log(&format!(
                        "[warn] {api_rel}: {naked}/{} 成员无注释",
                        main_doc.members.len()
                    ));
                }

                let md = render_module_md(code_set, side, &module, &api_rel, &main_doc, &d_sigs);
                let out = out_root.join(&out_name).join(side).join(format!("{module}.md"));
                if let Some(parent) = out.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&out, md)?;
                log(&format!("[gen] api 文档: {}", out.display()));
                index.push((
                    out_name.clone(),
                    side.to_string(),
                    module.clone(),
                    first_doc_line(&main_doc.header),
                ));
                generated.push(out);
            }
        }
    }

    // 总索引
    let mut idx = String::from("# API 文档索引（自动生成，请勿手改）\n\n");
    let mut cur = String::new();
    for (cs, side, module, summary) in &index {
        let group = format!("{cs}/{side}");
        if group != cur {
            cur = group.clone();
            idx.push_str(&format!("\n## {cur}\n\n"));
        }
        idx.push_str(&format!(
            "- [bgd_api.{side}.{module}]({cs}/{side}/{module}.md) — {summary}\n"
        ));
    }
    let idx_path = out_root.join("README.md");
    fs::write(&idx_path, idx)?;
    generated.push(idx_path);
    log(&format!("[ok] api 文档生成完成: {} 个模块", index.len()));
    Ok(generated)
}
