use super::types::{
    ExecutionSemantics, NodeTypeDefResponse, PortDefResponse, PortDirection, PropertyFieldDef,
};
use lazy_static::lazy_static;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct PortDef {
    pub name: &'static str,
    pub direction: PortDirection,
    pub datatype: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct NodeTypeDef {
    pub node_type: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub color: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
    pub ports: Vec<PortDef>,
    pub properties: Vec<PropertyFieldDef>,
    pub execution_semantics: ExecutionSemantics,
}

fn prop(
    key: &str,
    label: &str,
    field_type: &str,
    default: Option<serde_json::Value>,
    options: Option<Vec<&str>>,
    description: Option<&str>,
) -> PropertyFieldDef {
    PropertyFieldDef {
        key: key.to_string(),
        label: label.to_string(),
        field_type: field_type.to_string(),
        default,
        options: options.map(|v| v.into_iter().map(String::from).collect()),
        min: None,
        max: None,
        read_only: false,
        description: description.map(String::from),
    }
}

fn prop_num(key: &str, label: &str, default: f64, min: f64, max: f64) -> PropertyFieldDef {
    PropertyFieldDef {
        key: key.to_string(),
        label: label.to_string(),
        field_type: "number".to_string(),
        default: Some(json!(default)),
        options: None,
        min: Some(min),
        max: Some(max),
        read_only: false,
        description: None,
    }
}

fn prop_int(key: &str, label: &str, default: i64, min: f64, max: f64) -> PropertyFieldDef {
    PropertyFieldDef {
        key: key.to_string(),
        label: label.to_string(),
        field_type: "integer".to_string(),
        default: Some(json!(default)),
        options: None,
        min: Some(min),
        max: Some(max),
        read_only: false,
        description: None,
    }
}

fn prop_gpio(key: &str, label: &str) -> PropertyFieldDef {
    PropertyFieldDef {
        key: key.to_string(),
        label: label.to_string(),
        field_type: "gpio".to_string(),
        default: None,
        options: None,
        min: Some(0.0),
        max: Some(48.0),
        read_only: false,
        description: None,
    }
}

fn exec(phase: &str, trigger: Option<&str>, description: &str) -> ExecutionSemantics {
    ExecutionSemantics {
        phase: phase.to_string(),
        trigger: trigger.map(String::from),
        description: description.to_string(),
    }
}

static PORT_EXEC_IN: PortDef = PortDef {
    name: "exec_in",
    direction: PortDirection::Input,
    datatype: "execution",
    required: false,
};
static PORT_DATA_IN: PortDef = PortDef {
    name: "data_in",
    direction: PortDirection::Input,
    datatype: "signal",
    required: false,
};
static PORT_DATA_OUT: PortDef = PortDef {
    name: "data_out",
    direction: PortDirection::Output,
    datatype: "signal",
    required: false,
};
static PORT_EVENT_IN: PortDef = PortDef {
    name: "event_in",
    direction: PortDirection::Input,
    datatype: "event",
    required: false,
};
static PORT_EVENT_OUT: PortDef = PortDef {
    name: "event_out",
    direction: PortDirection::Output,
    datatype: "event",
    required: false,
};

static SYSTEM_INIT_PORTS: [PortDef; 1] = [PortDef {
    name: "boot_out",
    direction: PortDirection::Output,
    datatype: "execution",
    required: true,
}];

static GPIO_IN_PORTS: &[PortDef] = &[
    PortDef {
        name: "exec_in",
        direction: PortDirection::Input,
        datatype: "execution",
        required: false,
    },
    PortDef {
        name: "gpio_out",
        direction: PortDirection::Output,
        datatype: "gpio_level",
        required: true,
    },
];

static GPIO_OUT_PORTS: &[PortDef] = &[
    PortDef {
        name: "exec_in",
        direction: PortDirection::Input,
        datatype: "execution",
        required: true,
    },
    PortDef {
        name: "gpio_in",
        direction: PortDirection::Input,
        datatype: "gpio_level",
        required: false,
    },
];

static RTOS_TASK_PORTS: &[PortDef] = &[
    PortDef {
        name: "exec_in",
        direction: PortDirection::Input,
        datatype: "execution",
        required: false,
    },
    PortDef {
        name: "trigger_in",
        direction: PortDirection::Input,
        datatype: "event",
        required: false,
    },
    PortDef {
        name: "exec_out",
        direction: PortDirection::Output,
        datatype: "execution",
        required: false,
    },
    PortDef {
        name: "data_out",
        direction: PortDirection::Output,
        datatype: "signal",
        required: false,
    },
];

static WIFI_PORTS: &[PortDef] = &[
    PortDef {
        name: "exec_in",
        direction: PortDirection::Input,
        datatype: "execution",
        required: true,
    },
    PortDef {
        name: "network_out",
        direction: PortDirection::Output,
        datatype: "network",
        required: true,
    },
];

static MQTT_PORTS: &[PortDef] = &[
    PortDef {
        name: "network_in",
        direction: PortDirection::Input,
        datatype: "network",
        required: true,
    },
    PortDef {
        name: "data_in",
        direction: PortDirection::Input,
        datatype: "payload",
        required: false,
    },
    PortDef {
        name: "publish_out",
        direction: PortDirection::Output,
        datatype: "mqtt_message",
        required: false,
    },
];

static ML_PORTS: &[PortDef] = &[
    PortDef {
        name: "tensor_in",
        direction: PortDirection::Input,
        datatype: "tensor",
        required: true,
    },
    PortDef {
        name: "tensor_out",
        direction: PortDirection::Output,
        datatype: "tensor",
        required: true,
    },
];

macro_rules! node_def {
    ($ty:expr, $cat:expr, $label:expr, $color:expr, $icon:expr, $desc:expr, $ports:expr, $props:expr, $exec:expr) => {
        NodeTypeDef {
            node_type: $ty,
            category: $cat,
            label: $label,
            color: $color,
            icon: $icon,
            description: $desc,
            ports: $ports,
            properties: $props,
            execution_semantics: $exec,
        }
    };
}

lazy_static! {
    static ref NODE_REGISTRY: Vec<NodeTypeDef> = vec![
        node_def!(
            "system_init",
            "system",
            "System Init",
            "#6366F1",
            "boot",
            "Boot-time system initialization and hardware setup.",
            SYSTEM_INIT_PORTS.to_vec(),
            vec![
                prop(
                    "target",
                    "IDF Target",
                    "enum",
                    Some(json!("esp32s3")),
                    Some(vec![
                        "esp32", "esp32s2", "esp32s3", "esp32c3", "esp32c6", "esp32h2"
                    ]),
                    None
                ),
                prop(
                    "log_level",
                    "Log Level",
                    "enum",
                    Some(json!("info")),
                    Some(vec!["none", "error", "warn", "info", "debug", "verbose"]),
                    None
                ),
            ],
            exec("boot", None, "Runs once at startup before any tasks.")
        ),
        node_def!(
            "gpio_input",
            "gpio",
            "GPIO Input",
            "#3B82F6",
            "gpio_in",
            "Digital GPIO input pin reader.",
            GPIO_IN_PORTS.to_vec(),
            vec![
                prop_gpio("pin", "GPIO Pin"),
                prop(
                    "pull_mode",
                    "Pull Mode",
                    "enum",
                    Some(json!("none")),
                    Some(vec!["none", "up", "down"]),
                    None
                ),
                prop(
                    "invert",
                    "Invert Logic",
                    "boolean",
                    Some(json!(false)),
                    None,
                    None
                ),
            ],
            exec(
                "runtime",
                Some("poll"),
                "Samples GPIO level on poll or interrupt."
            )
        ),
        node_def!(
            "gpio_output",
            "gpio",
            "GPIO Output",
            "#10B981",
            "gpio_out",
            "Digital GPIO output driver.",
            GPIO_OUT_PORTS.to_vec(),
            vec![
                prop_gpio("pin", "GPIO Pin"),
                prop(
                    "active_level",
                    "Active Level",
                    "enum",
                    Some(json!("high")),
                    Some(vec!["high", "low"]),
                    None
                ),
                prop(
                    "initial_state",
                    "Initial State",
                    "boolean",
                    Some(json!(false)),
                    None,
                    None
                ),
            ],
            exec(
                "runtime",
                Some("event"),
                "Drives GPIO on execution or signal input."
            )
        ),
        node_def!(
            "adc_reader",
            "analog",
            "ADC Reader",
            "#8B5CF6",
            "adc",
            "Analog-to-digital converter sampling.",
            vec![PORT_EXEC_IN.clone(), PORT_DATA_OUT.clone()],
            vec![
                prop_gpio("pin", "ADC GPIO"),
                prop(
                    "attenuation",
                    "Attenuation",
                    "enum",
                    Some(json!("11db")),
                    Some(vec!["0db", "2.5db", "6db", "11db"]),
                    None
                ),
                prop_num("sample_rate_hz", "Sample Rate (Hz)", 100.0, 1.0, 100000.0),
            ],
            exec("runtime", Some("poll"), "Periodic ADC sampling.")
        ),
        node_def!(
            "pwm_output",
            "analog",
            "PWM Output",
            "#EC4899",
            "pwm",
            "LEDC PWM output channel.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop_gpio("pin", "GPIO Pin"),
                prop_int("frequency_hz", "Frequency (Hz)", 5000, 1.0, 40000000.0),
                prop_int("resolution_bits", "Resolution (bits)", 13, 1.0, 20.0),
            ],
            exec("runtime", Some("timer"), "PWM waveform generation.")
        ),
        node_def!(
            "uart_device",
            "communication",
            "UART Device",
            "#F59E0B",
            "uart",
            "UART serial communication.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop_int("port", "UART Port", 1, 0.0, 2.0),
                prop_int("baud_rate", "Baud Rate", 115200, 300.0, 5000000.0),
                prop_int("tx_pin", "TX GPIO", 17, 0.0, 48.0),
                prop_int("rx_pin", "RX GPIO", 16, 0.0, 48.0),
            ],
            exec("runtime", Some("stream"), "Async UART stream I/O.")
        ),
        node_def!(
            "i2c_device",
            "communication",
            "I2C Device",
            "#14B8A6",
            "i2c",
            "I2C bus peripheral.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop_int("sda_pin", "SDA GPIO", 21, 0.0, 48.0),
                prop_int("scl_pin", "SCL GPIO", 22, 0.0, 48.0),
                prop(
                    "address",
                    "I2C Address",
                    "string",
                    Some(json!("0x68")),
                    None,
                    None
                ),
                prop_int("clock_hz", "Clock (Hz)", 100000, 10000.0, 1000000.0),
            ],
            exec("runtime", Some("poll"), "I2C register read/write.")
        ),
        node_def!(
            "spi_device",
            "communication",
            "SPI Device",
            "#06B6D4",
            "spi",
            "SPI bus peripheral.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop(
                    "host",
                    "SPI Host",
                    "enum",
                    Some(json!("SPI2_HOST")),
                    Some(vec!["SPI2_HOST", "SPI3_HOST"]),
                    None
                ),
                prop_int("mosi_pin", "MOSI GPIO", 23, 0.0, 48.0),
                prop_int("miso_pin", "MISO GPIO", 19, 0.0, 48.0),
                prop_int("sclk_pin", "SCLK GPIO", 18, 0.0, 48.0),
                prop_int("cs_pin", "CS GPIO", 5, 0.0, 48.0),
            ],
            exec("runtime", Some("dma"), "SPI DMA transfers.")
        ),
        node_def!(
            "wifi_manager",
            "network",
            "WiFi Manager",
            "#0EA5E9",
            "wifi",
            "WiFi station/AP manager.",
            WIFI_PORTS.to_vec(),
            vec![
                prop("ssid", "SSID", "string", None, None, None),
                prop("password", "Password", "string", None, None, None),
                prop(
                    "mode",
                    "Mode",
                    "enum",
                    Some(json!("station")),
                    Some(vec!["station", "ap", "sta_ap"]),
                    None
                ),
            ],
            exec("init", Some("event"), "Connects to WiFi network.")
        ),
        node_def!(
            "ble_manager",
            "network",
            "BLE Manager",
            "#38BDF8",
            "ble",
            "Bluetooth Low Energy stack.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop(
                    "device_name",
                    "Device Name",
                    "string",
                    Some(json!("ESP32-BLE")),
                    None,
                    None
                ),
                prop(
                    "role",
                    "Role",
                    "enum",
                    Some(json!("peripheral")),
                    Some(vec!["central", "peripheral"]),
                    None
                ),
            ],
            exec("runtime", Some("event"), "BLE GATT services.")
        ),
        node_def!(
            "mqtt_client",
            "network",
            "MQTT Client",
            "#22C55E",
            "mqtt",
            "MQTT publish/subscribe client.",
            MQTT_PORTS.to_vec(),
            vec![
                prop(
                    "broker_url",
                    "Broker URL",
                    "url",
                    Some(json!("mqtt://broker.hivemq.com:1883")),
                    None,
                    None
                ),
                prop(
                    "topic",
                    "Topic",
                    "string",
                    Some(json!("esp32/sensor")),
                    None,
                    None
                ),
                prop("client_id", "Client ID", "string", None, None, None),
                prop_int("qos", "QoS", 0, 0.0, 2.0),
            ],
            exec("runtime", Some("event"), "MQTT message publish/subscribe.")
        ),
        node_def!(
            "http_client",
            "network",
            "HTTP Client",
            "#84CC16",
            "http",
            "HTTP/HTTPS REST client.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop("url", "URL", "url", None, None, None),
                prop(
                    "method",
                    "Method",
                    "enum",
                    Some(json!("GET")),
                    Some(vec!["GET", "POST", "PUT", "DELETE"]),
                    None
                ),
                prop_int("timeout_ms", "Timeout (ms)", 5000, 100.0, 60000.0),
            ],
            exec("runtime", Some("event"), "HTTP request/response.")
        ),
        node_def!(
            "websocket_client",
            "network",
            "WebSocket Client",
            "#A3E635",
            "websocket",
            "WebSocket streaming client.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop("url", "WebSocket URL", "url", None, None, None),
                prop_int("reconnect_ms", "Reconnect (ms)", 3000, 500.0, 60000.0),
            ],
            exec("runtime", Some("stream"), "Bidirectional WebSocket stream.")
        ),
        node_def!(
            "rtos_task",
            "rtos",
            "RTOS Task",
            "#F97316",
            "task",
            "FreeRTOS task wrapper.",
            RTOS_TASK_PORTS.to_vec(),
            vec![
                prop(
                    "task_name",
                    "Task Name",
                    "string",
                    Some(json!("worker_task")),
                    None,
                    None
                ),
                prop_int("priority", "Priority", 5, 0.0, 25.0),
                prop_int("stack_size", "Stack Size (bytes)", 4096, 512.0, 65536.0),
                prop_int("core", "Core Affinity", 0, 0.0, 1.0),
                prop_num("period_ms", "Period (ms)", 100.0, 0.0, 60000.0),
            ],
            exec("runtime", Some("timer"), "FreeRTOS task execution loop.")
        ),
        node_def!(
            "event_handler",
            "rtos",
            "Event Handler",
            "#FB923C",
            "event",
            "ESP event loop handler.",
            vec![
                PORT_EVENT_IN.clone(),
                PORT_EVENT_OUT.clone(),
                PortDef {
                    name: "exec_out",
                    direction: PortDirection::Output,
                    datatype: "execution",
                    required: false,
                },
            ],
            vec![
                prop(
                    "event_base",
                    "Event Base",
                    "string",
                    Some(json!("ESP_EVENT_ANY_BASE")),
                    None,
                    None
                ),
                prop_int("event_id", "Event ID", -1, -1.0, 65535.0),
            ],
            exec("runtime", Some("event"), "Dispatches ESP-IDF events.")
        ),
        node_def!(
            "timer_node",
            "rtos",
            "Timer",
            "#FBBF24",
            "timer",
            "ESP timer / FreeRTOS software timer.",
            vec![PORT_EXEC_IN.clone(), PORT_EVENT_OUT.clone()],
            vec![
                prop(
                    "timer_name",
                    "Timer Name",
                    "string",
                    Some(json!("periodic_timer")),
                    None,
                    None
                ),
                prop_num("period_ms", "Period (ms)", 1000.0, 1.0, 86400000.0),
                prop(
                    "auto_reload",
                    "Auto Reload",
                    "boolean",
                    Some(json!(true)),
                    None,
                    None
                ),
            ],
            exec("runtime", Some("timer"), "Periodic timer callback.")
        ),
        node_def!(
            "sensor_input",
            "sensors",
            "Sensor Input",
            "#A855F7",
            "sensor",
            "Generic sensor data source.",
            vec![
                PortDef {
                    name: "exec_in",
                    direction: PortDirection::Input,
                    datatype: "execution",
                    required: false,
                },
                PORT_DATA_OUT.clone(),
            ],
            vec![
                prop(
                    "sensor_type",
                    "Sensor Type",
                    "enum",
                    Some(json!("pir")),
                    Some(vec![
                        "pir",
                        "temperature",
                        "humidity",
                        "pressure",
                        "imu",
                        "custom"
                    ]),
                    None
                ),
                prop_gpio("pin", "GPIO Pin"),
                prop("driver", "Driver", "string", None, None, None),
            ],
            exec("runtime", Some("interrupt"), "Sensor interrupt or poll.")
        ),
        node_def!(
            "signal_processing",
            "pipeline",
            "Signal Processing",
            "#7C3AED",
            "signal",
            "Digital signal processing block.",
            vec![PORT_DATA_IN.clone(), PORT_DATA_OUT.clone()],
            vec![
                prop(
                    "algorithm",
                    "Algorithm",
                    "enum",
                    Some(json!("filter")),
                    Some(vec!["filter", "threshold", "normalize", "custom"]),
                    None
                ),
                prop_num("threshold", "Threshold", 0.5, 0.0, 1.0),
            ],
            exec("runtime", Some("stream"), "Processes signal data stream.")
        ),
        node_def!(
            "edge_ml_inference",
            "pipeline",
            "Edge ML Inference",
            "#D97706",
            "ml",
            "On-device ML model inference.",
            ML_PORTS.to_vec(),
            vec![
                prop("model_path", "Model Path", "path", None, None, None),
                prop(
                    "model_type",
                    "Model Type",
                    "enum",
                    Some(json!("tflite")),
                    Some(vec!["tflite", "espdl", "onnx"]),
                    None
                ),
                prop_int("input_width", "Input Width", 96, 1.0, 4096.0),
                prop_int("input_height", "Input Height", 96, 1.0, 4096.0),
            ],
            exec("runtime", Some("stream"), "Tensor inference pipeline.")
        ),
        node_def!(
            "camera_capture",
            "media",
            "Camera Capture",
            "#EF4444",
            "camera",
            "Camera frame capture.",
            vec![PORT_EXEC_IN.clone(), PORT_DATA_OUT.clone()],
            vec![
                prop(
                    "interface",
                    "Interface",
                    "enum",
                    Some(json!("dvp")),
                    Some(vec!["dvp", "mipi", "usb"]),
                    None
                ),
                prop_int("frame_width", "Frame Width", 640, 80.0, 4096.0),
                prop_int("frame_height", "Frame Height", 480, 80.0, 4096.0),
                prop_int("fps", "FPS", 15, 1.0, 120.0),
            ],
            exec("runtime", Some("dma"), "Camera DMA frame capture.")
        ),
        node_def!(
            "display_output",
            "media",
            "Display Output",
            "#F43F5E",
            "display",
            "Display frame output.",
            vec![PORT_DATA_IN.clone(), PORT_EXEC_IN.clone()],
            vec![
                prop(
                    "interface",
                    "Interface",
                    "enum",
                    Some(json!("spi")),
                    Some(vec!["spi", "i2c", "rgb", "mipi"]),
                    None
                ),
                prop_int("width", "Width", 240, 80.0, 4096.0),
                prop_int("height", "Height", 320, 80.0, 4096.0),
            ],
            exec("runtime", Some("stream"), "Display frame rendering.")
        ),
        node_def!(
            "storage_manager",
            "system",
            "Storage Manager",
            "#64748B",
            "storage",
            "Flash/NVS/SPIFFS storage manager.",
            vec![
                PORT_EXEC_IN.clone(),
                PORT_DATA_IN.clone(),
                PORT_DATA_OUT.clone()
            ],
            vec![
                prop(
                    "backend",
                    "Backend",
                    "enum",
                    Some(json!("nvs")),
                    Some(vec!["nvs", "spiffs", "fatfs", "littlefs"]),
                    None
                ),
                prop(
                    "namespace",
                    "Namespace",
                    "string",
                    Some(json!("app")),
                    None,
                    None
                ),
            ],
            exec("init", None, "Initializes storage subsystem.")
        ),
        node_def!(
            "ota_update",
            "system",
            "OTA Update",
            "#475569",
            "ota",
            "Over-the-air firmware update.",
            vec![PORT_EXEC_IN.clone(), PORT_DATA_IN.clone()],
            vec![
                prop(
                    "partition_label",
                    "Partition",
                    "string",
                    Some(json!("ota_1")),
                    None,
                    None
                ),
                prop("url", "Firmware URL", "url", None, None, None),
            ],
            exec("runtime", Some("event"), "OTA download and flash.")
        ),
        node_def!(
            "diagnostics",
            "system",
            "Diagnostics",
            "#94A3B8",
            "diagnostics",
            "System diagnostics and health checks.",
            vec![PORT_EXEC_IN.clone(), PORT_DATA_OUT.clone()],
            vec![
                prop(
                    "check_heap",
                    "Check Heap",
                    "boolean",
                    Some(json!(true)),
                    None,
                    None
                ),
                prop(
                    "check_stack",
                    "Check Stack",
                    "boolean",
                    Some(json!(true)),
                    None,
                    None
                ),
            ],
            exec("runtime", Some("timer"), "Periodic health monitoring.")
        ),
        node_def!(
            "logger",
            "system",
            "Logger",
            "#CBD5E1",
            "logger",
            "Structured logging output.",
            vec![PORT_DATA_IN.clone(), PORT_EXEC_IN.clone()],
            vec![
                prop("tag", "Log Tag", "string", Some(json!("APP")), None, None),
                prop(
                    "level",
                    "Level",
                    "enum",
                    Some(json!("info")),
                    Some(vec!["error", "warn", "info", "debug", "verbose"]),
                    None
                ),
            ],
            exec("runtime", Some("event"), "Logs messages to UART/console.")
        ),
    ];
}

