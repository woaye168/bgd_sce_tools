#![recursion_limit = "256"]
//! 可视化注入插件：扫描 API 模块生成星火编辑器可视化 JSON

use bgd_sce_tools_sdk::{BuildContext, BuildHook, Plugin, PluginError};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 可视化注入插件
pub struct VisualInjectorPlugin {
    _private: (),
}

impl VisualInjectorPlugin {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for VisualInjectorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for VisualInjectorPlugin {
    fn name(&self) -> &str {
        "visual-injector"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "扫描 API 模块生成星火编辑器可视化 JSON（函数/类/方法）"
    }
    fn author(&self) -> &str {
        "BGD"
    }
}

impl BuildHook for VisualInjectorPlugin {
    fn after_build(&self, ctx: &BuildContext) -> Result<(), PluginError> {
        ctx.log("[visual-injector] 开始扫描 API 生成可视化 JSON...");
        let count = scan_and_generate(ctx)?;
        ctx.log(&format!("[visual-injector] 生成可视化 JSON {count} 个"));
        Ok(())
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create() -> *mut dyn Plugin {
    Box::into_raw(Box::new(VisualInjectorPlugin::new()))
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create_build_hook() -> *mut dyn BuildHook {
    Box::into_raw(Box::new(VisualInjectorPlugin::new()))
}

// ---------------------------------------------------------------- 扫描与生成

/// 跳过的基础模块（不生成可视化 JSON）
const SKIP_MODULES: [&str; 10] = [
    "json", "log", "class", "co", "event", "promise", "exception", "deque", "event_deque", "init",
];

/// 引擎对象类型映射（Lua 类型 → Class:Xxx）
const ENGINE_CLASSES: [(&str, &str); 27] = [
    ("unit", "Unit"),
    ("player", "Player"),
    ("item", "Item"),
    ("skill", "Skill"),
    ("buff", "Buff"),
    ("timer", "Timer"),
    ("point", "Point"),
    ("target", "Target"),
    ("trigger", "Trigger"),
    ("region", "Region"),
    ("actor", "Actor"),
    ("slot", "Slot"),
    ("camera", "Camera"),
    ("vector", "Vector"),
    ("screenpos", "ScreenPos"),
    ("aisearcher", "AISearcher"),
    ("scorecommitter", "ScoreCommitter"),
    ("team", "Team"),
    ("mover", "Mover"),
    ("unitgroup", "UnitGroup"),
    ("line", "Line"),
    ("effectparam", "EffectParam"),
    ("effectparamshared", "EffectParamShared"),
    ("damageinstance", "DamageInstance"),
    ("healinstance", "HealInstance"),
    ("datacache", "DataCache"),
    ("ieventnotify", "IEventNotify"),
];

/// 扫描 .bgd/{libs,src}/{common,server,client}/api/*.lua，生成可视化 JSON
fn scan_and_generate(ctx: &BuildContext) -> Result<usize, PluginError> {
    let bgd_root = &ctx.bgd_root;
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    let mut count = 0;

    // 收集用户自定义类（本次扫描识别出的类，用于参数类型映射）
    let mut user_classes: HashMap<String, String> = HashMap::new();

    // 先扫描一遍收集类名
    for set in ["libs", "src"] {
        for side in ["common", "server", "client"] {
            let api_dir = bgd_root.join(set).join(side).join("api");
            if !api_dir.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&api_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                    continue;
                }
                let module = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if SKIP_MODULES.contains(&module) {
                    continue;
                }
                let content = fs::read_to_string(&path)?;
                for (class_name, _) in extract_classes(&content, module) {
                    user_classes.insert(class_name.to_lowercase(), class_name);
                }
            }
        }
    }

    // 再扫描生成 JSON
    for set in ["libs", "src"] {
        for side in ["common", "server", "client"] {
            let api_dir = bgd_root.join(set).join(side).join("api");
            if !api_dir.is_dir() {
                continue;
            }
            for entry in fs::read_dir(&api_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                    continue;
                }
                let module = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if SKIP_MODULES.contains(&module) {
                    continue;
                }
                let content = fs::read_to_string(&path)?;
                count += generate_module(ctx, project_root, side, module, &content, &user_classes)?;
            }
        }
    }

