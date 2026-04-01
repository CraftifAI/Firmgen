# GPIO & RTC GPIO

[[ä¸­æ]](../../../../../zh_CN/latest/esp32/api-reference/peripherals/gpio.html)

## GPIO Summary

The ESP32 chip features 34 physical GPIO pins (GPIO0 ~ GPIO19, GPIO21 ~ GPIO23, GPIO25 ~ GPIO27, and GPIO32 ~ GPIO39). Each pin can be used as a general-purpose I/O, or be connected to an internal peripheral signal. Through IO MUX, RTC IO MUX and the GPIO matrix, peripheral input signals can be from any IO pins, and peripheral output signals can be routed to any IO pins. Together these modules provide highly configurable I/O. For more details, see *ESP32 Technical Reference Manual* > *IO MUX and GPIO Matrix (GPIO, IO\_MUX)* [[PDF](https://www.espressif.com/sites/default/files/documentation/esp32_technical_reference_manual_en.pdf#iomuxgpio)].

The table below provides more information on pin usage, and please note the comments in the table for GPIOs with restrictions.

| GPIO | Analog Function | RTC GPIO | Comments |
| --- | --- | --- | --- |
| GPIO0 | ADC2\_CH1 | RTC\_GPIO11 | Strapping pin |
| GPIO1 |  |  | TXD |
| GPIO2 | ADC2\_CH2 | RTC\_GPIO12 | Strapping pin |
| GPIO3 |  |  | RXD |
| GPIO4 | ADC2\_CH0 | RTC\_GPIO10 |  |
| GPIO5 |  |  | Strapping pin |
| GPIO6 |  |  | SPI0/1 |
| GPIO7 |  |  | SPI0/1 |
| GPIO8 |  |  | SPI0/1 |
| GPIO9 |  |  | SPI0/1 |
| GPIO10 |  |  | SPI0/1 |
| GPIO11 |  |  | SPI0/1 |
| GPIO12 | ADC2\_CH5 | RTC\_GPIO15 | Strapping pin; JTAG |
| GPIO13 | ADC2\_CH4 | RTC\_GPIO14 | JTAG |
| GPIO14 | ADC2\_CH6 | RTC\_GPIO16 | JTAG |
| GPIO15 | ADC2\_CH3 | RTC\_GPIO13 | Strapping pin; JTAG |
| GPIO16 |  |  | SPI0/1 |
| GPIO17 |  |  | SPI0/1 |
| GPIO18 |  |  |  |
| GPIO19 |  |  |  |
| GPIO21 |  |  |  |
| GPIO22 |  |  |  |
| GPIO23 |  |  |  |
| GPIO25 | ADC2\_CH8 | RTC\_GPIO6 |  |
| GPIO26 | ADC2\_CH9 | RTC\_GPIO7 |  |
| GPIO27 | ADC2\_CH7 | RTC\_GPIO17 |  |
| GPIO32 | ADC1\_CH4 | RTC\_GPIO9 |  |
| GPIO33 | ADC1\_CH5 | RTC\_GPIO8 |  |
| GPIO34 | ADC1\_CH6 | RTC\_GPIO4 | GPI |
| GPIO35 | ADC1\_CH7 | RTC\_GPIO5 | GPI |
| GPIO36 | ADC1\_CH0 | RTC\_GPIO0 | GPI |
| GPIO37 | ADC1\_CH1 | RTC\_GPIO1 | GPI |
| GPIO38 | ADC1\_CH2 | RTC\_GPIO2 | GPI |
| GPIO39 | ADC1\_CH3 | RTC\_GPIO3 | GPI |

Note

- Strapping pin: GPIO0, GPIO2, GPIO5, GPIO12 (MTDI), and GPIO15 (MTDO) are strapping pins. For more information, please refer to [ESP32 datasheet](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf).
- SPI0/1: GPIO6-11 and GPIO16-17 are usually connected to the SPI flash and PSRAM integrated on the module and therefore should not be used for other purposes.
- JTAG: GPIO12-15 are usually used for inline debug.
- GPI: GPIO34-39 can only be set as input mode and do not have software-enabled pullup or pulldown functions.
- TXD & RXD are usually used for flashing and debugging.
- ADC2: ADC2 pins cannot be used when Wi-Fi is used. So, if you are having trouble getting the value from an ADC2 GPIO while using Wi-Fi, you may consider using an ADC1 GPIO instead, which should solve your problem. For more details, please refer to [Hardware Limitations of ADC Continuous Mode](adc/adc_continuous.html#hardware-limitations-adc-continuous) and [Hardware Limitations of ADC Oneshot Mode](adc/adc_oneshot.html#hardware-limitations-adc-oneshot).
- Please do not use the interrupt of GPIO36 and GPIO39 when using ADC or Wi-Fi and Bluetooth with sleep mode enabled. Please refer to [ESP32 ECO and Workarounds for Bugs](https://espressif.com/sites/default/files/documentation/eco_and_workarounds_for_bugs_in_esp32_en.pdf) > GPIO-3.11 for the detailed description of the issue.

There is also separate "RTC GPIO" support, which functions when GPIOs are routed to the "RTC" low-power and analog subsystem. These pin functions can be used when:

## IO Configuration

An IO can be used in two ways:

- As a simple GPIO input to read the level on the pin, or as a simple GPIO output to output the desired level on the pin.
- As a peripheral signal input/output.

IDF peripheral drivers always take care of the necessary IO configurations that need to be applied onto the pins, so that they can be used as the peripheral signal inputs or outputs. This means the users usually only need to be responsible for configuring the IOs as simple inputs or outputs. [`gpio_config()`](#_CPPv411gpio_configPK13gpio_config_t "gpio_config") is an all-in-one API that can be used to configure the I/O mode, internal pull-up/pull-down resistors, etc. for pins, including the ones reused by the USB PHY.

In some applications, an IO pin can serve dual purposes. For example, the IO, which outputs a LEDC PWM signal, can also act as a GPIO input to generate interrupts or GPIO ETM events. Careful handling on the configuration step is necessary for such dual use of IO pins cases. [`gpio_config()`](#_CPPv411gpio_configPK13gpio_config_t "gpio_config") is an API that overwrites all the current configurations, so it must be called to set the pin mode to [`gpio_mode_t::GPIO_MODE_INPUT`](#_CPPv4N11gpio_mode_t15GPIO_MODE_INPUTE "gpio_mode_t::GPIO_MODE_INPUT") before calling the LEDC driver API which connects the output signal to the pin. As an alternative, if no other configuration is needed other than making the pin input enabled, [`gpio_input_enable()`](#_CPPv417gpio_input_enable10gpio_num_t "gpio_input_enable") can be the one to call at any time to achieve the same purpose.

## Check Current Configuration of IOs

GPIO driver offers a dump function [`gpio_dump_io_configuration()`](#_CPPv426gpio_dump_io_configurationP4FILE8uint64_t "gpio_dump_io_configuration") to show the current configurations of IOs, such as pull-up/pull-down, input/output enable, pin mapping, etc. Below is an example of how to dump the configuration of GPIO4, GPIO18, and GPIO26:

```
gpio_dump_io_configuration(stdout, (1ULL << 4) | (1ULL << 18) | (1ULL << 26));
```

The dump will be like this:

```
================IO DUMP Start================
IO[4] -
  Pullup: 1, Pulldown: 0, DriveCap: 2
  InputEn: 1, OutputEn: 0, OpenDrain: 0
  FuncSel: 1 (GPIO)
  GPIO Matrix SigIn ID: (simple GPIO input)
  SleepSelEn: 1

IO[18] -
  Pullup: 0, Pulldown: 0, DriveCap: 2
  InputEn: 0, OutputEn: 1, OpenDrain: 0
  FuncSel: 1 (GPIO)
  GPIO Matrix SigOut ID: 256 (simple GPIO output)
  SleepSelEn: 1

IO[26] **RESERVED** -
  Pullup: 1, Pulldown: 0, DriveCap: 2
  InputEn: 1, OutputEn: 0, OpenDrain: 0
  FuncSel: 0 (IOMUX)
  SleepSelEn: 1

=================IO DUMP End==================
```

In addition, if you would like to dump the configurations of all IOs, you can use:

```
gpio_dump_io_configuration(stdout, SOC_GPIO_VALID_GPIO_MASK);
```

If an IO pin is routed to a peripheral signal through the GPIO matrix, the signal ID printed in the dump information is defined in the [soc/esp32/include/soc/gpio\_sig\_map.h](https://github.com/espressif/esp-idf/blob/35e0c8a3/components/soc/esp32/include/soc/gpio_sig_map.h) header file. The word `**RESERVED**` indicates the IO is occupied by either SPI flash or PSRAM. It is strongly not recommended to reconfigure them for other application purposes.

Do not rely on the default configurations values in the Technical Reference Manual, because it may be changed in the bootloader or application startup code before app\_main.

## API Reference - Normal GPIO

### Functions

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_config(const [gpio\_config\_t](#_CPPv413gpio_config_t "gpio_config_t") \*pGPIOConfig)
:   GPIO common configuration.

    ```
       Configure GPIO's Mode,pull-up,PullDown,IntrType
    ```

    Note

    This function always overwrite all the current IO configurations

    Parameters:
    :   **pGPIOConfig** -- Pointer to GPIO configure struct

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_reset\_pin(gpio\_num\_t gpio\_num)
:   Reset a GPIO to a certain state (select gpio function, enable pullup and disable input and output).

    Parameters:
    :   **gpio\_num** -- GPIO number.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_set\_intr\_type(gpio\_num\_t gpio\_num, [gpio\_int\_type\_t](#_CPPv415gpio_int_type_t "gpio_int_type_t") intr\_type)
:   GPIO set interrupt trigger type.

    Parameters:
    :   - **gpio\_num** -- GPIO number. If you want to set the trigger type of e.g. of GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);
        - **intr\_type** -- Interrupt type, select from gpio\_int\_type\_t

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_intr\_enable(gpio\_num\_t gpio\_num)
:   Enable GPIO module interrupt signal.

    Note

    ESP32: Please do not use the interrupt of GPIO36 and GPIO39 when using ADC or Wi-Fi and Bluetooth with sleep mode enabled. Please refer to the comments of `adc1_get_raw`. Please refer to GPIO-3.11 of [ESP32 ECO and Workarounds for Bugs](https://espressif.com/sites/default/files/documentation/eco_and_workarounds_for_bugs_in_esp32_en.pdf) for the description of this issue.

    Parameters:
    :   **gpio\_num** -- GPIO number. If you want to enable an interrupt on e.g. GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_intr\_disable(gpio\_num\_t gpio\_num)
:   Disable GPIO module interrupt signal.

    Note

    This function is allowed to be executed when Cache is disabled within ISR context, by enabling `CONFIG_GPIO_CTRL_FUNC_IN_IRAM`

    Parameters:
    :   **gpio\_num** -- GPIO number. If you want to disable the interrupt of e.g. GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_set\_level(gpio\_num\_t gpio\_num, uint32\_t level)
:   GPIO set output level.

    Note

    This function is allowed to be executed when Cache is disabled within ISR context, by enabling `CONFIG_GPIO_CTRL_FUNC_IN_IRAM`

    Parameters:
    :   - **gpio\_num** -- GPIO number. If you want to set the output level of e.g. GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);
        - **level** -- Output level. 0: low ; 1: high

    Returns:

int gpio\_get\_level(gpio\_num\_t gpio\_num)
:   GPIO get input level.

    Warning

    If the pad is not configured for input (or input and output) the returned value is always 0.

    Parameters:
    :   **gpio\_num** -- GPIO number. If you want to get the logic level of e.g. pin GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_set\_direction(gpio\_num\_t gpio\_num, [gpio\_mode\_t](#_CPPv411gpio_mode_t "gpio_mode_t") mode)
:   GPIO set direction.

    Configure GPIO mode,such as output\_only,input\_only,output\_and\_input

    Note

    This function always overwrite all the current modes that have applied on the IO pin

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_input\_enable(gpio\_num\_t gpio\_num)
:   Enable input for an IO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_set\_pull\_mode(gpio\_num\_t gpio\_num, [gpio\_pull\_mode\_t](#_CPPv416gpio_pull_mode_t "gpio_pull_mode_t") pull)
:   Configure GPIO internal pull-up/pull-down resistors.

    Note

    This function always overwrite the current pull-up/pull-down configurations

    Note

    ESP32: Only pins that support both input & output have integrated pull-up and pull-down resistors. Input-only GPIOs 34-39 do not.

    Parameters:
    :   - **gpio\_num** -- GPIO number. If you want to set pull up or down mode for e.g. GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);
        - **pull** -- GPIO pull up/down mode.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_wakeup\_enable(gpio\_num\_t gpio\_num, [gpio\_int\_type\_t](#_CPPv415gpio_int_type_t "gpio_int_type_t") intr\_type)
:   Enable GPIO wake-up function.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_wakeup\_disable(gpio\_num\_t gpio\_num)
:   Disable GPIO wake-up function.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_isr\_register(void (\*fn)(void\*), void \*arg, int intr\_alloc\_flags, [gpio\_isr\_handle\_t](#_CPPv417gpio_isr_handle_t "gpio_isr_handle_t") \*handle)
:   Register GPIO interrupt handler, the handler is an ISR. The handler will be attached to the same CPU core that this function is running on.

    This ISR function is called whenever any GPIO interrupt occurs. See the alternative gpio\_install\_isr\_service() and gpio\_isr\_handler\_add() API in order to have the driver support per-GPIO ISRs.

    To disable or remove the ISR, pass the returned handle to the [interrupt allocation functions](../system/intr_alloc.html).

    Parameters:
    :   - **fn** -- Interrupt handler function.
        - **arg** -- Parameter for handler function
        - **intr\_alloc\_flags** -- Flags used to allocate the interrupt. One or multiple (ORred) ESP\_INTR\_FLAG\_\* values. See esp\_intr\_alloc.h for more info.
        - **handle** -- Pointer to return handle. If non-NULL, a handle for the interrupt will be returned here.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_pullup\_en(gpio\_num\_t gpio\_num)
:   Enable pull-up on GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_pullup\_dis(gpio\_num\_t gpio\_num)
:   Disable pull-up on GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_pulldown\_en(gpio\_num\_t gpio\_num)
:   Enable pull-down on GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_pulldown\_dis(gpio\_num\_t gpio\_num)
:   Disable pull-down on GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_output\_enable(gpio\_num\_t gpio\_num)
:   Enable output for an IO (as a simple GPIO output)

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_output\_disable(gpio\_num\_t gpio\_num)
:   Disable output for an IO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_od\_enable(gpio\_num\_t gpio\_num)
:   Enable open-drain for an IO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_od\_disable(gpio\_num\_t gpio\_num)
:   Disable open-drain for an IO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_install\_isr\_service(int intr\_alloc\_flags)
:   Install the GPIO driver's ETS\_GPIO\_INTR\_SOURCE ISR handler service, which allows per-pin GPIO interrupt handlers.

    This function is incompatible with gpio\_isr\_register() - if that function is used, a single global ISR is registered for all GPIO interrupts. If this function is used, the ISR service provides a global GPIO ISR and individual pin handlers are registered via the gpio\_isr\_handler\_add() function.

    Parameters:
    :   **intr\_alloc\_flags** -- Flags used to allocate the interrupt. One or multiple (ORred) ESP\_INTR\_FLAG\_\* values. See esp\_intr\_alloc.h for more info.

    Returns:
    :   - ESP\_OK Success
        - ESP\_ERR\_NO\_MEM No memory to install this service
        - ESP\_ERR\_INVALID\_STATE ISR service already installed.
        - ESP\_ERR\_NOT\_FOUND No free interrupt found with the specified flags
        - ESP\_ERR\_INVALID\_ARG GPIO error

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_uninstall\_isr\_service(void)
:   Uninstall the driver's GPIO ISR service, freeing related resources.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_isr\_handler\_add(gpio\_num\_t gpio\_num, [gpio\_isr\_t](#_CPPv410gpio_isr_t "gpio_isr_t") isr\_handler, void \*args)
:   Add ISR handler for the corresponding GPIO pin.

    Call this function after using gpio\_install\_isr\_service() to install the driver's GPIO ISR handler service.

    The pin ISR handlers no longer need to be declared with IRAM\_ATTR, unless you pass the ESP\_INTR\_FLAG\_IRAM flag when allocating the ISR in gpio\_install\_isr\_service().

    This ISR handler will be called from an ISR. So there is a stack size limit (configurable as "ISR stack size" in menuconfig). This limit is smaller compared to a global GPIO interrupt handler due to the additional level of indirection.

    Parameters:


    Returns:
    :   - ESP\_OK Success
        - ESP\_ERR\_INVALID\_STATE Wrong state, the ISR service has not been initialized.
        - ESP\_ERR\_INVALID\_ARG Parameter error

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_isr\_handler\_remove(gpio\_num\_t gpio\_num)
:   Remove ISR handler for the corresponding GPIO pin.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:
    :   - ESP\_OK Success
        - ESP\_ERR\_INVALID\_STATE Wrong state, the ISR service has not been initialized.
        - ESP\_ERR\_INVALID\_ARG Parameter error

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_set\_drive\_capability(gpio\_num\_t gpio\_num, [gpio\_drive\_cap\_t](#_CPPv416gpio_drive_cap_t "gpio_drive_cap_t") strength)
:   Set GPIO pad drive capability.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_get\_drive\_capability(gpio\_num\_t gpio\_num, [gpio\_drive\_cap\_t](#_CPPv416gpio_drive_cap_t "gpio_drive_cap_t") \*strength)
:   Get GPIO pad drive capability.

    Parameters:
    :   - **gpio\_num** -- GPIO number, only support output GPIOs
        - **strength** -- Pointer to accept drive capability of the pad

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_hold\_en(gpio\_num\_t gpio\_num)
:   Enable gpio pad hold function.

    When a GPIO is set to hold, its state is latched at that moment and will not change when the internal signal or the IO MUX/GPIO configuration is modified (including input enable, output enable, output value, function, and drive strength values). This function can be used to retain the state of GPIOs when the power domain of where GPIO/IOMUX belongs to becomes off. For example, chip or system is reset (e.g. watchdog time-out, Deep-sleep events are triggered), or peripheral power-down in Light-sleep.

    This function works in both input and output modes, and only applicable to output-capable GPIOs. If this function is enabled: in output mode: the output level of the GPIO will be locked and can not be changed. in input mode: the input read value can still reflect the changes of the input signal.

    Power down or call `gpio_hold_dis` will disable this function.

    Please be aware that,

    1. USB pads cannot hold at low level after waking up from Deep-sleep. The USB related registers are reset, so the USB pull-up is back.
    2. For ESP32-P4 rev < 3.0, the states of IOs can not be hold after waking up from Deep-sleep.
    3. For ESP32/S2/C3/S3/C2, this function cannot be used to hold the state of a digital GPIO during Deep-sleep. Even if this function is enabled, the digital GPIO will be reset to its default state when the chip wakes up from Deep-sleep. If you want to hold the state of a digital GPIO during Deep-sleep, please call `gpio_deep_sleep_hold_en`.

    Parameters:
    :   **gpio\_num** -- GPIO number, only support output-capable GPIOs

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_hold\_dis(gpio\_num\_t gpio\_num)
:   Disable gpio pad hold function.

    When the chip is woken up from peripheral power-down sleep, the gpio will be set to the default mode, so, the gpio will output the default level if this function is called. If you don't want the level changes, the gpio should be configured to a known state before this function is called. e.g. If you hold gpio18 high during Deep-sleep, after the chip is woken up and `gpio_hold_dis` is called, gpio18 will output low level(because gpio18 is input mode by default). If you don't want this behavior, you should configure gpio18 as output mode and set it to high level before calling `gpio_hold_dis`.

    Parameters:
    :   **gpio\_num** -- GPIO number, only support output-capable GPIOs

    Returns:

void gpio\_deep\_sleep\_hold\_en(void)
:   Enable all digital gpio pads hold function during Deep-sleep.

    Enabling this feature makes all digital gpio pads be at the holding state during Deep-sleep. The state of each pad holds is its active configuration (not pad's sleep configuration!).

    Note:

    1. For digital IO, this API takes effect only if the corresponding digital IO pad hold function has been enabled. You can enable the GPIO pad hold function by calling `gpio_hold_en`. has been enabled. You can call `gpio_hold_en` to enable the gpio pad hold function.
    2. Though this API targets all digital IOs, the pad hold feature only works when the chip is in Deep-sleep mode. When the chip is in active mode, the digital GPIO state can be changed freely even if you have called this function, except for IOs that are already held by `gpio_hold_en`.

    After this API is being called, the digital gpio Deep-sleep hold feature will work during every sleep process. You should call `gpio_deep_sleep_hold_dis` to disable this feature.

void gpio\_deep\_sleep\_hold\_dis(void)
:   Disable all digital gpio pads hold function during Deep-sleep.

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_sleep\_sel\_en(gpio\_num\_t gpio\_num)
:   SOC\_GPIO\_SUPPORT\_HOLD\_SINGLE\_IO\_IN\_DSLP.

    Enable SLP\_SEL to change GPIO status automantically in lightsleep.

    Parameters:
    :   **gpio\_num** -- GPIO number of the pad.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_sleep\_sel\_dis(gpio\_num\_t gpio\_num)
:   Disable SLP\_SEL to change GPIO status automantically in lightsleep.

    Parameters:
    :   **gpio\_num** -- GPIO number of the pad.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_sleep\_set\_direction(gpio\_num\_t gpio\_num, [gpio\_mode\_t](#_CPPv411gpio_mode_t "gpio_mode_t") mode)
:   GPIO set direction at sleep.

    Configure GPIO direction,such as output\_only,input\_only,output\_and\_input

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_sleep\_set\_pull\_mode(gpio\_num\_t gpio\_num, [gpio\_pull\_mode\_t](#_CPPv416gpio_pull_mode_t "gpio_pull_mode_t") pull)
:   Configure GPIO pull-up/pull-down resistors at sleep.

    Note

    ESP32: Only pins that support both input & output have integrated pull-up and pull-down resistors. Input-only GPIOs 34-39 do not.

    Parameters:
    :   - **gpio\_num** -- GPIO number. If you want to set pull up or down mode for e.g. GPIO16, gpio\_num should be GPIO\_NUM\_16 (16);
        - **pull** -- GPIO pull up/down mode.

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_dump\_io\_configuration(FILE \*out\_stream, uint64\_t io\_bit\_mask)
:   Dump IO configuration information to console.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") gpio\_get\_io\_config(gpio\_num\_t gpio\_num, [gpio\_io\_config\_t](#_CPPv416gpio_io_config_t "gpio_io_config_t") \*out\_io\_config)
:   Get the configuration for an IO.

    Parameters:


    Returns:

### Structures

struct gpio\_config\_t
:   Configuration parameters of GPIO pad for gpio\_config function.

### Type Definitions

typedef [intr\_handle\_t](../system/intr_alloc.html#_CPPv413intr_handle_t "intr_handle_t") gpio\_isr\_handle\_t

typedef void (\*gpio\_isr\_t)(void \*arg)
:   GPIO interrupt handler.

    Param arg:
    :   User registered data

### Header File

- [components/esp\_hal\_gpio/include/hal/gpio\_types.h](https://github.com/espressif/esp-idf/blob/35e0c8a3/components/esp_hal_gpio/include/hal/gpio_types.h)
- This header file can be included with:

  > ```
  > #include "hal/gpio_types.h"
  > ```
- This header file is a part of the API provided by the `esp_hal_gpio` component. To declare that your component depends on `esp_hal_gpio`, add the following to your CMakeLists.txt:

  > or
  >
  > ```
  > PRIV_REQUIRES esp_hal_gpio
  > ```

### Structures

struct gpio\_io\_config\_t
:   Structure that contains the configuration of an IO.

    Public Members

    uint32\_t fun\_sel
    :   Value of IOMUX function selection

    uint32\_t sig\_out
    :   Index of the outputting peripheral signal

    [gpio\_drive\_cap\_t](#_CPPv416gpio_drive_cap_t "gpio_drive_cap_t") drv
    :   Value of drive strength

    bool pu
    :   Status of pull-up enabled or not

    bool pd
    :   Status of pull-down enabled or not

    bool ie
    :   Status of input enabled or not

    bool oe
    :   Status of output enabled or not

    bool oe\_ctrl\_by\_periph
    :   True if use output enable signal from peripheral, otherwise False

    bool oe\_inv
    :   Whether the output enable signal is inversed or not

    bool od
    :   Status of open-drain enabled or not

    bool slp\_sel
    :   Status of pin sleep mode enabled or not

### Macros

GPIO\_PIN\_COUNT

GPIO\_IS\_VALID\_GPIO(gpio\_num)
:   Check whether it is a valid GPIO number.

GPIO\_IS\_VALID\_OUTPUT\_GPIO(gpio\_num)
:   Check whether it can be a valid GPIO number of output mode.

GPIO\_IS\_VALID\_DIGITAL\_IO\_PAD(gpio\_num)
:   Check whether it can be a valid digital I/O pad.

GPIO\_PIN\_REG\_0

GPIO\_PIN\_REG\_1

GPIO\_PIN\_REG\_2

GPIO\_PIN\_REG\_3

GPIO\_PIN\_REG\_4

GPIO\_PIN\_REG\_5

GPIO\_PIN\_REG\_6

GPIO\_PIN\_REG\_7

GPIO\_PIN\_REG\_8

GPIO\_PIN\_REG\_9

GPIO\_PIN\_REG\_10

GPIO\_PIN\_REG\_11

GPIO\_PIN\_REG\_12

GPIO\_PIN\_REG\_13

GPIO\_PIN\_REG\_14

GPIO\_PIN\_REG\_15

GPIO\_PIN\_REG\_16

GPIO\_PIN\_REG\_17

GPIO\_PIN\_REG\_18

GPIO\_PIN\_REG\_19

GPIO\_PIN\_REG\_20

GPIO\_PIN\_REG\_21

GPIO\_PIN\_REG\_22

GPIO\_PIN\_REG\_23

GPIO\_PIN\_REG\_24

GPIO\_PIN\_REG\_25

GPIO\_PIN\_REG\_26

GPIO\_PIN\_REG\_27

GPIO\_PIN\_REG\_28

GPIO\_PIN\_REG\_29

GPIO\_PIN\_REG\_30

GPIO\_PIN\_REG\_31

GPIO\_PIN\_REG\_32

GPIO\_PIN\_REG\_33

GPIO\_PIN\_REG\_34

GPIO\_PIN\_REG\_35

GPIO\_PIN\_REG\_36

GPIO\_PIN\_REG\_37

GPIO\_PIN\_REG\_38

GPIO\_PIN\_REG\_39

GPIO\_PIN\_REG\_40

GPIO\_PIN\_REG\_41

GPIO\_PIN\_REG\_42

GPIO\_PIN\_REG\_43

GPIO\_PIN\_REG\_44

GPIO\_PIN\_REG\_45

GPIO\_PIN\_REG\_46

GPIO\_PIN\_REG\_47

GPIO\_PIN\_REG\_48

GPIO\_PIN\_REG\_49

GPIO\_PIN\_REG\_50

GPIO\_PIN\_REG\_51

GPIO\_PIN\_REG\_52

GPIO\_PIN\_REG\_53

GPIO\_PIN\_REG\_54

### Enumerations

enum gpio\_port\_t
:   *Values:*

    enumerator GPIO\_PORT\_0

    enumerator GPIO\_PORT\_MAX

enum gpio\_int\_type\_t
:   *Values:*

    enumerator GPIO\_INTR\_DISABLE
    :   Disable GPIO interrupt

    enumerator GPIO\_INTR\_POSEDGE
    :   GPIO interrupt type : rising edge

    enumerator GPIO\_INTR\_NEGEDGE
    :   GPIO interrupt type : falling edge

    enumerator GPIO\_INTR\_ANYEDGE
    :   GPIO interrupt type : both rising and falling edge

    enumerator GPIO\_INTR\_LOW\_LEVEL
    :   GPIO interrupt type : input low level trigger

    enumerator GPIO\_INTR\_HIGH\_LEVEL
    :   GPIO interrupt type : input high level trigger

    enumerator GPIO\_INTR\_MAX

enum gpio\_mode\_t
:   *Values:*

    enumerator GPIO\_MODE\_DISABLE
    :   GPIO mode : disable input and output

    enumerator GPIO\_MODE\_INPUT
    :   GPIO mode : input only

    enumerator GPIO\_MODE\_OUTPUT
    :   GPIO mode : output only mode

    enumerator GPIO\_MODE\_OUTPUT\_OD
    :   GPIO mode : output only with open-drain mode

    enumerator GPIO\_MODE\_INPUT\_OUTPUT\_OD
    :   GPIO mode : output and input with open-drain mode

    enumerator GPIO\_MODE\_INPUT\_OUTPUT
    :   GPIO mode : output and input mode

enum gpio\_pullup\_t
:   *Values:*

    enumerator GPIO\_PULLUP\_DISABLE
    :   Disable GPIO pull-up resistor

    enumerator GPIO\_PULLUP\_ENABLE
    :   Enable GPIO pull-up resistor

enum gpio\_pulldown\_t
:   *Values:*

    enumerator GPIO\_PULLDOWN\_DISABLE
    :   Disable GPIO pull-down resistor

    enumerator GPIO\_PULLDOWN\_ENABLE
    :   Enable GPIO pull-down resistor

enum gpio\_pull\_mode\_t
:   *Values:*

    enumerator GPIO\_PULLUP\_ONLY
    :   Pad pull up

    enumerator GPIO\_PULLDOWN\_ONLY
    :   Pad pull down

    enumerator GPIO\_PULLUP\_PULLDOWN
    :   Pad pull up + pull down

    enumerator GPIO\_FLOATING
    :   Pad floating

enum gpio\_drive\_cap\_t
:   *Values:*

    enumerator GPIO\_DRIVE\_CAP\_0
    :   Pad drive capability: weak

    enumerator GPIO\_DRIVE\_CAP\_1
    :   Pad drive capability: stronger

    enumerator GPIO\_DRIVE\_CAP\_2
    :   Pad drive capability: medium

    enumerator GPIO\_DRIVE\_CAP\_DEFAULT
    :   Pad drive capability: medium

    enumerator GPIO\_DRIVE\_CAP\_3
    :   Pad drive capability: strongest

    enumerator GPIO\_DRIVE\_CAP\_MAX

## API Reference - RTC GPIO

### Header File

- [components/esp\_driver\_gpio/include/driver/rtc\_io.h](https://github.com/espressif/esp-idf/blob/35e0c8a3/components/esp_driver_gpio/include/driver/rtc_io.h)
- This header file can be included with:

  > ```
  > #include "driver/rtc_io.h"
  > ```
- This header file is a part of the API provided by the `esp_driver_gpio` component. To declare that your component depends on `esp_driver_gpio`, add the following to your CMakeLists.txt:

  > or
  >
  > ```
  > PRIV_REQUIRES esp_driver_gpio
  > ```

### Functions

bool rtc\_gpio\_is\_valid\_gpio(gpio\_num\_t gpio\_num)
:   Determine if the specified IO is a valid RTC GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:
    :   true if the IO is valid for RTC GPIO use. false otherwise.

int rtc\_io\_number\_get(gpio\_num\_t gpio\_num)
:   Get RTC IO index number by GPIO number.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:
    :   >=0: Index of RTC IO. -1 : The IO is not an RTC IO.

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_init(gpio\_num\_t gpio\_num)
:   Init an IO to be an RTC GPIO, route to RTC IO MUX.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_deinit(gpio\_num\_t gpio\_num)
:   Deinit an IO as an RTC GPIO, route back to IO MUX.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

uint32\_t rtc\_gpio\_get\_level(gpio\_num\_t gpio\_num)
:   Get the RTC IO input level.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_set\_level(gpio\_num\_t gpio\_num, uint32\_t level)
:   Set the RTC IO output level.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_set\_direction(gpio\_num\_t gpio\_num, [rtc\_gpio\_mode\_t](#_CPPv415rtc_gpio_mode_t "rtc_gpio_mode_t") mode)
:   RTC GPIO set direction.

    Configure RTC GPIO direction, such as output only, input only, output and input.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_set\_direction\_in\_sleep(gpio\_num\_t gpio\_num, [rtc\_gpio\_mode\_t](#_CPPv415rtc_gpio_mode_t "rtc_gpio_mode_t") mode)
:   RTC GPIO set direction in deep sleep mode or disable sleep status (default). In some application scenarios, IO needs to have another states during deep sleep.

    NOTE: ESP32 supports INPUT\_ONLY mode. The rest targets support INPUT\_ONLY, OUTPUT\_ONLY, INPUT\_OUTPUT mode.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_pullup\_en(gpio\_num\_t gpio\_num)
:   RTC GPIO pullup enable.

    This function only works for RTC IOs. In general, call gpio\_pullup\_en, which will work both for normal GPIOs and RTC IOs.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_pulldown\_en(gpio\_num\_t gpio\_num)
:   RTC GPIO pulldown enable.

    This function only works for RTC IOs. In general, call gpio\_pulldown\_en, which will work both for normal GPIOs and RTC IOs.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_pullup\_dis(gpio\_num\_t gpio\_num)
:   RTC GPIO pullup disable.

    This function only works for RTC IOs. In general, call gpio\_pullup\_dis, which will work both for normal GPIOs and RTC IOs.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_pulldown\_dis(gpio\_num\_t gpio\_num)
:   RTC GPIO pulldown disable.

    This function only works for RTC IOs. In general, call gpio\_pulldown\_dis, which will work both for normal GPIOs and RTC IOs.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_set\_drive\_capability(gpio\_num\_t gpio\_num, [gpio\_drive\_cap\_t](#_CPPv416gpio_drive_cap_t "gpio_drive_cap_t") strength)
:   Set RTC GPIO pad drive capability.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_get\_drive\_capability(gpio\_num\_t gpio\_num, [gpio\_drive\_cap\_t](#_CPPv416gpio_drive_cap_t "gpio_drive_cap_t") \*strength)
:   Get RTC GPIO pad drive capability.

    Parameters:
    :   - **gpio\_num** -- GPIO number, only support output GPIOs
        - **strength** -- Pointer to accept drive capability of the pad

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_iomux\_func\_sel(gpio\_num\_t gpio\_num, int func)
:   Select a RTC IOMUX function for the RTC IO.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_hold\_en(gpio\_num\_t gpio\_num)
:   Enable hold function on an RTC IO pad.

    Enabling HOLD function will cause the pad to latch current values of input enable, output enable, output value, function, drive strength values. This function is useful when going into light or deep sleep mode to prevent the pin configuration from changing.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_hold\_dis(gpio\_num\_t gpio\_num)
:   Disable hold function on an RTC IO pad.

    Disabling hold function will allow the pad receive the values of input enable, output enable, output value, function, drive strength from RTC\_IO peripheral.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12)

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_force\_hold\_en\_all(void)
:   Enable force hold signal for all RTC IOs.

    Each RTC pad has a "force hold" input signal from the RTC controller. If this signal is set, pad latches current values of input enable, function, output enable, and other signals which come from the RTC mux. Force hold signal is enabled before going into deep sleep for pins which are used for EXT1 wakeup.

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_force\_hold\_dis\_all(void)
:   Disable force hold signal for all RTC IOs.

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_isolate(gpio\_num\_t gpio\_num)
:   Helper function to disconnect internal circuits from an RTC IO This function disables input, output, pullup, pulldown, and enables hold feature for an RTC IO. Use this function if an RTC IO needs to be disconnected from internal circuits in deep sleep, to minimize leakage current.

    In particular, for ESP32-WROVER module, call rtc\_gpio\_isolate(GPIO\_NUM\_12) before entering deep sleep, to reduce deep sleep current.

    Parameters:
    :   **gpio\_num** -- GPIO number (e.g. GPIO\_NUM\_12).

    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_wakeup\_enable(gpio\_num\_t gpio\_num, [gpio\_int\_type\_t](#_CPPv415gpio_int_type_t "gpio_int_type_t") intr\_type)
:   Enable wakeup from sleep mode using specific GPIO.

    Parameters:


    Returns:

[esp\_err\_t](../system/esp_err.html#_CPPv49esp_err_t "esp_err_t") rtc\_gpio\_wakeup\_disable(gpio\_num\_t gpio\_num)
:   Disable wakeup from sleep mode using specific GPIO.

    Parameters:
    :   **gpio\_num** -- GPIO number

    Returns:

### Macros

RTC\_GPIO\_IS\_VALID\_GPIO(gpio\_num)

### Header File

- [components/esp\_driver\_gpio/include/driver/lp\_io.h](https://github.com/espressif/esp-idf/blob/35e0c8a3/components/esp_driver_gpio/include/driver/lp_io.h)
- This header file can be included with:

  > ```
  > #include "driver/lp_io.h"
  > ```
- This header file is a part of the API provided by the `esp_driver_gpio` component. To declare that your component depends on `esp_driver_gpio`, add the following to your CMakeLists.txt:

  > or
  >
  > ```
  > PRIV_REQUIRES esp_driver_gpio
  > ```

### Header File

- [components/esp\_hal\_gpio/include/hal/rtc\_io\_types.h](https://github.com/espressif/esp-idf/blob/35e0c8a3/components/esp_hal_gpio/include/hal/rtc_io_types.h)
- This header file can be included with:

  > ```
  > #include "hal/rtc_io_types.h"
  > ```
- This header file is a part of the API provided by the `esp_hal_gpio` component. To declare that your component depends on `esp_hal_gpio`, add the following to your CMakeLists.txt:

  > or
  >
  > ```
  > PRIV_REQUIRES esp_hal_gpio
  > ```

### Enumerations

enum rtc\_gpio\_mode\_t
:   RTCIO output/input mode type.

    *Values:*

    enumerator RTC\_GPIO\_MODE\_INPUT\_ONLY
    :   Pad input

    enumerator RTC\_GPIO\_MODE\_OUTPUT\_ONLY
    :   Pad output

    enumerator RTC\_GPIO\_MODE\_INPUT\_OUTPUT
    :   Pad input + output

    enumerator RTC\_GPIO\_MODE\_DISABLED
    :   Pad (output + input) disable

    enumerator RTC\_GPIO\_MODE\_OUTPUT\_OD
    :   Pad open-drain output

    enumerator RTC\_GPIO\_MODE\_INPUT\_OUTPUT\_OD
    :   Pad input + open-drain output