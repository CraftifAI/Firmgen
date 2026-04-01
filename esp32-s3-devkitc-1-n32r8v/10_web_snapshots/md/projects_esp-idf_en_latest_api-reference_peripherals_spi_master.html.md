SPI Master driver is a program that controls ESP32's General Purpose SPI (GP-SPI) peripheral(s) when it functions as a master.

## SPI Transactions

An SPI bus transaction consists of five phases which can be found in the table below. Any of these phases can be skipped.

| Phase | Description |
| --- | --- |
| Command | In this phase, a command (0-16 bit) is written to the bus by the Host. |
| Address | In this phase, an address (0-64 bit) is transmitted over the bus by the Host. |
| Dummy | This phase is configurable and is used to meet the timing requirements. |
| Write | Host sends data to a Device. This data follows the optional command and address phases and is indistinguishable from them at the electrical level. |
| Read | Device sends data to its Host. |

The attributes of a transaction are determined by the bus configuration structure [`spi_bus_config_t`](#_CPPv416spi_bus_config_t "spi_bus_config_t"), Device configuration structure [`spi_device_interface_config_t`](#_CPPv429spi_device_interface_config_t "spi_device_interface_config_t"), and transaction configuration structure [`spi_transaction_t`](#_CPPv417spi_transaction_t "spi_transaction_t").

An SPI Host can send full-duplex transactions, during which the Read and Write phases occur simultaneously. The total transaction length is determined by the sum of the following members:

While the member [`spi_transaction_t::rxlength`](#_CPPv4N17spi_transaction_t8rxlengthE "spi_transaction_t::rxlength") only determines the length of data received into the buffer.

In half-duplex transactions, the Read and Write phases are not simultaneous (one direction at a time). The lengths of the Write and Read phases are determined by [`spi_transaction_t::length`](#_CPPv4N17spi_transaction_t6lengthE "spi_transaction_t::length") and [`spi_transaction_t::rxlength`](#_CPPv4N17spi_transaction_t8rxlengthE "spi_transaction_t::rxlength") respectively.

The Command and Address phases are optional, as not every SPI Device requires a command and/or address. This is reflected in the Device's configuration: if [`spi_device_interface_config_t::command_bits`](#_CPPv4N29spi_device_interface_config_t12command_bitsE "spi_device_interface_config_t::command_bits") and/or [`spi_device_interface_config_t::address_bits`](#_CPPv4N29spi_device_interface_config_t12address_bitsE "spi_device_interface_config_t::address_bits") are set to zero, no Command or Address phase will occur.

The Read and Write phases can also be optional, as not every transaction requires both writing and reading data. If [`spi_transaction_t::rx_buffer`](#_CPPv4N17spi_transaction_t9rx_bufferE "spi_transaction_t::rx_buffer") is `NULL` and [`SPI_TRANS_USE_RXDATA`](#c.SPI_TRANS_USE_RXDATA "SPI_TRANS_USE_RXDATA") is not set, the Read phase is skipped. If [`spi_transaction_t::tx_buffer`](#_CPPv4N17spi_transaction_t9tx_bufferE "spi_transaction_t::tx_buffer") is `NULL` and [`SPI_TRANS_USE_TXDATA`](#c.SPI_TRANS_USE_TXDATA "SPI_TRANS_USE_TXDATA") is not set, the Write phase is skipped.

The driver supports two types of transactions: interrupt transactions and polling transactions. The programmer can choose to use a different transaction type per Device. If your Device requires both transaction types, see [Notes on Sending Mixed Transactions to the Same Device](#mixed-transactions).

### Interrupt Transactions

Interrupt transactions blocks the transaction routine until the transaction completes, thus allowing the CPU to run other tasks.

An application task can queue multiple transactions, and the driver automatically handles them one by one in the interrupt service routine (ISR). It allows the task to switch to other procedures until all the transactions are complete.

### Polling Transactions

Polling transactions do not use interrupts. The routine keeps polling the SPI Host's status bit until the transaction is finished.

All the tasks that use interrupt transactions can be blocked by the queue. At this point, they need to wait for the ISR to run twice before the transaction is finished. Polling transactions save time otherwise spent on queue handling and context switching, which results in smaller transaction duration. The disadvantage is that the CPU is busy while these transactions are in progress.

The [`spi_device_polling_end()`](#_CPPv422spi_device_polling_end19spi_device_handle_t8uint32_t "spi_device_polling_end") routine needs an overhead of at least 1 Âµs to unblock other tasks when the transaction is finished. It is strongly recommended to wrap a series of polling transactions using the functions [`spi_device_acquire_bus()`](#_CPPv422spi_device_acquire_bus19spi_device_handle_t8uint32_t "spi_device_acquire_bus") and [`spi_device_release_bus()`](#_CPPv422spi_device_release_bus19spi_device_handle_t "spi_device_release_bus") to avoid the overhead. For more information, see [Bus Acquiring](#bus-acquiring).

### Transaction Line Mode

Supported line modes for ESP32 are listed as follows, to make use of these modes, set the member `flags` in the struct [`spi_transaction_t`](#_CPPv417spi_transaction_t "spi_transaction_t") as shown in the `Transaction Flag` column. If you want to check if corresponding IO pins are set or not, set the member `flags` in the [`spi_bus_config_t`](#_CPPv416spi_bus_config_t "spi_bus_config_t") as shown in the `Bus IO setting Flag` column.

| Mode name | Command Line Width | Address Line Width | Data Line Width | Transaction Flag | Bus IO Setting Flag |
| --- | --- | --- | --- | --- | --- |
| Normal SPI | 1 | 1 | 1 | 0 | 0 |
| Dual Output | 1 | 1 | 2 | SPI\_TRANS\_MODE\_DIO | SPICOMMON\_BUSFLAG\_DUAL |
| Dual I/O | 1 | 2 | 2 | SPI\_TRANS\_MODE\_DIO SPI\_TRANS\_MULTILINE\_ADDR | SPICOMMON\_BUSFLAG\_DUAL |
| Quad Output | 1 | 1 | 4 | SPI\_TRANS\_MODE\_QIO | SPICOMMON\_BUSFLAG\_QUAD |
| Quad I/O | 1 | 4 | 4 | SPI\_TRANS\_MODE\_QIO SPI\_TRANS\_MULTILINE\_ADDR | SPICOMMON\_BUSFLAG\_QUAD |

### Write and Read Phases

Normally, the data that needs to be transferred to or from a Device is read from or written to a chunk of memory indicated by the members [`spi_transaction_t::rx_buffer`](#_CPPv4N17spi_transaction_t9rx_bufferE "spi_transaction_t::rx_buffer") and [`spi_transaction_t::tx_buffer`](#_CPPv4N17spi_transaction_t9tx_bufferE "spi_transaction_t::tx_buffer"). If DMA is enabled for transfers, the buffers are required to be:

> 1. Allocated in DMA-capable internal memory (MALLOC\_CAP\_DMA), see [DMA-Capable Memory](../system/mem_alloc.html#dma-capable-memory).
> 2. 32-bit aligned (starting from a 32-bit boundary and having a length of multiples of 4 bytes).

If these requirements are not satisfied, the transaction efficiency will be affected due to the allocation and copying of temporary buffers.

If using more than one data line to transmit, please set `SPI_DEVICE_HALFDUPLEX` flag for the member `flags` in the struct [`spi_device_interface_config_t`](#_CPPv429spi_device_interface_config_t "spi_device_interface_config_t"). And the member `flags` in the struct [`spi_transaction_t`](#_CPPv417spi_transaction_t "spi_transaction_t") should be set as described in [Transaction Line Mode](#transaction-line-mode).

Note

Half-duplex transactions with both Read and Write phases are not supported when using DMA. For details and workarounds, see [Known Issues](#spi-known-issues).

### Bus Acquiring

Sometimes you might want to send SPI transactions exclusively and continuously so that it takes as little time as possible. For this, you can use bus acquiring, which helps to suspend transactions (both polling or interrupt) to other Devices until the bus is released. To acquire and release a bus, use the functions [`spi_device_acquire_bus()`](#_CPPv422spi_device_acquire_bus19spi_device_handle_t8uint32_t "spi_device_acquire_bus") and [`spi_device_release_bus()`](#_CPPv422spi_device_release_bus19spi_device_handle_t "spi_device_release_bus").

## Driver Usage

- Initialize an SPI bus by calling the function [`spi_bus_initialize()`](#_CPPv418spi_bus_initialize17spi_host_device_tPK16spi_bus_config_t14spi_dma_chan_t "spi_bus_initialize"). Make sure to set the correct I/O pins in the struct [`spi_bus_config_t`](#_CPPv416spi_bus_config_t "spi_bus_config_t"). Set the signals that are not needed to `-1`.
- Register a Device connected to the bus with the driver by calling the function [`spi_bus_add_device()`](#_CPPv418spi_bus_add_device17spi_host_device_tPK29spi_device_interface_config_tP19spi_device_handle_t "spi_bus_add_device"). Make sure to configure any timing requirements the Device might need with the parameter `dev_config`. You should now have obtained the Device's handle which will be used when sending a transaction to it.
- To interact with the Device, fill one or more [`spi_transaction_t`](#_CPPv417spi_transaction_t "spi_transaction_t") structs with any transaction parameters required. Then send the structs either using a polling transaction or an interrupt transaction:
- (Optional) To perform back-to-back transactions with a Device, call the function [`spi_device_acquire_bus()`](#_CPPv422spi_device_acquire_bus19spi_device_handle_t8uint32_t "spi_device_acquire_bus") before sending transactions and [`spi_device_release_bus()`](#_CPPv422spi_device_release_bus19spi_device_handle_t "spi_device_release_bus") after the transactions have been sent.
- (Optional) To remove a certain Device from the bus, call [`spi_bus_remove_device()`](#_CPPv421spi_bus_remove_device19spi_device_handle_t "spi_bus_remove_device") with the Device handle as an argument.
- (Optional) To remove the driver from the bus, make sure no more devices are attached and call [`spi_bus_free()`](#_CPPv412spi_bus_free17spi_host_device_t "spi_bus_free").

The example code for the SPI Master driver can be found in the [peripherals/spi\_master](https://github.com/espressif/esp-idf/tree/35e0c8a3/examples/peripherals/spi_master) directory of ESP-IDF examples.

### Transactions with Integers Other than `uint8_t`

An SPI Host reads and writes data into memory byte by byte. By default, data is sent with the most significant bit (MSB) first, as LSB is first used in rare cases. If a value of fewer than 8 bits needs to be sent, the bits should be written into memory in the MSB first manner.

For example, if `0b00010` needs to be sent, it should be written into a `uint8_t` variable, and the length for reading should be set to 5 bits. The Device will still receive 8 bits with 3 additional "random" bits, so the reading must be performed correctly.

On top of that, ESP32 is a little-endian chip, which means that the least significant byte of `uint16_t` and `uint32_t` variables is stored at the smallest address. Hence, if `uint16_t` is stored in memory, bits [7:0] are sent first, followed by bits [15:8].

For cases when the data to be transmitted has a size differing from `uint8_t` arrays, the following macros can be used to transform data to the format that can be sent by the SPI driver directly:

### Notes on Sending Mixed Transactions to the Same Device

To reduce coding complexity, send only one type of transaction (interrupt or polling) to one Device. However, you still can send both interrupt and polling transactions alternately. The notes below explain how to do this.

The polling transactions should be initiated only after all the polling and interrupt transactions are finished.

Since an unfinished polling transaction blocks other transactions, please do not forget to call the function [`spi_device_polling_end()`](#_CPPv422spi_device_polling_end19spi_device_handle_t8uint32_t "spi_device_polling_end") after [`spi_device_polling_start()`](#_CPPv424spi_device_polling_start19spi_device_handle_tP17spi_transaction_t8uint32_t "spi_device_polling_start") to allow other transactions or to allow other Devices to use the bus. Remember that if there is no need to switch to other tasks during your polling transaction, you can initiate a transaction with [`spi_device_polling_transmit()`](#_CPPv427spi_device_polling_transmit19spi_device_handle_tP17spi_transaction_t "spi_device_polling_transmit") so that it will be ended automatically.

In-flight polling transactions are disturbed by the ISR operation to accommodate interrupt transactions. Always make sure that all the interrupt transactions sent to the ISR are finished before you call [`spi_device_polling_start()`](#_CPPv424spi_device_polling_start19spi_device_handle_tP17spi_transaction_t8uint32_t "spi_device_polling_start"). To do that, you can keep calling [`spi_device_get_trans_result()`](#_CPPv427spi_device_get_trans_result19spi_device_handle_tPP17spi_transaction_t8uint32_t "spi_device_get_trans_result") until all the transactions are returned.

To have better control of the calling sequence of functions, send mixed transactions to the same Device only within a single task.

### Notes on Using the SPI Master Driver on SPI1 Bus

Note

Though the [SPI Bus Lock](spi_features.html#spi-bus-lock) feature makes it possible to use SPI Master driver on the SPI1 bus, it is still tricky and needs a lot of special treatment. It is a feature for advanced developers.

To use SPI Master driver on SPI1 bus, you have to take care of two problems:

1. The code and data should be in the internal memory when the driver is operating on SPI1 bus.

   SPI1 bus is shared among Devices and the cache for data (code) in the flash as well as the PSRAM. The cache should be disabled when other drivers are operating on the SPI1 bus. Hence the data (code) in the flash as well as the PSRAM cannot be fetched while the driver acquires the SPI1 bus by:

   During the time above, all other tasks and most ISRs will be disabled (see [IRAM-Safe Interrupt Handlers](spi_flash/spi_flash_concurrency.html#iram-safe-interrupt-handlers)). Application code and data used by the current task should be placed in internal memory (DRAM or IRAM), or already in the ROM. Access to external memory (flash code, const data in the flash, and static/heap data in the PSRAM) will cause a `Cache disabled but cached memory region accessed` exception. For differences between IRAM, DRAM, and flash cache, please refer to the [application memory layout](../../api-guides/memory-types.html#memory-layout) documentation.

   To place functions into the IRAM, you can either:

   1. Add `IRAM_ATTR` (include `esp_attr.h`) to the function like:

      > IRAM\_ATTR void foo(void) { }

      Please note that when a function is inlined, it will follow its caller's segment, and the attribute will not take effect. You may need to use `NOLINE_ATTR` to avoid this. Please also note that the compiler may transform some code into a lookup table in the const data, so `noflash_text` is not safe.
   2. Use the `noflash` placement in the `linker.lf`. See more in [Linker Script Generation](../../api-guides/linker-script-generation.html). Please note that the compiler may transform some code into a lookup table in the const data, so `noflash_text` is not safe.

   Please do take care that the optimization level may affect the compiler behavior of inline, or transform some code into a lookup table in the const data, etc.

   To place data into the DRAM, you can either:

   1. Add `DRAM_ATTR` (include `esp_attr.h`) to the data definition like:
   2. Use the `noflash` placement in the linker.lf. See more in [Linker Script Generation](../../api-guides/linker-script-generation.html).

Please also see the example [peripherals/spi\_master/hd\_eeprom](https://github.com/espressif/esp-idf/tree/35e0c8a3/examples/peripherals/spi_master/hd_eeprom).

### GPIO Matrix and IO\_MUX

Most of ESP32's peripheral signals have a direct connection to their dedicated IO\_MUX pins. However, the signals can also be routed to any other available pins using the less direct GPIO matrix. If at least one signal is routed through the GPIO matrix, then all signals will be routed through it.

The GPIO matrix introduces flexibility of routing but also brings the following disadvantages:

- Increases the input delay of the MISO signal, which makes MISO setup time violations more likely. If SPI needs to operate at high speeds, use dedicated IO\_MUX pins.
- Allows signals with clock frequencies only up to 40 MHz, as opposed to 80 MHz if IO\_MUX pins are used.

Note

For more details about the influence of the MISO input delay on the maximum clock frequency, see [Timing Considerations](#timing-considerations).

The IO\_MUX pins for SPI buses are given below.

| Pin Name | SPI 2 (GPIO Number) | SPI 3 (GPIO Number) |
| --- | --- | --- |
| CS0 | 15 | 5 |
| SCLK | 14 | 18 |
| MISO | 12 | 19 |
| MOSI | 13 | 23 |
| QUADWP | 2 | 22 |
| QUADHD | 4 | 21 |

## Transfer Speed Considerations

There are three factors limiting the transfer speed:

The main parameter that determines the transfer speed for large transactions is clock frequency. For multiple small transactions, the transfer speed is mostly determined by the length of transaction intervals.

### Transaction Duration

Transaction duration includes setting up SPI peripheral registers, copying data to FIFOs or setting up DMA links, and the time for SPI transactions.

Interrupt transactions allow appending extra overhead to accommodate the cost of FreeRTOS queues and the time needed for switching between tasks and the ISR.

For **interrupt transactions**, the CPU can switch to other tasks when a transaction is in progress. This saves CPU time but increases the transaction duration. See [Interrupt Transactions](#interrupt-transactions). For **polling transactions**, it does not block the task but allows to do polling when the transaction is in progress. For more information, see [Polling Transactions](#polling-transactions).

If DMA is enabled, setting up the linked list requires about 2 Âµs per transaction. When a master is transferring data, it automatically reads the data from the linked list. If DMA is not enabled, the CPU has to write and read each byte from the FIFO by itself. Usually, this is faster than 2 Âµs, but the transaction length is limited to 64 bytes for both write and read.

The typical transaction duration for one byte of data is given below.

- Interrupt Transaction via DMA: 28 Âµs.
- Interrupt Transaction via CPU: 25 Âµs.
- Polling Transaction via DMA: 10 Âµs.
- Polling Transaction via CPU: 8 Âµs.

Note that these data are tested with [CONFIG\_SPI\_MASTER\_ISR\_IN\_IRAM](../kconfig-reference.html#config-spi-master-isr-in-iram) enabled. SPI transaction related code are placed in the internal memory. If this option is turned off (for example, for internal memory optimization), the transaction duration may be affected.

### SPI Clock Frequency

The clock source of the GPSPI peripherals can be selected by setting [`spi_device_interface_config_t::clock_source`](#_CPPv4N29spi_device_interface_config_t12clock_sourceE "spi_device_interface_config_t::clock_source"). You can refer to [`spi_clock_source_t`](#_CPPv418spi_clock_source_t "spi_clock_source_t") to know the supported clock sources.

By default driver sets clock source to `SPI_CLK_SRC_DEFAULT`. This usually stands for the highest frequency among GPSPI supported clock sources. Its value is different among chips.

The actual clock frequency of a device may not be exactly equal to the number you set, it is re-calculated by the driver to the nearest hardware-compatible number, and no more than the frequency of selected clock source. You can call [`spi_device_get_actual_freq()`](#_CPPv426spi_device_get_actual_freq19spi_device_handle_tPi "spi_device_get_actual_freq") to know the actual frequency computed by the driver.

The clock frequency of the device can be changed during transmission by setting [`spi_transaction_t::override_freq_hz`](#_CPPv4N17spi_transaction_t16override_freq_hzE "spi_transaction_t::override_freq_hz"). This operation will use new clock frequency for the device's current and later transmissions. If the expected clock frequency cannot be achieved, the driver will print an warning and continue to use the previous clock frequency for transmission.

The theoretical maximum transfer speed of the Write or Read phase can be calculated according to the table below:

| Line Width of Write/Read phase | Speed (Bps) |
| --- | --- |
| 1-Line | *SPI Frequency / 8* |
| 2-Line | *SPI Frequency / 4* |
| 4-Line | *SPI Frequency / 2* |

The transfer speed calculation of other phases (Command, Address, Dummy) is similar.

If the clock frequency is too high, the use of some functions might be limited. See [Timing Considerations](#timing-considerations).

### Cache Missing

The default config puts only the ISR into the IRAM. Other SPI-related functions, including the driver itself and the callback, might suffer from cache misses and need to wait until the code is read from flash. Select [CONFIG\_SPI\_MASTER\_IN\_IRAM](../kconfig-reference.html#config-spi-master-in-iram) to put the whole SPI driver into IRAM and put the entire callback(s) and its callee functions into IRAM to prevent cache missing.

For an interrupt transaction, the overall cost is **20+8n/Fspi[MHz]** [Âµs] for n bytes transferred in one transaction. Hence, the transferring speed is: **n/(20+8n/Fspi)**. An example of transferring speed at 8 MHz clock speed is given in the following table.

| Frequency (MHz) | Transaction Interval (Âµs) | Transaction Length (bytes) | Total Time (Âµs) | Total Speed (KBps) |
| --- | --- | --- | --- | --- |
| 8 | 25 | 1 | 26 | 38.5 |
| 8 | 25 | 8 | 33 | 242.4 |
| 8 | 25 | 16 | 41 | 490.2 |
| 8 | 25 | 64 | 89 | 719.1 |
| 8 | 25 | 128 | 153 | 836.6 |

When a transaction length is short, the cost of the transaction interval is high. If possible, try to squash several short transactions into one transaction to achieve a higher transfer speed.

Please note that the ISR is disabled during flash operation by default. To keep sending transactions during flash operations, enable [CONFIG\_SPI\_MASTER\_ISR\_IN\_IRAM](../kconfig-reference.html#config-spi-master-isr-in-iram) and set `ESP_INTR_FLAG_IRAM` in the member [`spi_bus_config_t::intr_flags`](#_CPPv4N16spi_bus_config_t10intr_flagsE "spi_bus_config_t::intr_flags"). In this case, all the transactions queued before starting flash operations are handled by the ISR in parallel. Also note that the callback of each Device and their `callee` functions should be in IRAM, or your callback will crash due to cache missing. For more details, see [IRAM-Safe Interrupt Handlers](spi_flash/spi_flash_concurrency.html#iram-safe-interrupt-handlers).

## Timing Considerations

As shown in the figure below, there is a delay on the MISO line after the SCLK launch edge and before the signal is latched by the internal register. As a result, the MISO pin setup time is the limiting factor for the SPI clock speed. When the delay is too long, the setup slack is < 0, which means the setup timing requirement is violated and the reading might be incorrect.

[![../../_images/spi_miso.png](../../_images/spi_miso.png)](../../_images/spi_miso.png)

The maximum allowed frequency is dependent on:

When the GPIO matrix is used, the maximum allowed frequency is reduced to about 33 ~ 77% in comparison to the existing **input delay**. To retain a higher frequency, you have to use the IO\_MUX pins or the **dummy bit workaround**. You can obtain the maximum reading frequency of the master by using the function [`spi_get_freq_limit()`](#_CPPv418spi_get_freq_limitbi "spi_get_freq_limit").

**Dummy bit workaround**: Dummy clocks, during which the Host does not read data, can be inserted before the Read phase begins. The Device still sees the dummy clocks and sends out data, but the Host does not read until the Read phase comes. This compensates for the lack of the MISO setup time required by the Host and allows the Host to do reading at a higher frequency.

In the ideal case, if the Device is so fast that the input delay is shorter than an APB clock cycle - 12.5 ns - the maximum frequency at which the Host can read (or read and write) in different conditions is as follows:

| Frequency Limit (MHz) | Frequency Limit (MHz) | Dummy Bits Used by Driver | Comments |
| --- | --- | --- | --- |
| GPIO Matrix | IO\_MUX Pins |  |  |
| 26.6 | 80 | No |  |
| 40 | -- | Yes | Half-duplex, no DMA allowed |

If the Host only writes data, the **dummy bit workaround** and the frequency check can be disabled by setting the bit `SPI_DEVICE_NO_DUMMY` in the member [`spi_device_interface_config_t::flags`](#_CPPv4N29spi_device_interface_config_t5flagsE "spi_device_interface_config_t::flags"). When disabled, the output frequency can be 80 MHz, even if the GPIO matrix is used.

[`spi_device_interface_config_t::flags`](#_CPPv4N29spi_device_interface_config_t5flagsE "spi_device_interface_config_t::flags")

The SPI Master driver still works even if the [`spi_device_interface_config_t::input_delay_ns`](#_CPPv4N29spi_device_interface_config_t14input_delay_nsE "spi_device_interface_config_t::input_delay_ns") in the structure [`spi_device_interface_config_t`](#_CPPv429spi_device_interface_config_t "spi_device_interface_config_t") is set to 0. However, setting an accurate value helps to:

You can approximate the maximum data valid time after the launch edge of SPI clocks by checking the statistics in the AC characteristics chapter of your Device's specification or measure the time using an oscilloscope or logic analyzer.

Please note that the actual PCB layout design and excessive loads may increase the input delay. It means that non-optimal wiring and/or a load capacitor on the bus will most likely lead to input delay values exceeding the values given in the Device specification or measured while the bus is floating.

Some typical delay values are shown in the following table. These data are retrieved when the slave Device is on a different physical chip.

| Device | Input Delay (ns) |
| --- | --- |
| Ideal Device | 0 |
| ESP32 slave using IO\_MUX | 50 |
| ESP32 slave using GPIO\_MATRIX | 75 |

The MISO path delay (valid time) consists of a slave's **input delay** plus the master's **GPIO matrix delay**. The delay determines the above frequency limit for full-duplex transfers. Once exceeding, full-duplex transfers will not work as well as the half-duplex transactions that use dummy bits. The frequency limit is:

> *Freq limit [MHz] = 80 / (floor(MISO delay[ns]/12.5) + 1)*

The figure below shows the relationship between frequency limit and input delay. Two extra APB clock cycle periods should be added to the MISO delay if the master uses the GPIO matrix.

![../../_images/spi_master_freq_tv.png](../../_images/spi_master_freq_tv.png)

Corresponding frequency limits for different Devices with different **input delay** times are shown in the table below.

When the master is IO\_MUX (0 ns):

| Input Delay (ns) | MISO Path Delay (ns) | Freq. Limit (MHz) |
| --- | --- | --- |
| 0 | 0 | 80 |
| 50 | 50 | 16 |
| 75 | 75 | 11.43 |

When the master is GPIO\_MATRIX (25 ns):

| Input Delay (ns) | MISO Path Delay (ns) | Freq. Limit (MHz) |
| --- | --- | --- |
| 0 | 25 | 26.67 |
| 50 | 75 | 11.43 |
| 75 | 100 | 8.89 |