    Ok(count)
}

/// 提取类（---@class 注解）
fn extract_classes(content: &str, _module: &str) -> Vec<(String, bool)> {
    let mut classes = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("---@class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                classes.push((name, true));
            }
        }
    }
    classes
}

/// 生成单个模块的可视化 JSON
fn generate_module(
    ctx: &BuildContext,
    project_root: &Path,
    side: &str,
    module: &str,
    content: &str,
    user_classes: &HashMap<String, String>,
) -> Result<usize, PluginError> {
    let mut count = 0;
    let functions = parse_functions(content, module);
    let classes = parse_classes(content, module, user_classes);

    // 确定输出目录（common 双端，server 只服务端，client 只客户端）
    let sides: Vec<&str> = match side {
        "common" => vec!["server", "client"],
        "server" => vec!["server"],
        "client" => vec!["client"],
        _ => vec![],
    };

    for target_side in sides {
        let methods_dir = if target_side == "server" {
            project_root.join("src").join("data").join("methods")
        } else {
            project_root.join("ui").join("src").join("data").join("methods")
        };
        let classes_dir = if target_side == "server" {
            project_root.join("src").join("data").join("classes")
        } else {
            project_root.join("ui").join("src").join("data").join("classes")
        };

        fs::create_dir_all(&methods_dir)?;
        fs::create_dir_all(&classes_dir)?;

        // 生成函数 JSON
        for func in &functions {
            let json = function_to_json(func, module, target_side, user_classes);
            let file = methods_dir.join(format!("{}_{}.json", module, func.name));
            fs::write(&file, serde_json::to_string_pretty(&json)?)?;
            ctx.log(&format!("[visual-injector] 生成 {}: {}", target_side, file.display()));
            count += 1;
        }

        // 生成类 JSON
        for class in &classes {
            let json = class_to_json(class, target_side, user_classes);
            let file = classes_dir.join(format!("{}.json", class.name));
            fs::write(&file, serde_json::to_string_pretty(&json)?)?;
            ctx.log(&format!("[visual-injector] 生成 {}: {}", target_side, file.display()));
            count += 1;
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------- 函数解析

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    params: Vec<ParamInfo>,
    returns: Vec<String>,
    description: String,
    ui_text: String,
    is_static: bool,
    is_method: bool,
    is_event: bool,
    has_rest: bool,
}

#[derive(Debug)]
struct ParamInfo {
    name: String,
    lua_type: String,
    description: String,
}

/// 解析函数（function M.Xxx / function M:Xxx）
fn parse_functions(content: &str, _module: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();
    let mut doc_block = String::new();
    let mut in_doc = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("---") {
            in_doc = true;
            doc_block.push_str(line);
            doc_block.push('\n');
            continue;
        }

        if in_doc && !trimmed.is_empty() && !trimmed.starts_with("---") {
            // 文档块结束，检查是否是函数定义
            if let Some(func) = parse_function_line(trimmed, &doc_block) {
                if !func.is_event {
                    functions.push(func);
                }
            }
            in_doc = false;
            doc_block.clear();
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // 无文档的函数定义
        if let Some(func) = parse_function_line(trimmed, "") {
            if !func.is_event {
                functions.push(func);
            }
        }
    }

    functions
}

/// 解析单行函数定义
fn parse_function_line(line: &str, doc: &str) -> Option<FunctionInfo> {
    // function M.Xxx(a, b) 或 function M:Xxx(a, b)
    let line = line.strip_prefix("function ")?;
    let line = line.strip_prefix("M.").or_else(|| line.strip_prefix("M:"))?;
    let is_method = line.contains(':');
    let name_end = line.find('(')?;
    let name = line[..name_end].trim().to_string();
    if name.is_empty() || name.starts_with('_') {
        return None;
    }

    // 跳过事件注册函数（OnXxx 开头）
    let is_event = name.starts_with("On") && name.len() > 2 && name[2..].chars().next()?.is_uppercase();

    // 解析参数
    let params_str = &line[name_end + 1..line.rfind(')')?];
    let mut params = Vec::new();
    let mut has_rest = false;
    for p in params_str.split(',') {
        let p = p.trim();
        if p == "..." {
            has_rest = true;
            continue;
        }
        if p.is_empty() {
            continue;
        }
        params.push(ParamInfo {
            name: p.to_string(),
            lua_type: "any".to_string(),
            description: String::new(),
        });
    }

    // 从文档块提取信息
    let mut description = String::new();
    let mut ui_text = String::new();
    let mut returns = Vec::new();
    let mut is_static = false;

    for doc_line in doc.lines() {
        let doc_line = doc_line.trim();
        if let Some(rest) = doc_line.strip_prefix("---@param ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let param_name = parts[0];
                let lua_type = parts[1];
                let desc = parts[2..].join(" ");
                if let Some(param) = params.iter_mut().find(|p| p.name == param_name) {
                    param.lua_type = lua_type.to_string();
                    param.description = desc;
                }
            }
        } else if let Some(rest) = doc_line.strip_prefix("---@return ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if !parts.is_empty() {
                returns.push(parts[0].to_string());
            }
        } else if let Some(rest) = doc_line.strip_prefix("---@visual.uiText ") {
            ui_text = rest.to_string();
        } else if doc_line.starts_with("---@visual.static") {
            is_static = true;
        } else if !doc_line.starts_with("---@") {
            // 普通注释行作为描述
            if description.is_empty() {
                description = doc_line.trim_start_matches("---").trim().to_string();
            }
        }
    }

    Some(FunctionInfo {
        name,
        params,
        returns,
        description,
        ui_text,
        is_static,
        is_method,
        is_event,
        has_rest,
    })
}

