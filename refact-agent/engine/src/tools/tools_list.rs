use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock as ARwLock;

use crate::call_validation::ChatMode;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::integrations::running_integrations::load_integrations;

use super::tools_description::{Tool, ToolGroup, ToolGroupCategory};

fn tool_available(
    tool: &Box<dyn Tool + Send>,
    ast_on: bool,
    vecdb_on: bool,
    is_there_a_thinking_model: bool,
    allow_knowledge: bool,
    allow_experimental: bool,
    platforms_enabled: &std::collections::HashSet<String>,
) -> bool {
    let dependencies = tool.tool_depends_on();
    if dependencies.contains(&"ast".to_string()) && !ast_on {
        return false;
    }
    if dependencies.contains(&"vecdb".to_string()) && !vecdb_on {
        return false;
    }
    if dependencies.contains(&"thinking".to_string()) && !is_there_a_thinking_model {
        return false;
    }
    if dependencies.contains(&"knowledge".to_string()) && !allow_knowledge {
        return false;
    }
    // Platform dependency checks
    if dependencies.contains(&"c2000".to_string()) && !platforms_enabled.contains("c2000") {
        return false;
    }
    if dependencies.contains(&"esp32".to_string()) && !platforms_enabled.contains("esp32") {
        return false;
    }
    if tool.tool_description().experimental && !allow_experimental {
        return false;
    }
    true
}

async fn tool_available_from_gcx(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> impl Fn(&Box<dyn Tool + Send>) -> bool {
    let (ast_on, vecdb_on, allow_experimental, active_group_id, platforms_enabled) = {
        let gcx_locked = gcx.read().await;
        // Check both dynamic VecDB and static VecDBs
        let dynamic_vecdb_on = gcx_locked.vec_db.lock().await.is_some();
        let static_vecdb_on = !gcx_locked.static_vec_db.lock().await.is_empty();
        let vecdb_on = dynamic_vecdb_on || static_vecdb_on;
        
        // Parse platform flags from command line
        let platforms: std::collections::HashSet<String> = gcx_locked.cmdline.platform
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        
        (gcx_locked.ast_service.is_some(), vecdb_on, 
         gcx_locked.cmdline.experimental, gcx_locked.active_group_id.clone(),
         platforms)
    };

    let (is_there_a_thinking_model, allow_knowledge) = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => {
            (caps.chat_models.get(&caps.defaults.chat_thinking_model).is_some(), active_group_id.is_some())
        },
        Err(_) => (false, false),
    };

    move |tool: &Box<dyn Tool + Send>| {
        tool_available(
            tool,
            ast_on,
            vecdb_on,
            is_there_a_thinking_model,
            allow_knowledge,
            allow_experimental,
            &platforms_enabled,
        )
    }
}

impl ToolGroup {
    pub async fn retain_available_tools(
        &mut self,
        gcx: Arc<ARwLock<GlobalContext>>,
    ) {
        let tool_available = tool_available_from_gcx(gcx.clone()).await;
        self.tools.retain(|tool| tool_available(tool));
    }
}

