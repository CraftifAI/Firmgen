pub mod config;
pub mod project_create;
pub mod code_evaluator;
pub mod build;
pub mod flash;
pub mod uart_capture;
pub mod config_validate;
pub mod target_detect;
pub mod example_list;
pub mod sysconfig_modify;
pub mod sysconfig_modify_llm;
pub mod projectspec_modify;

// Re-export the main tools for easy access
pub use config::C2000Config;
pub use project_create::ToolC2000ProjectCreate;
pub use code_evaluator::ToolC2000CodeEvaluator;
pub use build::ToolC2000Build;
pub use flash::ToolC2000Flash;
pub use uart_capture::ToolC2000UartCapture;
pub use config_validate::ToolC2000ConfigValidate;
pub use target_detect::ToolC2000TargetDetect;
pub use example_list::ToolC2000ExampleList;
pub use sysconfig_modify::ToolC2000SysconfigModify;
pub use sysconfig_modify_llm::ToolC2000SysconfigModifyLlm;
pub use projectspec_modify::ToolC2000ProjectspecModify;

