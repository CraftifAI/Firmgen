//! Tree-sitter C/C++ extraction for GPIO, tasks, and app_main.

use tree_sitter::{Node, Parser};

use super::super::schema::{AnalysisFacts, GpioFact, TaskFact};
use super::static_extract;

pub fn extract_from_file(rel: &str, content: &str, facts: &mut AnalysisFacts) {
    if !rel.ends_with(".c")
        && !rel.ends_with(".cpp")
        && !rel.ends_with(".h")
        && !rel.ends_with(".hpp")
    {
        return;
    }

    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(content, None) else {
        return;
    };

    let root = tree.root_node();
    walk_node(&root, content, rel, facts);
}

fn walk_node(node: &Node, content: &str, rel: &str, facts: &mut AnalysisFacts) {
    match node.kind() {
        "function_definition" => check_app_main(node, content, rel, facts),
        "call_expression" => check_call(node, content, rel, facts),
        _ => {}
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_node(&cursor.node(), content, rel, facts);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn check_app_main(node: &Node, content: &str, rel: &str, facts: &mut AnalysisFacts) {
    let Some(decl) = node.child_by_field_name("declarator") else {
        return;
    };
    let name = function_name_from_declarator(&decl, content);
    if name.as_deref() == Some("app_main") {
        facts.has_app_main = true;
        facts.app_main_file = Some(rel.to_string());
    }
}

fn function_name_from_declarator(node: &Node, content: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(node_text(node, content)),
        "function_declarator" => {
            if let Some(inner) = node.child_by_field_name("declarator") {
                return function_name_from_declarator(&inner, content);
            }
            None
        }
        "pointer_declarator" | "reference_declarator" | "parenthesized_declarator" => {
            if let Some(inner) = node.child_by_field_name("declarator") {
                return function_name_from_declarator(&inner, content);
            }
            node.child(0)
                .map(|n| function_name_from_declarator(&n, content))
                .flatten()
        }
        _ => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if let Some(name) = function_name_from_declarator(&child, content) {
                        return Some(name);
                    }
                }
            }
            None
        }
    }
}

fn check_call(node: &Node, content: &str, rel: &str, facts: &mut AnalysisFacts) {
    let func_node = node
        .child_by_field_name("function")
        .or_else(|| node.child(0));
    let Some(func_node) = func_node else {
        return;
    };
    let func_name = callee_name(&func_node, content);
    let line = node.start_position().row as u32 + 1;

    match func_name.as_str() {
        "xTaskCreate" | "xTaskCreatePinnedToCore" => {
            parse_xtask_create(node, content, rel, line, facts);
        }
        "gpio_set_level" | "gpio_set_direction" | "gpio_reset_pin" => {
            if let Some(pin) = first_numeric_arg(node, content) {
                if should_skip_ast_gpio_pin(facts, pin) {
                    return;
                }
                push_gpio_ast(facts, rel, line, pin, "gpio_output", "GPIO Output");
            }
        }
        "gpio_get_level" => {
            if let Some(pin) = first_numeric_arg(node, content) {
                if should_skip_ast_gpio_pin(facts, pin) {
                    return;
                }
                push_gpio_ast(facts, rel, line, pin, "gpio_input", "GPIO Input");
            }
        }
        "gpio_config" => {
            if let Some(pin) = find_gpio_pin_near_call(node, content) {
                if should_skip_ast_gpio_pin(facts, pin) {
                    return;
                }
                push_gpio_ast(facts, rel, line, pin, "gpio_output", "GPIO");
            } else {
                facts.unresolved.push(serde_json::json!({
                    "kind": "gpio_indirect",
                    "file": rel,
                    "line": line,
                    "symbol": "gpio_config",
                    "hint": "gpio_config call — pin may be in struct initializer"
                }));
            }
        }
        _ => {}
    }
}

fn app_config_pin_manifest_present(facts: &AnalysisFacts) -> bool {
    facts
        .analyzed_files
        .iter()
        .any(|p| static_extract::is_app_config_path(p))
        || facts
            .gpio_facts
            .iter()
            .any(|g| static_extract::is_app_config_path(&g.file))
        || facts
            .network_facts
            .iter()
            .any(|n| static_extract::is_app_config_path(&n.file))
}

fn is_transport_binding_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    let parts: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.iter().any(|part| {
        matches!(
            *part,
            "sclk"
                | "mosi"
                | "miso"
                | "cs"
                | "dc"
                | "rst"
                | "reset"
                | "sda"
                | "scl"
                | "tx"
                | "rx"
                | "rts"
                | "cts"
                | "clk"
                | "clock"
                | "mclk"
                | "bclk"
                | "lrck"
                | "ws"
                | "din"
                | "dout"
        )
    }) {
        return true;
    }
    if lower.starts_with('d') && lower[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    lower.starts_with("adc") && (lower.contains("channel") || lower.contains("_ch"))
}

