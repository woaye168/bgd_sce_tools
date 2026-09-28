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
    /// 跨模块转发绑定的源变量与源成员（`M.x = sub.y` → (sub, y)）；仅当 doc 为空时用于回溯取注释
    bind: Option<(String, String)>,
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

/// 解析 function/M 绑定行，返回 (table, name, sig, bind)
fn parse_member_line(line: &str) -> Option<(String, String, String, Option<(String, String)>)> {
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
        return Some((tab, name, t.to_string(), None));
    }
    // X.y = ...（仅限 M 或大写开头表，排除 core.xxx/override.xxx 等内部注入）
    let eq = t.find('=')?;
    let lhs = t[..eq].trim();
    let rhs = t[eq + 1..].trim();
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
    // 右值是另一变量的成员引用（`sub.y`，无调用无表构造）→ 记录转发绑定源
    let bind = rhs
        .split_once('.')
        .filter(|(var, member)| {
            !var.is_empty()
                && var.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !member.is_empty()
                && member.chars().all(|c| c.is_alphanumeric() || c == '_')
        })
        .map(|(var, member)| (var.to_string(), member.to_string()));
    Some((tab, name, t.to_string(), bind))
}

/// 从一行中提取 require 路径（剥到引号内为止）
fn extract_require(rest: &str) -> Option<String> {
    let rest = rest.strip_prefix("require")?.trim_start();
    let rest = rest.strip_prefix('(')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

/// 纯转发判定：一行形态的 `return require('...')`
fn parse_forward(line: &str) -> Option<String> {
    let t = line.trim();
    let rest = t.strip_prefix("return")?.trim_start();
    extract_require(rest)
}

/// 转发模块表判定：两行形态的 `local M = require('...')`（后续 `return M`）
fn parse_forward_require_local(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    let rest = t.strip_prefix("local")?.trim_start();
    let eq = rest.find('=')?;
    let var = rest[..eq].trim();
    if var.is_empty() || !var.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let req = extract_require(rest[eq + 1..].trim_start())?;
    Some((var.to_string(), req))
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
    let mut forward_local: Option<(String, String)> = None; // (变量名, require 路径)
    let mut return_vars: Vec<String> = Vec::new();
    // require 局部变量 → 模块路径（供 M.x = sub.y 的 bind 回溯定位子模块文件）
    let mut require_vars: Vec<(String, String)> = Vec::new();
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
        } else if let Some(fl) = parse_forward_require_local(line) {
            forward_local = Some(fl.clone());
            require_vars.push(fl);
        } else if let Some(var) = t.strip_prefix("return").map(|r| r.trim()) {
            if !var.is_empty() && var.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return_vars.push(var.to_string());
            }
        }
        if let Some((tab, name, sig, bind)) = parse_member_line(line) {
            doc.members.push(MemberDoc {
                table: tab,
                name,
                sig,
                doc: std::mem::take(&mut pending),
                bind,
            });
        } else {
            pending.clear();
        }
    }
    // bind 回溯：成员的 doc 为空且 bind 源变量是 require 的子模块 → 追到子模块取注释
    if !require_vars.is_empty() {
        for m in doc.members.iter_mut() {
            // bind 成员自身注释无效时才回溯：空，或单行且等于源变量名（分类注释如 "-- array"）
            let own_useless = match m.doc.as_slice() {
                [] => true,
                [single] => single.trim().eq_ignore_ascii_case(
                    m.bind.as_ref().map(|(v, _)| v.as_str()).unwrap_or(""),
                ),
                _ => false,
            };
            if !own_useless {
                continue;
            }
            let Some((var, member)) = &m.bind else { continue };
            let Some((_, req)) = require_vars.iter().find(|(v, _)| v == var) else {
                continue;
            };
            // 只回溯一跳：require 路径解析为文件，找其 M.<member> 的注释
            let sub_path = resolve_submodule_file(path, req);
            if let Some(sub_path) = sub_path {
                if let Ok(sub_doc) = parse_lua(&sub_path, false, skip_annotation) {
                    if let Some(sm) = sub_doc.members.iter().find(|sm| &sm.name == member) {
                        if !sm.doc.is_empty() {
                            m.doc = sm.doc.clone();
                        }
                    }
                }
            }
        }
    }
    // 顺链判定（三种形态统一）：
    //   纯转发一行：`return require('...')`（唯一代码行）
    //   纯转发两行：`local M = require('...')` + `return M`（无自有成员）
    //   混合式：`local M = require('...')` + 自有 M.成员 + `return M`（顺链 + 自有成员合并覆盖）
    // 关键区别：混合式的 doc.members 非空（自有增量/覆写），顺链目标作基底合并
    let fwd = if code_lines == 1 {
        forward
    } else {
        match (&forward_local, return_vars.as_slice()) {
            (Some((var, req)), [rv]) if rv == var => Some(req.clone()),
            _ => None,
        }
    };
    doc.forward = fwd;
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