pub fn list_node_types() -> Vec<NodeTypeDefResponse> {
    NODE_REGISTRY.iter().map(to_response).collect()
}

pub fn get_node_type_def(node_type: &str) -> Option<NodeTypeDefResponse> {
    NODE_REGISTRY
        .iter()
        .find(|d| d.node_type == node_type)
        .map(to_response)
}

fn to_response(def: &NodeTypeDef) -> NodeTypeDefResponse {
    NodeTypeDefResponse {
        node_type: def.node_type.to_string(),
        category: def.category.to_string(),
        label: def.label.to_string(),
        color: def.color.to_string(),
        icon: def.icon.to_string(),
        description: def.description.to_string(),
        ports: def
            .ports
            .iter()
            .map(|p| PortDefResponse {
                name: p.name.to_string(),
                direction: p.direction.clone(),
                datatype: p.datatype.to_string(),
                required: Some(p.required),
            })
            .collect(),
        properties: def.properties.to_vec(),
        execution_semantics: def.execution_semantics.clone(),
    }
}

pub fn default_ports_for_type(node_type: &str, node_id: &str) -> Vec<super::types::Port> {
    let Some(def) = NODE_REGISTRY.iter().find(|d| d.node_type == node_type) else {
        return Vec::new();
    };
    def.ports
        .iter()
        .enumerate()
        .map(|(i, p)| super::types::Port {
            id: format!("{}_{}_{}", node_id, p.name, i),
            name: p.name.to_string(),
            direction: p.direction.clone(),
            datatype: Some(p.datatype.to_string()),
            signal: None,
            hardware: None,
            required: p.required,
            multiplicity: "one".to_string(),
        })
        .collect()
}