// ---------------------------------------------------------------- 类解析

#[derive(Debug)]
struct ClassInfo {
    name: String,
    description: String,
    methods: Vec<FunctionInfo>,
    has_constructor: bool,
}

/// 解析类（---@class + M.New / prototype:____constructor / prototype:Method / M:Method）
fn parse_classes(content: &str, _module: &str, _user_classes: &HashMap<String, String>) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    let mut current_class: Option<ClassInfo> = None;
    let mut doc_block = String::new();
    let mut in_doc = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("---") {
            in_doc = true;
            doc_block.push_str(line);
            doc_block.push('\n');
            continue;
        }

        if in_doc && !trimmed.is_empty() && !trimmed.starts_with("---") {
            // 检查是否是类定义
            if trimmed.contains("M.New(") || trimmed.contains("prototype:____constructor(") {
                let class_name = extract_class_name_from_doc(&doc_block);
                if !class_name.is_empty() {
                    current_class = Some(ClassInfo {
                        name: class_name,
                        description: extract_description_from_doc(&doc_block),
                        methods: Vec::new(),
                        has_constructor: true,
                    });
                }
            }
            // 检查是否是方法定义
            else if trimmed.contains("prototype:") || trimmed.contains("M:") {
                if let Some(ref mut class) = current_class {
                    if let Some(func) = parse_function_line(trimmed, &doc_block) {
                        class.methods.push(func);
                    }
                }
            }
            in_doc = false;
            doc_block.clear();
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }
    }

    if let Some(class) = current_class {
        classes.push(class);
    }

    classes
}

fn extract_class_name_from_doc(doc: &str) -> String {
    for line in doc.lines() {
        if let Some(rest) = line.trim().strip_prefix("---@class ") {
            return rest.split_whitespace().next().unwrap_or("").to_string();
        }
    }
    String::new()
}

fn extract_description_from_doc(doc: &str) -> String {
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("---@") {
            return trimmed.trim_start_matches("---").trim().to_string();
        }
    }
    String::new()
}

// ---------------------------------------------------------------- JSON 生成

