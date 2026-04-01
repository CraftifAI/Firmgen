## Introduction

A Universal Asynchronous Receiver/Transmitter (UART) is a hardware feature that handles communication (i.e., timing requirements and data framing) using widely-adopted asynchronous serial communication interfaces, such as RS232, RS422, and RS485. A UART provides a widely adopted and cheap method to realize full-duplex or half-duplex data exchange among different devices.

The ESP32 chip has 3 UART controllers (also referred to as port), each featuring an identical set of registers to simplify programming and for more flexibility.

Each UART controller is independently configurable with parameters such as baud rate, data bit length, bit ordering, number of stop bits, parity bit, etc. All the regular UART controllers are compatible with UART-enabled devices from various manufacturers and can also support Infrared Data Association (IrDA) protocols.

## Functional Overview

The overview describes how to establish communication between an ESP32 and other UART devices using the functions and data types of the UART driver. A typical programming workflow is broken down into the sections provided below:

1. [Install Drivers](#uart-api-driver-installation) - Allocating ESP32's resources for the UART driver
2. [Set Communication Parameters](#uart-api-setting-communication-parameters) - Setting baud rate, data bits, stop bits, etc.
3. [Set Communication Pins](#uart-api-setting-communication-pins) - Assigning pins for connection to a device
4. [Run UART Communication](#uart-api-running-uart-communication) - Sending/receiving data
5. [Use Interrupts](#uart-api-using-interrupts) - Triggering interrupts on specific communication events
6. [Deleting a Driver](#uart-api-deleting-driver) - Freeing allocated resources if a UART communication is no longer required

Steps 1 to 3 comprise the configuration stage. Step 4 is where the UART starts operating. Steps 5 and 6 are optional.

The UART driver's functions identify each of the UART controllers using [`uart_port_t`](#_CPPv411uart_port_t "uart_port_t"). This identification is needed for all the following function calls.

### Install Drivers

First of all, install the driver by calling [`uart_driver_install()`](#_CPPv419uart_driver_install11uart_port_tiiiP13QueueHandle_ti "uart_driver_install") and specify the following parameters:

The function allocates the required internal resources for the UART driver.

```
// Setup UART buffered IO with event queue
const int uart_buffer_size = (1024 * 2);
QueueHandle_t uart_queue;
// Install UART driver using an event queue here
ESP_ERROR_CHECK(uart_driver_install(UART_NUM_2, uart_buffer_size, uart_buffer_size, 10, &uart_queue, 0));
```

### Set Communication Parameters

As the next step, UART communication parameters can be configured all in a single step or individually in multiple steps.

#### Single Step

Call the function [`uart_param_config()`](#_CPPv417uart_param_config11uart_port_tPK13uart_config_t "uart_param_config") and pass to it a [`uart_config_t`](#_CPPv413uart_config_t "uart_config_t") structure. The [`uart_config_t`](#_CPPv413uart_config_t "uart_config_t") structure should contain all the required parameters. See the example below.

```
const uart_port_t uart_num = UART_NUM_2;
uart_config_t uart_config = {
    .baud_rate = 115200,
    .data_bits = UART_DATA_8_BITS,
    .parity = UART_PARITY_DISABLE,
    .stop_bits = UART_STOP_BITS_1,
    .flow_ctrl = UART_HW_FLOWCTRL_CTS_RTS,
    .rx_flow_ctrl_thresh = 122,
};
// Configure UART parameters
ESP_ERROR_CHECK(uart_param_config(uart_num, &uart_config));
```

For more information on how to configure the hardware flow control options, please refer to [peripherals/uart/uart\_echo](https://github.com/espressif/esp-idf/tree/35e0c8a3/examples/peripherals/uart/uart_echo).

#### Multiple Steps

Configure specific parameters individually by calling a dedicated function from the table given below. These functions are also useful if re-configuring a single parameter.

Each of the above functions has a `_get_` counterpart to check the currently set value. For example, to check the current baud rate value, call [`uart_get_baudrate()`](#_CPPv417uart_get_baudrate11uart_port_tP8uint32_t "uart_get_baudrate").

### Set Communication Pins

After setting communication parameters, configure the physical GPIO pins to which the other UART device will be connected. For this, call the function `uart_set_pin()` and specify the GPIO pin numbers to which the driver should route the TX, RX, RTS, CTS, DTR, and DSR signals. If you want to keep a currently allocated pin number for a specific signal, pass the macro [`UART_PIN_NO_CHANGE`](#c.UART_PIN_NO_CHANGE "UART_PIN_NO_CHANGE").

The same macro [`UART_PIN_NO_CHANGE`](#c.UART_PIN_NO_CHANGE "UART_PIN_NO_CHANGE") should be specified for pins that will not be used.

```
// Set UART pins(TX: IO4, RX: IO5, RTS: IO18, CTS: IO19, DTR: UNUSED, DSR: UNUSED)
ESP_ERROR_CHECK(uart_set_pin(UART_NUM_2, 4, 5, 18, 19, UART_PIN_NO_CHANGE, UART_PIN_NO_CHANGE));
```

### Run UART Communication

Serial communication is controlled by each UART controller's finite state machine (FSM).

The process of sending data involves the following steps:

1. Write data into TX FIFO buffer
2. FSM serializes the data
3. FSM sends the data out

The process of receiving data is similar, but the steps are reversed:

1. FSM processes an incoming serial stream and parallelizes it
2. FSM writes the data into RX FIFO buffer
3. Read the data from RX FIFO buffer

Therefore, an application only writes and reads data from a specific buffer using [`uart_write_bytes()`](#_CPPv416uart_write_bytes11uart_port_tPKv6size_t "uart_write_bytes") and [`uart_read_bytes()`](#_CPPv415uart_read_bytes11uart_port_tPv8uint32_t8uint32_t "uart_read_bytes") respectively, and the FSM does the rest.

#### Transmit Data

After preparing the data for transmission, call the function [`uart_write_bytes()`](#_CPPv416uart_write_bytes11uart_port_tPKv6size_t "uart_write_bytes") and pass the data buffer's address and data length to it. The function copies the data to the TX ring buffer (either immediately or after enough space is available), and then exit. When there is free space in the TX FIFO buffer, an interrupt service routine (ISR) moves the data from the TX ring buffer to the TX FIFO buffer in the background. The code below demonstrates the use of this function.

```
// Write data to UART.
char* test_str = "This is a test string.\n";
uart_write_bytes(uart_num, (const char*)test_str, strlen(test_str));
```

The function [`uart_write_bytes_with_break()`](#_CPPv427uart_write_bytes_with_break11uart_port_tPKv6size_ti "uart_write_bytes_with_break") is similar to [`uart_write_bytes()`](#_CPPv416uart_write_bytes11uart_port_tPKv6size_t "uart_write_bytes") but adds a serial break signal at the end of the transmission. A 'serial break signal' means holding the TX line low for a period longer than one data frame.

```
// Write data to UART, end with a break signal.
uart_write_bytes_with_break(uart_num, "test break\n",strlen("test break\n"), 100);
```

Another function for writing data to the TX FIFO buffer is [`uart_tx_chars()`](#_CPPv413uart_tx_chars11uart_port_tPKc8uint32_t "uart_tx_chars"). Unlike [`uart_write_bytes()`](#_CPPv416uart_write_bytes11uart_port_tPKv6size_t "uart_write_bytes"), this function does not block until space is available. Instead, it writes all data which can immediately fit into the hardware TX FIFO, and then return the number of bytes that were written.

There is a 'companion' function [`uart_wait_tx_done()`](#_CPPv417uart_wait_tx_done11uart_port_t8uint32_t "uart_wait_tx_done") that monitors the status of the TX FIFO buffer and returns once it is empty.

```
// Wait for packet to be sent
const uart_port_t uart_num = UART_NUM_2;
ESP_ERROR_CHECK(uart_wait_tx_done(uart_num, 100)); // wait timeout is 100 RTOS ticks (TickType_t)
```

#### Receive Data

Once the data is received by the UART and saved in the RX FIFO buffer, it needs to be retrieved using the function [`uart_read_bytes()`](#_CPPv415uart_read_bytes11uart_port_tPv8uint32_t8uint32_t "uart_read_bytes"). Before reading data, you can check the number of bytes available in the RX FIFO buffer by calling [`uart_get_buffered_data_len()`](#_CPPv426uart_get_buffered_data_len11uart_port_tP6size_t "uart_get_buffered_data_len"). An example of using these functions is given below.

```
// Read data from UART.
const uart_port_t uart_num = UART_NUM_2;
uint8_t data[128];
int length = 0;
ESP_ERROR_CHECK(uart_get_buffered_data_len(uart_num, (size_t*)&length));
length = uart_read_bytes(uart_num, data, length, 100);
```

If the data in the RX FIFO buffer is no longer needed, you can clear the buffer by calling [`uart_flush()`](#_CPPv410uart_flush11uart_port_t "uart_flush").

#### Software Flow Control

If the hardware flow control is disabled, you can manually set the RTS and DTR signal levels by using the functions [`uart_set_rts()`](#_CPPv412uart_set_rts11uart_port_ti "uart_set_rts") and [`uart_set_dtr()`](#_CPPv412uart_set_dtr11uart_port_ti "uart_set_dtr") respectively.

#### Communication Mode Selection

The UART controller supports a number of communication modes. A mode can be selected using the function [`uart_set_mode()`](#_CPPv413uart_set_mode11uart_port_t11uart_mode_t "uart_set_mode"). Once a specific mode is selected, the UART driver handles the behavior of a connected UART device accordingly. As an example, it can control the RS485 driver chip using the RTS line to allow half-duplex RS485 communication.

```
// Setup UART in rs485 half duplex mode
ESP_ERROR_CHECK(uart_set_mode(uart_num, UART_MODE_RS485_HALF_DUPLEX));
```

### Use Interrupts

There are many interrupts that can be generated depending on specific UART states or detected errors. The full list of available interrupts is provided in *ESP32 Technical Reference Manual* > *UART Controller (UART)* > *UART Interrupts* [[PDF](https://www.espressif.com/sites/default/files/documentation/esp32_technical_reference_manual_en.pdf#uart)]. You can enable or disable specific interrupts by calling [`uart_enable_intr_mask()`](#_CPPv421uart_enable_intr_mask11uart_port_t8uint32_t "uart_enable_intr_mask") or [`uart_disable_intr_mask()`](#_CPPv422uart_disable_intr_mask11uart_port_t8uint32_t "uart_disable_intr_mask") respectively.

The UART driver provides a convenient way to handle specific interrupts by wrapping them into corresponding events. Events defined in [`uart_event_type_t`](#_CPPv417uart_event_type_t "uart_event_type_t") can be reported to a user application using the FreeRTOS queue functionality.

To receive the events that have happened, call [`uart_driver_install()`](#_CPPv419uart_driver_install11uart_port_tiiiP13QueueHandle_ti "uart_driver_install") and get the event queue handle returned from the function. Please see the above [code snippet](#driver-code-snippet) as an example.

The processed events include the following:

- **FIFO overflow** ([`UART_FIFO_OVF`](#_CPPv4N17uart_event_type_t13UART_FIFO_OVFE "UART_FIFO_OVF")): The RX FIFO can trigger an interrupt when it receives more data than the FIFO can store.

  > - (Optional) Configure the full threshold of the FIFO space by entering it in the structure [`uart_intr_config_t`](#_CPPv418uart_intr_config_t "uart_intr_config_t") and call [`uart_intr_config()`](#_CPPv416uart_intr_config11uart_port_tPK18uart_intr_config_t "uart_intr_config") to set the configuration. This can help the data stored in the RX FIFO can be processed timely in the driver to avoid FIFO overflow.
  > - Enable the interrupts using the functions [`uart_enable_rx_intr()`](#_CPPv419uart_enable_rx_intr11uart_port_t "uart_enable_rx_intr").
  > - Disable these interrupts using the corresponding functions [`uart_disable_rx_intr()`](#_CPPv420uart_disable_rx_intr11uart_port_t "uart_disable_rx_intr").

  ```
  const uart_port_t uart_num = UART_NUM_2;
  // Configure a UART interrupt threshold and timeout
  uart_intr_config_t uart_intr = {
      .intr_enable_mask = UART_INTR_RXFIFO_FULL | UART_INTR_RXFIFO_TOUT,
      .rxfifo_full_thresh = 100,
      .rx_timeout_thresh = 10,
  };
  ESP_ERROR_CHECK(uart_intr_config(uart_num, &uart_intr));

  // Enable UART RX FIFO full threshold and timeout interrupts
  ESP_ERROR_CHECK(uart_enable_rx_intr(uart_num));
  ```
- **Pattern detection** ([`UART_PATTERN_DET`](#_CPPv4N17uart_event_type_t16UART_PATTERN_DETE "UART_PATTERN_DET")): An interrupt triggered on detecting a 'pattern' of the same character being received/sent repeatedly. It can be used, e.g., to detect a command string with a specific number of identical characters (the 'pattern') at the end. The following functions are available:



  ```
  //Set UART pattern detect function
  uart_enable_pattern_det_baud_intr(EX_UART_NUM, '+', PATTERN_CHR_NUM, 9, 0, 0);
  ```
- **Other events**: The UART driver can report other events such as data receiving ([`UART_DATA`](#_CPPv4N17uart_event_type_t9UART_DATAE "UART_DATA")), ring buffer full ([`UART_BUFFER_FULL`](#_CPPv4N17uart_event_type_t16UART_BUFFER_FULLE "UART_BUFFER_FULL")), detecting NULL after the stop bit ([`UART_BREAK`](#_CPPv4N17uart_event_type_t10UART_BREAKE "UART_BREAK")), parity check error ([`UART_PARITY_ERR`](#_CPPv4N17uart_event_type_t15UART_PARITY_ERRE "UART_PARITY_ERR")), and frame error ([`UART_FRAME_ERR`](#_CPPv4N17uart_event_type_t14UART_FRAME_ERRE "UART_FRAME_ERR")).

The strings inside of brackets indicate corresponding event names. An example of how to handle various UART events can be found in [peripherals/uart/uart\_events](https://github.com/espressif/esp-idf/tree/35e0c8a3/examples/peripherals/uart/uart_events).

### Macros

The API also defines several macros. For example, [`UART_HW_FIFO_LEN`](#c.UART_HW_FIFO_LEN "UART_HW_FIFO_LEN") defines the length of hardware FIFO buffers; [`UART_BITRATE_MAX`](#c.UART_BITRATE_MAX "UART_BITRATE_MAX") gives the maximum baud rate supported by the UART controllers, etc.

## Overview of RS485 Specific Communication 0ptions

Note

The following section uses `[UART_REGISTER_NAME].[UART_FIELD_BIT]` to refer to UART register fields/bits. For more information on a specific option bit, see **ESP32 Technical Reference Manual** > **UART Controller (UART)** > **Register Summary** [[PDF](https://www.espressif.com/sites/default/files/documentation/esp32_technical_reference_manual_en.pdf#uart-reg-summ)]. Use the register name to navigate to the register description and then find the field/bit.

- `UART_RS485_CONF_REG.UART_RS485_EN`: setting this bit enables RS485 communication mode support.
- `UART_RS485_CONF_REG.UART_RS485TX_RX_EN`: if this bit is set, the transmitter's output signal loops back to the receiver's input signal.
- `UART_RS485_CONF_REG.UART_RS485RXBY_TX_EN`: if this bit is set, the transmitter will still be sending data if the receiver is busy (remove collisions automatically by hardware).

The ESP32's RS485 UART hardware can detect signal collisions during transmission of a datagram and generate the interrupt `UART_RS485_CLASH_INT` if this interrupt is enabled. The term collision means that a transmitted datagram is not equal to the one received on the other end. Data collisions are usually associated with the presence of other active devices on the bus or might occur due to bus errors.

The collision detection feature allows handling collisions when their interrupts are activated and triggered. The interrupts `UART_RS485_FRM_ERR_INT` and `UART_RS485_PARITY_ERR_INT` can be used with the collision detection feature to control frame errors and parity bit errors accordingly in RS485 mode. This functionality is supported in the UART driver and can be used by selecting the [`UART_MODE_RS485_APP_CTRL`](#_CPPv4N11uart_mode_t24UART_MODE_RS485_APP_CTRLE "UART_MODE_RS485_APP_CTRL") mode (see the function [`uart_set_mode()`](#_CPPv413uart_set_mode11uart_port_t11uart_mode_t "uart_set_mode")).

The collision detection feature can work with circuit A and circuit C (see Section [Interface Connection Options](#interface-connection-options)). Use the function [`uart_get_collision_flag()`](#_CPPv423uart_get_collision_flag11uart_port_tPb "uart_get_collision_flag") to check if the collision detection flag has been raised. In the case of using circuit A or B, either DTR or RTS pin can be connected to the DE/~RE pin of the transceiver module to achieve half-duplex communication.

The RS485 half-duplex communication mode is supported by the UART driver and can be activated by selecting the [`UART_MODE_RS485_HALF_DUPLEX`](#_CPPv4N11uart_mode_t27UART_MODE_RS485_HALF_DUPLEXE "UART_MODE_RS485_HALF_DUPLEX") mode calling [`uart_set_mode()`](#_CPPv413uart_set_mode11uart_port_t11uart_mode_t "uart_set_mode"). The DTR line is automatically controlled by the hardware directly under RS485 half-duplex mode, while the RTS line is software-controlled by the UART driver. Once the host starts writing data to the TX FIFO buffer, the UART driver automatically asserts the RTS pin (logic 1); once the last bit of the data has been transmitted, the driver de-asserts the RTS pin (logic 0). To use this mode, the software would have to disable the hardware flow control function. Since the switching is made in the interrupt handler, comparing to DTR line, some latency is expected on RTS line.

Note

On ESP32, DTR signal is only available on UART0. For other UART ports, you can only connect RTS signal to the DE/~RE pin of the transceiver module.

### Interface Connection Options

This section provides example schematics to demonstrate the basic aspects of ESP32's RS485 interface connection.

Note

- The schematics below do **not** necessarily contain **all required elements**.
- The **analog devices** ADM483 & ADM2483 are examples of common RS485 transceivers and **can be replaced** with other similar transceivers.

#### Circuit A: Collision Detection Circuit

```
        VCC ---------------+
                           |
                   +-------x-------+
        RXD <------| R             |
                   |              B|----------<> B
        TXD ------>| D    ADM483   |
ESP                |               |     RS485 bus side
    DTR/RTS ------>| DE            |
                   |              A|----------<> A
              +----| /RE           |
              |    +-------x-------+
              |            |
             GND          GND
```

This circuit is preferable because it allows for collision detection and is quite simple at the same time. The receiver in the line driver is constantly enabled, which allows the UART to monitor the RS485 bus. Echo suppression is performed by the UART peripheral when the bit `UART_RS485_CONF_REG.UART_RS485TX_RX_EN` is enabled.

#### Circuit B: Manual Switching Transmitter/Receiver Without Collision Detection

```
        VCC ---------------+
                           |
                   +-------x-------+
        RXD <------| R             |
                   |              B|-----------<> B
        TXD ------>| D    ADM483   |
ESP                |               |     RS485 bus side
    DTR/RTS --+--->| DE            |
              |    |              A|-----------<> A
              +----| /RE           |
                   +-------x-------+
                           |
                          GND
```

This circuit does not allow for collision detection. It suppresses the null bytes that the hardware receives when the bit `UART_RS485_CONF_REG.UART_RS485TX_RX_EN` is set. The bit `UART_RS485_CONF_REG.UART_RS485RXBY_TX_EN` is not applicable in this case.

#### Circuit C: Auto Switching Transmitter/Receiver

```
 VCC1 <-------------------+-----------+           +-------------------+----> VCC2
               10K ____   |           |           |                   |
              +---|____|--+       +---x-----------x---+    10K ____   |
              |                   |                   |   +---|____|--+
RX <----------+-------------------| RXD               |   |
                   10K ____       |                  A|---+---------------<> A (+)
              +-------|____|------| PV    ADM2483     |   |    ____  120
              |   ____            |                   |   +---|____|---+  RS485 bus side
      VCC1 <--+--|____|--+------->| DE                |                |
              10K        |        |                  B|---+------------+--<> B (-)
                      ---+    +-->| /RE               |   |    ____
         10K          |       |   |                   |   +---|____|---+
        ____       | /-C      +---| TXD               |    10K         |
TX >---|____|--+_B_|/   NPN   |   |                   |                |
                   |\         |   +---x-----------x---+                |
                   | \-E      |       |           |                    |
                      |       |       |           |                    |
                     GND1    GND1    GND1        GND2                 GND2
```

This galvanically isolated circuit does not require RTS pin control by a software application or driver because it controls the transceiver direction automatically. However, it requires suppressing null bytes during transmission by setting `UART_RS485_CONF_REG.UART_RS485RXBY_TX_EN` to 1 and `UART_RS485_CONF_REG.UART_RS485TX_RX_EN` to 0. This setup can work in any RS485 UART mode or even in [`UART_MODE_UART`](#_CPPv4N11uart_mode_t14UART_MODE_UARTE "UART_MODE_UART").