async fn get_builtin_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let config_dir = gcx.read().await.config_dir.clone();
    let config_path = config_dir.join("builtin_tools.yaml").to_string_lossy().to_string();

    let codebase_search_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_ast_definition::ToolAstDefinition{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_ast_reference::ToolAstReference{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_tree::ToolTree{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_cat::ToolCat{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_regex_search::ToolRegexSearch{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_search::ToolSearch{config_path: config_path.clone()}),
        // Box::new(crate::tools::tool_locate_search::ToolLocateSearch{config_path: config_path.clone()}),
    ];

    let codebase_change_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::file_edit::tool_create_textdoc::ToolCreateTextDoc{config_path: config_path.clone()}),
        Box::new(crate::tools::file_edit::tool_update_textdoc::ToolUpdateTextDoc{config_path: config_path.clone()}),
        Box::new(crate::tools::file_edit::tool_update_textdoc_regex::ToolUpdateTextDocRegex{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_rm::ToolRm{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_mv::ToolMv{config_path: config_path.clone()}),
    ];

    let web_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_web::ToolWeb{config_path: config_path.clone()}),
    ];

    let deep_analysis_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_strategic_planning::ToolStrategicPlanning{config_path: config_path.clone()}),
    ];

    let workflow_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_task_list::ToolTaskList{config_path: config_path.clone()}),
    ];

    let knowledge_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::tool_knowledge::ToolGetKnowledge{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_create_knowledge::ToolCreateKnowledge{config_path: config_path.clone()}),
        Box::new(crate::tools::tool_create_memory_bank::ToolCreateMemoryBank{config_path: config_path.clone()}),
    ];

    let c2000_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::c2000_tools::ToolC2000ProjectCreate{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000CodeEvaluator{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000Build{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000Flash{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000UartCapture{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000ConfigValidate{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000TargetDetect{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000ExampleList{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000SysconfigModify{config_path: config_path.clone()}),
        // Temporarily disabled - experimental LLM-based tool, can be enabled later
        // Box::new(crate::tools::c2000_tools::ToolC2000SysconfigModifyLlm{config_path: config_path.clone()}),
        Box::new(crate::tools::c2000_tools::ToolC2000ProjectspecModify{config_path: config_path.clone()}),
    ];

    let esp32_tools: Vec<Box<dyn Tool + Send>> = vec![
        Box::new(crate::tools::esp32_tools::ESP32Project{config_path: config_path.clone()}),
        Box::new(crate::tools::esp32_tools::ESP32Build{config_path: config_path.clone()}),
        Box::new(crate::tools::esp32_tools::ESP32Device{config_path: config_path.clone()}),
        Box::new(crate::tools::esp32_tools::ESP32ConfigTool{config_path: config_path.clone()}),
        Box::new(crate::tools::esp32_tools::ESP32Component{config_path: config_path.clone()}),
        // Disabled: ESP32Analyze uses subchat_single which causes UI state issues (flickering "uncalled tools" message)
        // The main agent can perform code review directly without this nested LLM call
        // Box::new(crate::tools::esp32_tools::ESP32Analyze{config_path: config_path.clone()}),
    ];

    let mut tool_groups = vec![
        ToolGroup {
            name: "Codebase Search".to_string(),
            description: "Codebase search tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: codebase_search_tools,
        },
        ToolGroup {
            name: "Codebase Change".to_string(),
            description: "Codebase modification tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: codebase_change_tools,
        },
        ToolGroup {
            name: "Web".to_string(),
            description: "Web tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: web_tools,
        },
        ToolGroup {
            name: "Strategic Planning".to_string(),
            description: "Strategic planning tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: deep_analysis_tools,
        },
        ToolGroup {
            name: "Workflow".to_string(),
            description: "Workflow and task management tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: workflow_tools,
        },
        ToolGroup {
            name: "Knowledge".to_string(),
            description: "Knowledge tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: knowledge_tools,
        },
        ToolGroup {
            name: "C2000 Development".to_string(),
            description: "TI C2000 microcontroller development tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: c2000_tools,
        },
        ToolGroup {
            name: "ESP32 Development".to_string(),
            description: "ESP32/ESP-IDF microcontroller development tools".to_string(),
            category: ToolGroupCategory::Builtin,
            tools: esp32_tools,
        },
    ];

    for tool_group in tool_groups.iter_mut() {
        tool_group.retain_available_tools(gcx.clone()).await;
    }

    tool_groups
}

async fn get_integration_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let mut integrations_group = ToolGroup {
        name: "Integrations".to_string(),
        description: "Integration tools".to_string(),
        category: ToolGroupCategory::Integration,
        tools: vec![],
    };

    let mut mcp_groups = HashMap::new();

    let (integrations_map, _yaml_errors) = load_integrations(gcx.clone(), &["**/*".to_string()]).await;
    for (name, integr) in integrations_map {
        for tool in integr.integr_tools(&name).await {
            let tool_desc = tool.tool_description();
            if tool_desc.name.starts_with("mcp") {
                let mcp_server_name = std::path::Path::new(&tool_desc.source.config_path)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown");

                if !mcp_groups.contains_key(mcp_server_name) {
                    mcp_groups.insert(
                        mcp_server_name.to_string(),
                        ToolGroup {
                            name: format!("MCP {}", mcp_server_name),
                            description: format!("MCP tools for {}", mcp_server_name),
                            category: ToolGroupCategory::MCP,
                            tools: vec![],
                        },
                    );
                }
                mcp_groups.entry(mcp_server_name.to_string())
                    .and_modify(|group| group.tools.push(tool));
            } else {
                integrations_group.tools.push(tool);
            }
        }
    }

    let mut tool_groups = vec![integrations_group];
    tool_groups.extend(mcp_groups.into_values());

    for tool_group in tool_groups.iter_mut() {
        tool_group.retain_available_tools(gcx.clone()).await;
    }

    tool_groups
}

pub async fn get_available_tool_groups(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<ToolGroup> {
    let mut tools_all = get_builtin_tools(gcx.clone()).await;
    tools_all.extend(
        get_integration_tools(gcx).await
    );

    tools_all
}

pub async fn get_available_tools(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<Box<dyn Tool + Send>> {
    get_available_tool_groups(gcx).await.into_iter().flat_map(|g| g.tools).collect()
}

pub async fn get_available_tools_by_chat_mode(
    gcx: Arc<ARwLock<GlobalContext>>,
    chat_mode: ChatMode,
) -> Vec<Box<dyn Tool + Send>> {
    if chat_mode == ChatMode::NO_TOOLS {
        return vec![];
    }

    let tools = get_available_tool_groups(gcx).await.into_iter()
        .flat_map(|g| g.tools)
        .filter(|tool| tool.config().unwrap_or_default().enabled);


    match chat_mode {
        ChatMode::NO_TOOLS => unreachable!("Condition handled at the beginning of the function."),
        ChatMode::EXPLORE => {
            tools.filter(|tool| !tool.tool_description().agentic).collect()
        },
        ChatMode::AGENT => {
            tools.collect()
        }
        ChatMode::CONFIGURE => {
            let blacklist = ["tree", "locate", "knowledge", "search"];
            tools.filter(|tool| !blacklist.contains(&tool.tool_description().name.as_str())).collect()
        },
        ChatMode::PROJECT_SUMMARY => {
            let whitelist = ["cat", "tree"];
            tools.filter(|tool| whitelist.contains(&tool.tool_description().name.as_str())).collect()
        },
    }
}