/// 函数 → FunctionDefine JSON
fn function_to_json(
    func: &FunctionInfo,
    module: &str,
    side: &str,
    user_classes: &HashMap<String, String>,
) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(10000000..99999999);

    let params: Vec<Value> = func.params.iter().map(|p| {
        param_to_json(p, side, user_classes)
    }).collect();

    let return_type = if func.returns.is_empty() {
        json!({
            "ElementName": "SimpleType",
            "name": "void",
            "displayName": "void",
            "flags": {},
            "tips": "",
            "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
            "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
        })
    } else {
        type_to_json(&func.returns[0], side, user_classes)
    };

    let mut flags = json!({
        "enableDisplayName": true,
        "isDeclare": true,
        "noSelf": true,
        "pop": true
    });
    if func.is_static {
        flags["isStatic"] = json!(true);
    }

    let mut result = json!({
        "ElementName": "FunctionDefine",
        "name": format!("{}_{}", module, func.name),
        "packageName": "p_eelc",
        "id": format!("FunctionDefine:{}_{}:{}", module, func.name, id),
        "displayName": func.description,
        "description": func.description,
        "keyword": format!("{} {}", module, func.name),
        "tips": func.description,
        "uiText": func.ui_text,
        "rankOrder": 2,
        "v2_version": 0.9,
        "s_or_c": side,
        "flags": flags,
        "parameters": { "__TYPE": "Array", "contents": params },
        "realReturnType": return_type,
        "actions": { "__TYPE": "Array", "contents": {} },
        "subsections": { "__TYPE": "Array", "contents": {} },
        "typeParameters": { "__TYPE": "Array", "contents": {} },
        "typeParametersExtends": { "__TYPE": "Map", "contents": {} },
        "variables": { "__TYPE": "Array", "contents": {} },
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 },
        "defaultResetParameterCount": 0
    });

    if func.has_rest {
        result["restParameter"] = json!({
            "ElementName": "Parameter",
            "name": "args",
            "displayName": "args",
            "realType": {
                "ElementName": "InstanceType",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": "Class:Array",
                        "packageName": "__common__",
                        "s_or_c": "common"
                    }
                },
                "typeArgs": {
                    "__TYPE": "Array",
                    "contents": [{
                        "ElementName": "SimpleType",
                        "name": "any",
                        "displayName": "任意",
                        "flags": {},
                        "tips": "",
                        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                    }]
                }
            }
        });
    }

    result
}

/// 参数 → Parameter JSON
fn param_to_json(param: &ParamInfo, side: &str, user_classes: &HashMap<String, String>) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(1000000000..9999999999u64);

    json!({
        "ElementName": "Parameter",
        "name": param.name,
        "displayName": param.description,
        "id": format!("Variable:{}:{}", param.name, id),
        "keyword": "",
        "label": "默认",
        "packageName": "p_eelc",
        "tips": param.description,
        "s_or_c": side,
        "flags": {},
        "realType": type_to_json(&param.lua_type, side, user_classes),
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
    })
}

/// Lua 类型 → 类型 JSON
fn type_to_json(lua_type: &str, side: &str, user_classes: &HashMap<String, String>) -> Value {
    fn simple_type(name: &str, display: &str) -> Value {
        json!({
            "ElementName": "SimpleType",
            "name": name,
            "displayName": display,
            "flags": {},
            "tips": "",
            "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
            "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
        })
    }

    match lua_type {
        "number" => simple_type("number", "数值"),
        "string" => simple_type("string", "字符串"),
        "boolean" => simple_type("boolean", "布尔"),
        "table" => simple_type("table", "表格"),
        "void" => simple_type("void", "void"),
        t if t.ends_with("[]") => {
            let elem_type = &t[..t.len() - 2];
            json!({
                "ElementName": "InstanceType",
                "displayName": "",
                "flags": {},
                "tips": "",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": "Class:Array",
                        "packageName": "__common__",
                        "s_or_c": "common"
                    }
                },
                "typeArgs": {
                    "__TYPE": "Array",
                    "contents": [type_to_json(elem_type, side, user_classes)]
                },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            })
        }
        t if t.starts_with("fun(") => {
            json!({
                "ElementName": "FuncType",
                "displayName": "",
                "flags": {},
                "isArrowFunc": false,
                "noSelf": true,
                "optionalParams": { "__TYPE": "Array", "contents": {} },
                "params": { "__TYPE": "Array", "contents": {} },
                "returnType": {
                    "ElementName": "SimpleType",
                    "name": "void",
                    "displayName": "void",
                    "flags": {},
                    "tips": "",
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "tips": "",
                "typeParams": { "__TYPE": "Array", "contents": {} },
                "typeParamsExtends": { "__TYPE": "Map", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            })
        }
        t => {
            // 引擎对象或用户自定义类
            let lower = t.to_lowercase();
            if let Some((_, class_name)) = ENGINE_CLASSES.iter().find(|(k, _)| *k == lower) {
                let pkg = if side == "server" { "__server__" } else { "__client__" };
                json!({
                    "ElementName": "InstanceType",
                    "displayName": "",
                    "flags": {},
                    "tips": "",
                    "source": {
                        "ElementName": "Source",
                        "targetUninit": {
                            "id": format!("Class:{}", class_name),
                            "packageName": pkg,
                            "s_or_c": side
                        }
                    },
                    "typeArgs": { "__TYPE": "Array", "contents": {} },
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                })
            } else if let Some(class_name) = user_classes.get(&lower) {
                json!({
                    "ElementName": "InstanceType",
                    "displayName": "",
                    "flags": {},
                    "tips": "",
                    "source": {
                        "ElementName": "Source",
                        "targetUninit": {
                            "id": format!("Class:{}", class_name),
                            "packageName": "p_eelc",
                            "s_or_c": side
                        }
                    },
                    "typeArgs": { "__TYPE": "Array", "contents": {} },
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                })
            } else {
                // 未知类型 → table 近似
                simple_type("table", "表格")
            }
        }
    }
}