pub fn default_properties_for_type(node_type: &str) -> serde_json::Value {
    let Some(def) = NODE_REGISTRY.iter().find(|d| d.node_type == node_type) else {
        return serde_json::json!({});
    };
    let mut map = serde_json::Map::new();
    for prop in &def.properties {
        if let Some(default) = &prop.default {
            map.insert(prop.key.clone(), default.clone());
        }
    }
    serde_json::Value::Object(map)
}

pub fn default_editable_for_type(node_type: &str) -> Vec<String> {
    let Some(def) = NODE_REGISTRY.iter().find(|d| d.node_type == node_type) else {
        return Vec::new();
    };
    def.properties
        .iter()
        .filter(|p| !p.read_only)
        .map(|p| p.key.clone())
        .collect()
}

/// Returns true if source datatype can connect to target datatype.
pub fn are_datatypes_compatible(source: &str, target: &str) -> bool {
    if source == target {
        return true;
    }
    let compatible: &[(&str, &str)] = &[
        ("execution", "execution"),
        ("event", "event"),
        ("signal", "signal"),
        ("signal", "payload"),
        ("payload", "mqtt_message"),
        ("gpio_level", "gpio_level"),
        ("gpio_level", "signal"),
        ("network", "network"),
        ("tensor", "tensor"),
        ("execution", "event"),
        ("event", "execution"),
    ];
    compatible.iter().any(|(s, t)| *s == source && *t == target)
}