fn should_skip_ast_gpio_pin(facts: &AnalysisFacts, pin: u8) -> bool {
    if !app_config_pin_manifest_present(facts) {
        return false;
    }
    facts.network_facts.iter().any(|component| {
        let direct_binding = component
            .properties
            .get("pin_bindings")
            .and_then(|v| v.as_object())
            .map(|bindings| {
                bindings.iter().any(|(name, value)| {
                    is_transport_binding_name(name)
                        && value
                            .as_u64()
                            .and_then(|v| u8::try_from(v).ok())
                            .map(|v| v == pin)
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if direct_binding {
            return true;
        }
        component
            .properties
            .as_object()
            .map(|props| {
                props.iter().any(|(key, value)| {
                    if !key.ends_with("_pin") {
                        return false;
                    }
                    let binding = key.trim_end_matches("_pin");
                    is_transport_binding_name(binding)
                        && value
                            .as_u64()
                            .and_then(|v| u8::try_from(v).ok())
                            .map(|v| v == pin)
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    })
}

fn callee_name(node: &Node, content: &str) -> String {
    match node.kind() {
        "identifier" | "field_identifier" => node_text(node, content),
        "qualified_identifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                node_text(&name, content)
            } else {
                node_text(node, content)
                    .rsplit("::")
                    .next()
                    .unwrap_or("")
                    .to_string()
            }
        }
        _ => node_text(node, content),
    }
}

fn parse_xtask_create(node: &Node, content: &str, rel: &str, line: u32, facts: &mut AnalysisFacts) {
    let Some(args) = node.child_by_field_name("arguments") else {
        return;
    };
    let arg_nodes: Vec<Node> = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .filter(|n| n.kind() != "," && n.kind() != "(" && n.kind() != ")")
        .collect();
    if arg_nodes.len() < 4 {
        return;
    }
    let task_name =
        string_literal_value(&arg_nodes[1], content).unwrap_or_else(|| "task".to_string());
    let stack_size = numeric_value(&arg_nodes[2], content);
    let priority = numeric_value(&arg_nodes[3], content).map(|n| n as u8);

    let existing: Vec<String> = facts.task_facts.iter().map(|t| t.node_id.clone()).collect();
    let id = static_extract::unique_id_public(&existing, &task_name);
    if facts
        .task_facts
        .iter()
        .any(|t| t.task_name == task_name && t.file == rel)
    {
        return;
    }
    facts.task_facts.push(TaskFact {
        node_id: id,
        task_name,
        priority,
        stack_size,
        period_ms: None,
        file: rel.to_string(),
        line: Some(line),
    });
}

fn first_numeric_arg(node: &Node, content: &str) -> Option<u8> {
    let Some(args) = node.child_by_field_name("arguments") else {
        return None;
    };
    for i in 0..args.child_count() {
        if let Some(arg) = args.child(i) {
            if arg.kind() == "," || arg.kind() == "(" || arg.kind() == ")" {
                continue;
            }
            if let Some(n) = numeric_value(&arg, content) {
                return Some(n as u8);
            }
            break;
        }
    }
    None
}

fn find_gpio_pin_near_call(node: &Node, content: &str) -> Option<u8> {
    let call_text = node_text(node, content);
    for token in call_text.split(|c: char| !c.is_ascii_digit()) {
        if let Ok(pin) = token.parse::<u8>() {
            if pin <= 48 {
                return Some(pin);
            }
        }
    }
    if let Some(parent) = node.parent() {
        let parent_text = node_text(&parent, content);
        if parent_text.contains("GPIO_NUM_") {
            for part in parent_text.split("GPIO_NUM_") {
                if let Some(num) = part
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u8>()
                    .ok()
                {
                    return Some(num);
                }
            }
        }
    }
    None
}

fn string_literal_value(node: &Node, content: &str) -> Option<String> {
    let text = node_text(node, content);
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        Some(text[1..text.len() - 1].to_string())
    } else {
        None
    }
}

fn numeric_value(node: &Node, content: &str) -> Option<u32> {
    let raw = node_text(node, content);
    raw.trim().parse().ok()
}

fn node_text(node: &Node, content: &str) -> String {
    content.get(node.byte_range()).unwrap_or("").to_string()
}

fn push_gpio_ast(
    facts: &mut AnalysisFacts,
    rel: &str,
    line: u32,
    pin: u8,
    node_type: &str,
    label: &str,
) {
    if facts
        .gpio_facts
        .iter()
        .any(|g| g.pin == pin && g.file == rel)
    {
        return;
    }
    let existing: Vec<String> = facts.gpio_facts.iter().map(|g| g.node_id.clone()).collect();
    let id = static_extract::unique_id_public(&existing, &format!("{}_{}", node_type, pin));
    facts.gpio_facts.push(GpioFact {
        node_id: id,
        node_type: node_type.to_string(),
        label: format!("{} (GPIO {})", label, pin),
        pin,
        file: rel.to_string(),
        line: Some(line),
    });
}

/// Merge AST-extracted facts into the base facts; AST entries override regex on same file+pin/task.
pub fn merge_ast_into(base: &mut AnalysisFacts, ast: AnalysisFacts) {
    for g in ast.gpio_facts {
        if let Some(existing) = base
            .gpio_facts
            .iter_mut()
            .find(|x| x.pin == g.pin && x.file == g.file)
        {
            *existing = g;
        } else {
            base.gpio_facts.push(g);
        }
    }
    for t in ast.task_facts {
        if let Some(existing) = base
            .task_facts
            .iter_mut()
            .find(|x| x.task_name == t.task_name && x.file == t.file)
        {
            *existing = t;
        } else {
            base.task_facts.push(t);
        }
    }
    if ast.has_app_main {
        base.has_app_main = true;
        if ast.app_main_file.is_some() {
            base.app_main_file = ast.app_main_file;
        }
    }
    for u in ast.unresolved {
        if !base.unresolved.contains(&u) {
            base.unresolved.push(u);
        }
    }
}