/// 类 → Class JSON
fn class_to_json(class: &ClassInfo, side: &str, user_classes: &HashMap<String, String>) -> Value {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen_range(1000000000..9999999999u64);

    let constructor = if class.has_constructor {
        json!({
            "ElementName": "ConstructorDefine",
            "name": "____constructor",
            "id": format!("Class:{}:{}, ConstructorDefine:____constructor", class.name, id),
            "packageName": "p_eelc",
            "s_or_c": side,
            "flags": { "isDeclare": true },
            "parameters": { "__TYPE": "Array", "contents": [] },
            "realReturnType": {
                "ElementName": "InstanceType",
                "source": {
                    "ElementName": "Source",
                    "targetUninit": {
                        "id": format!("Class:{}:{}", class.name, id),
                        "packageName": "p_eelc",
                        "s_or_c": side
                    }
                },
                "typeArgs": { "__TYPE": "Array", "contents": {} },
                "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
            }
        })
    } else {
        Value::Null
    };

    let methods: Vec<Value> = class.methods.iter().map(|m| {
        let _method_id: u64 = rng.gen_range(10000000..99999999);
        let mut flags = json!({ "isDeclare": true });
        if m.is_static {
            flags["isStatic"] = json!(true);
            flags["noSelf"] = json!(true);
        }
        json!({
            "ElementName": "MethodDefine",
            "name": m.name,
            "id": format!("Class:{}:{}, MethodDefine:{}", class.name, id, m.name),
            "packageName": "p_eelc",
            "s_or_c": side,
            "flags": flags,
            "parameters": { "__TYPE": "Array", "contents": m.params.iter().map(|p| param_to_json(p, side, user_classes)).collect::<Vec<_>>() },
            "realReturnType": if m.returns.is_empty() {
                json!({
                    "ElementName": "SimpleType",
                    "name": "void",
                    "displayName": "void",
                    "flags": {},
                    "tips": "",
                    "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
                    "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
                })
            } else {
                type_to_json(&m.returns[0], side, user_classes)
            }
        })
    }).collect();

    json!({
        "ElementName": "Class",
        "name": class.name,
        "id": format!("Class:{}:{}", class.name, id),
        "packageName": "p_eelc",
        "displayName": class.description,
        "description": class.description,
        "keyword": class.name,
        "tips": class.description,
        "s_or_c": side,
        "flags": {},
        "_constructor": constructor,
        "_methods": { "__TYPE": "Array", "contents": methods },
        "folder": {
            "ElementName": "Folder",
            "name": class.name,
            "id": format!("Folder:{}:{}", class.name, id),
            "packageName": "p_eelc",
            "s_or_c": side
        },
        "thisVariable": {
            "ElementName": "Variable",
            "name": "this",
            "id": format!("Variable:this:{}", id),
            "packageName": "p_eelc",
            "s_or_c": side
        },
        "staticWarningMsgs": { "__TYPE": "Array", "contents": {} },
        "breakPointInfo": { "disabled": false, "hasBreakPoint": false, "type": 2 }
    })
}