/// bind 回溯用的子模块定位：从当前文件向上找 code set 根（含 libs/src 前缀的父目录），
/// 再把 require 路径解析成实际文件。当前文件在 <code_set>/<...>/<file>.lua，
/// require 首段是 libs/src 根名 → 上溯到 code set 目录后拼接剩余段。
fn resolve_submodule_file(cur_file: &Path, req: &str) -> Option<PathBuf> {
    let mut segs: Vec<&str> = req.split('.').collect();
    if segs.len() < 2 {
        return None;
    }
    let root_prefix = segs.remove(0); // libs / src
    // 从当前文件向上找名为 root_prefix 的祖先目录（code set 根）
    let mut dir = cur_file.parent();
    while let Some(d) = dir {
        if d.file_name().and_then(|n| n.to_str()) == Some(root_prefix) {
            let p = d.join(segs.join("/")).with_extension("lua");
            return p.is_file().then_some(p);
        }
        dir = d.parent();
    }
    None
}

/// 成员概览表里的首行说明（跳过空行取首行）
/// 统一 Vue 安全转义（所有用户自由文本出口必须走这里）。
/// 规则：
/// - `{{` / `}}` → HTML 实体（Vue 插值表达式炸编译）
/// - 单 `{` / `}` → HTML 实体（Vue 模板里属性位置也会解析）
/// - `<` 后紧跟字母或 `/` → `&lt;`（防被当 HTML 标签；反引号/代码块内不动）
/// - `|` → `\|`（表格分隔符）
/// - 反引号奇数个时配平（防吞行）
fn escape_vue_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let chars: Vec<char> = s.chars().collect();
    let mut in_code = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            in_code = !in_code;
            out.push(c);
            i += 1;
            continue;
        }
        if in_code {
            out.push(c);
            i += 1;
            continue;
        }
        // {{ 插值表达式 → HTML 实体
        if c == '{' && chars.get(i + 1) == Some(&'{') {
            out.push_str("&#123;&#123;");
            i += 2;
            continue;
        }
        if c == '}' && chars.get(i + 1) == Some(&'}') {
            out.push_str("&#125;&#125;");
            i += 2;
            continue;
        }
        // 单 { } 也转义（Vue 模板属性位置会解析）
        if c == '{' {
            out.push_str("&#123;");
            i += 1;
            continue;
        }
        if c == '}' {
            out.push_str("&#125;");
            i += 1;
            continue;
        }
        // 裸 <word> / </word> → &lt;
        if c == '<' && chars.get(i + 1).is_some_and(|&n| n.is_alphabetic() || n == '/') {
            out.push_str("&lt;");
            i += 1;
            continue;
        }
        // 表格分隔符
        if c == '|' {
            out.push_str("\\|");
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    // 反引号配平（奇数个时末尾补一个，防吞行）
    if in_code {
        out.push('`');
    }
    out
}

fn first_doc_line(doc: &[String]) -> String {
    doc.iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default()
}

/// 单元格转义（统一走 escape_vue_text）
fn cell(s: &str) -> String {
    escape_vue_text(s)
}

/// 头部说明里的代码行识别（保守：明显是 Lua 语句的行才算，误包中文说明的代价是
/// 说明变代码块——可以接受；漏包代码的代价是 VitePress/Vue 模板编译炸——不能接受）。
/// 规则：含 `=` 赋值 / `local ` / `function` / `end` / 方法调用 `xxx.yyy(...)` 的行。
fn is_lua_code_line(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    // 明显 Lua 特征
    if t.starts_with("local ") || t.starts_with("function") || t == "end" || t == "end)" {
        return true;
    }
    // 赋值（= 且左侧是标识符/成员链）或方法调用
    if t.contains("=") && !t.starts_with("--") {
        return true;
    }
    // xxx.yyy(...) 调用形态
    if t.contains('.') && t.contains('(') && t.contains(')') {
        return true;
    }
    false
}

