use super::registry::{default_ports_for_type, default_properties_for_type};
use super::types::{ExecutionMetadata, FirmwareGraph, FirmwareNode, HardwareMetadata, SCHEMA_VERSION};
use serde_json::json;

pub fn sample_blink() -> FirmwareGraph {
    let boot = node(
        "boot",
        "system_init",
        "Boot",
        json!({"target": "esp32s3", "log_level": "info"}),
        None,
    );
    let gpio_init = node(
        "gpio_init",
        "gpio_output",
        "GPIO Init",
        json!({"pin": 2, "active_level": "high", "initial_state": false}),
        Some(HardwareMetadata {
            gpio: Some(2),
            bus: Some("gpio".to_string()),
            peripheral: None,
            pin_label: Some("LED".to_string()),
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        }),
    );
    let blink_task = node(
        "blink_task",
        "rtos_task",
        "Blink Task",
        json!({"task_name": "blink", "priority": 5, "stack_size": 2048, "core": 0, "period_ms": 500}),
        None,
    );
    let led = node(
        "led_out",
        "gpio_output",
        "LED Output",
        json!({"pin": 2, "active_level": "high", "initial_state": false}),
        Some(HardwareMetadata {
            gpio: Some(2),
            bus: Some("gpio".to_string()),
            peripheral: None,
            pin_label: Some("LED".to_string()),
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        }),
    );

    FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some("sample_blink".to_string()),
        name: Some("Blink LED".to_string()),
        description: Some(
            "Classic ESP-IDF blink: Boot → GPIO Init → Blink Task → LED Output".to_string(),
        ),
        board_id: Some("esp32-s3-devkitc-1".to_string()),
        nodes: vec![boot, gpio_init, blink_task, led],
        connections: vec![
            [
                port_id("boot", "boot_out", 0),
                port_id("gpio_init", "exec_in", 0),
            ],
            [
                port_id("gpio_init", "exec_in", 0),
                port_id("blink_task", "trigger_in", 0),
            ],
            [
                port_id("blink_task", "exec_out", 1),
                port_id("led_out", "exec_in", 0),
            ],
        ],
        layout: None,
        runtime_metadata: None,
    }
}

pub fn sample_pir_motion() -> FirmwareGraph {
    let boot = node(
        "boot",
        "system_init",
        "Boot",
        json!({"target": "esp32s3", "log_level": "info"}),
        None,
    );
    let pir = node(
        "pir_sensor",
        "sensor_input",
        "PIR Motion Sensor",
        json!({"sensor_type": "pir", "pin": 4, "driver": "gpio"}),
        Some(HardwareMetadata {
            gpio: Some(4),
            bus: Some("gpio".to_string()),
            peripheral: Some("pir".to_string()),
            pin_label: None,
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        }),
    );
    let gpio_isr = node(
        "gpio_isr",
        "event_handler",
        "GPIO Interrupt",
        json!({"event_base": "GPIO", "event_id": 4}),
        None,
    );
    let motion_task = node(
        "motion_task",
        "rtos_task",
        "Motion Task",
        json!({"task_name": "motion_handler", "priority": 6, "stack_size": 4096, "core": 0, "period_ms": 0}),
        None,
    );
    let led = node(
        "led_out",
        "gpio_output",
        "LED Output",
        json!({"pin": 2, "active_level": "high", "initial_state": false}),
        Some(HardwareMetadata {
            gpio: Some(2),
            bus: Some("gpio".to_string()),
            peripheral: None,
            pin_label: Some("LED".to_string()),
            i2c_address: None,
            spi_host: None,
            uart_port: None,
        }),
    );
    let wifi = node(
        "wifi",
        "wifi_manager",
        "WiFi",
        json!({"ssid": "MyNetwork", "password": "", "mode": "station"}),
        None,
    );
    let mqtt = node(
        "mqtt",
        "mqtt_client",
        "MQTT Publish",
        json!({"broker_url": "mqtt://broker.hivemq.com:1883", "topic": "home/motion", "client_id": "esp32-pir", "qos": 0}),
        None,
    );

    FirmwareGraph {
        schema_version: SCHEMA_VERSION,
        id: Some("sample_pir_motion".to_string()),
        name: Some("PIR Motion → MQTT".to_string()),
        description: Some(
            "PIR sensor triggers GPIO interrupt, RTOS task drives LED and MQTT publish over WiFi"
                .to_string(),
        ),
        board_id: Some("esp32-s3-devkitc-1".to_string()),
        nodes: vec![boot, pir, gpio_isr, motion_task, led, wifi, mqtt],
        connections: vec![
            [
                port_id("boot", "boot_out", 0),
                port_id("pir_sensor", "exec_in", 0),
            ],
            [
                port_id("pir_sensor", "data_out", 1),
                port_id("gpio_isr", "event_in", 0),
            ],
            [
                port_id("gpio_isr", "exec_out", 2),
                port_id("motion_task", "trigger_in", 0),
            ],
            [
                port_id("motion_task", "exec_out", 1),
                port_id("led_out", "exec_in", 0),
            ],
            [
                port_id("motion_task", "data_out", 2),
                port_id("mqtt", "data_in", 1),
            ],
            [
                port_id("boot", "boot_out", 0),
                port_id("wifi", "exec_in", 0),
            ],
            [
                port_id("wifi", "network_out", 1),
                port_id("mqtt", "network_in", 0),
            ],
        ],
        layout: None,
        runtime_metadata: None,
    }
}

fn node(
    id: &str,
    node_type: &str,
    label: &str,
    properties: serde_json::Value,
    hardware: Option<HardwareMetadata>,
) -> FirmwareNode {
    FirmwareNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        label: Some(label.to_string()),
        description: None,
        ports: default_ports_for_type(node_type, id),
        properties: merge_properties(node_type, properties),
        hardware,
        execution: Some(default_execution(node_type)),
        visual: None,
        validation_state: None,
        runtime_state: None,
    }
}

fn merge_properties(node_type: &str, overrides: serde_json::Value) -> serde_json::Value {
    let mut base = default_properties_for_type(node_type);
    if let (Some(base_map), Some(override_map)) = (base.as_object_mut(), overrides.as_object()) {
        for (k, v) in override_map {
            base_map.insert(k.clone(), v.clone());
        }
    }
    base
}

fn default_execution(node_type: &str) -> ExecutionMetadata {
    use super::registry::get_node_type_def;
    let def = get_node_type_def(node_type);
    ExecutionMetadata {
        phase: def.as_ref().map(|d| d.execution_semantics.phase.clone()),
        priority: None,
        stack_size: None,
        core_affinity: None,
        period_ms: None,
        trigger: def.and_then(|d| d.execution_semantics.trigger.clone()),
    }
}

fn port_id(node_id: &str, port_name: &str, index: usize) -> String {
    format!("{}_{}_{}", node_id, port_name, index)
}