/// fun(...) 类型提取签名尾参（） 形参：fun 的类型串以括号收尾时，
/// 尾参说明 = 闭括号之后的内容 + 下一个字段的类型名（无法与类型串区分，并入说明）。
/// 例：`fun(rows: any[][]|nil, err: string|nil) 结果回调` →
///   (ty=`fun(rows: any[][]|nil, err: string|nil)`, desc=`结果回调`)
fn split_fun_type(s: &str) -> Option<(String, String)> {
    // 找与 fun( 匹配的闭括号（允许嵌套括号）
    let open = s.find('(')?;
    let mut depth = 0i32;
    let mut close = None;
    for (i, ch) in s.char_indices().skip(open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let ty = s[..=close].to_string();
    let desc = s[close + 1..].trim().to_string();
    Some((ty, desc))
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
            // 形态：name[?] type 说明…（type 可能含空格如 fun(...) 签名，说明可缺省）
            let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let name = &rest[..name_end];
            let after = rest[name_end..].trim_start();
            // fun(...) 类型串含空格/竖线，按括号配平提取；否则按首空白切分
            let (ty, desc) = if after.starts_with("fun(") {
                split_fun_type(after).unwrap_or_else(|| (after.to_string(), String::new()))
            } else {
                split_type_desc(after)
            };
            params.push((cell(name), cell(&ty), cell(&desc)));
        } else if let Some(rest) = t.strip_prefix("@return") {
            let after = rest.trim_start();
            let (ty, desc) = if after.starts_with("fun(") {
                split_fun_type(after).unwrap_or_else(|| (after.to_string(), String::new()))
            } else {
                split_type_desc(after)
            };
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
            out.push_str(&escape_vue_text(l));
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
        // 头部说明里的代码行包进 code block（VitePress/Vue 把 md 当 Vue 模板编译，
        // 裸文本里的 {{ }} / <id> 会炸模板——代码必须进 fenced block）
        let mut in_code = false;
        for l in &main.header {
            let is_code = is_lua_code_line(l);
            if is_code && !in_code {
                out.push_str("```lua\n");
                in_code = true;
            } else if !is_code && in_code {
                out.push_str("```\n\n");
                in_code = false;
            }
            out.push_str(&escape_vue_text(l));
            out.push('\n');
        }
        if in_code {
            out.push_str("```\n");
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
                cell(&first_doc_line(&m.doc))
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

                // 顺链（纯转发/混合式统一）：目标 main 成员作基底 + 同目录 *.d.lua；
                // api 自有成员后合并（同名覆盖 = 与运行时覆写语义一致）
                if let Some(req) = main_doc.forward.clone() {
                    if let Some(target) = resolve_require(&root_prefix, &src_root, &req) {
                        let mut tdoc = parse_lua(&target, false, &cfg.doc_skip_annotation)?;
                        // 头部：转发文件自身有头则用转发文件的，否则用目标的
                        if main_doc.header.is_empty() {
                            main_doc.header = tdoc.header.clone();
                        }
                        // 成员合并：基底(main) + 自有(api) 同名覆盖
                        let own = std::mem::take(&mut main_doc.members);
                        main_doc.members = std::mem::take(&mut tdoc.members);
                        for m in own {
                            if let Some(base) = main_doc
                                .members
                                .iter_mut()
                                .find(|b| b.table == m.table && b.name == m.name)
                            {
                                *base = m; // 覆写：api 自有成员覆盖 main 同名
                            } else {
                                main_doc.members.push(m); // 增量：api 新增成员
                            }
                        }
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
                    escape_vue_text(&first_doc_line(&main_doc.header)),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 混合式：api = 转发 + 自有增量 + 同名覆写 → main 基底 + 自有合并覆盖
    #[test]
    fn mixed_forward_merges_with_override() {
        let tmp = std::env::temp_dir().join("bgd_docgen_mix_test");
        let _ = fs::remove_dir_all(&tmp);
        let api_dir = tmp.join("libs/server/api");
        let mod_dir = tmp.join("libs/server/modules/mixmod");
        fs::create_dir_all(&api_dir).unwrap();
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("main.lua"),
            "-- main\nlocal M = {}\n---基础A\nfunction M.base_a() end\n---基础B\nfunction M.base_b() end\nreturn M\n",
        )
        .unwrap();
        fs::write(
            api_dir.join("mixmod.lua"),
            "-- mix\nlocal M = require('libs.server.modules.mixmod.main')\n---自有C\nfunction M.own_c() end\n---覆写A\nfunction M.base_a() end\nreturn M\n",
        )
        .unwrap();

        let mut api = parse_lua(&api_dir.join("mixmod.lua"), false, "").unwrap();
        assert_eq!(
            api.forward.as_deref(),
            Some("libs.server.modules.mixmod.main"),
            "混合式应识别顺链目标"
        );
        let tdoc = parse_lua(&mod_dir.join("main.lua"), false, "").unwrap();
        // 复刻 generate 的合并逻辑
        let own = std::mem::take(&mut api.members);
        let mut merged = tdoc.members;
        for m in own {
            if let Some(b) = merged.iter_mut().find(|b| b.table == m.table && b.name == m.name) {
                *b = m;
            } else {
                merged.push(m);
            }
        }
        let names: Vec<&str> = merged.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["base_a", "base_b", "own_c"]);
        // base_a 被 api 覆写（doc 第一行是 api 侧的「覆写A」）
        let base_a = merged.iter().find(|m| m.name == "base_a").unwrap();
        assert_eq!(base_a.doc.first().map(|s| s.as_str()), Some("覆写A"));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// 纯转发两行：local M = require + return M（无自有成员）
    #[test]
    fn plain_forward_two_lines() {
        let tmp = std::env::temp_dir().join("bgd_docgen_fwd_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("x.lua");
        fs::write(&f, "-- x\nlocal M = require('libs.a.b.main')\nreturn M\n").unwrap();
        let d = parse_lua(&f, false, "").unwrap();
        assert_eq!(d.forward.as_deref(), Some("libs.a.b.main"));
        assert!(d.members.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }
}
