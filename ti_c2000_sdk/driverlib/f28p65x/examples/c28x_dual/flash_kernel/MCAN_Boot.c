//###########################################################################
//
// FILE:    MCAN_Boot.c
//
// TITLE:   MCAN (CAN-FD) Bootloader
//
// Functions involved in running MCAN bootloader
//
// ---------------------------------------------------
// |Opt No.|  BOOTDEF      |  CANATXA   |  CANARXA   |
// ---------------------------------------------------
// |  0    |  0x08(default)|  04        |  10        |
// |  1    |  0x18         |  08        |  10        |
// |  2    |  0x28         |  19        |  18        |
// |  3    |  0x38         |  04        |  05        |
// |  4    |  0x48         |  74        |  75        |
// |  5    |  0x58 (TEST)  |  04        |  10        |
// |  6    |  0x68 (TEST)  |  08        |  10        |
// |  7    |  0x78 (TEST)  |  19        |  18        |
// |  8    |  0x88 (TEST)  |  04        |  05        |
// |  9    |  0x98 (TEST)  |  74        |  75        |
// ---------------------------------------------------
//
//###########################################################################
// $TI Release: $
// 
// $Copyright:
// Copyright (C) 2022 Texas Instruments Incorporated - http://www.ti.com
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//   Redistributions of source code must retain the above copyright
//   notice, this list of conditions and the following disclaimer.
//
//   Redistributions in binary form must reproduce the above copyright
//   notice, this list of conditions and the following disclaimer in the
//   documentation and/or other materials provided with the
//   distribution.
//
//   Neither the name of Texas Instruments Incorporated nor the names of
//   its contributors may be used to endorse or promote products derived
//   from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
// $
//###########################################################################

//
// Included Files
//
#include "cpu1bootrom.h"
#include "cpu1brom_utils.h"
#include "inc/hw_mcan.h"
#include "mcan.h"

#include "FlashTech_F28P65x_C28x.h"
#include "flash_programming_f28p65x.h"
#include "f28p65x_kernel_commands_cpu1.h"

//from hw_memmap.h
#define MCAN_MSG_RAM_BASE         0x00059000U

#define BLANK_ERROR                 0x2000
#define VERIFY_ERROR                0x3000
#define PROGRAM_ERROR               0x4000

#define BUFFER_SIZE               0x200
//
// Polling Timeout Define
// (Values in uS)
//
#define MCAN_POLLING_TIMEOUT              11000UL      // ~11ms
#define MCAN_POLLING_FIRST_MSG_TIMEOUT    10000000UL  // ~10sec
#define MCAN_LOOP_CYCLE_SCALER            20UL        // Value indicating "average" CPU cycles
                                                      // per while loop which includes a 32b
                                                      // comparison and counter increment
#define MCAN_FIRST_MSG_LOOP_CYCLE_SCALER  25UL        // Value indicating "average" CPU cycles
                                                      // per while loop which includes a 32b
                                                      // comparison,counter increment, and
                                                      // register read

//
// MCAN Clocking and Bit Rate Defines
// (Values for NBTR and DBTR registers)
//
#define MCAN_NOM_SJW                1UL
#define MCAN_NOM_BRP                1UL
#define MCAN_NOM_TSEG1              6UL
#define MCAN_NOM_TSEG2              1UL

#define MCAN_DATA_SJW               0UL
#define MCAN_DATA_BRP               1UL
#define MCAN_DATA_TSEG1             2UL
#define MCAN_DATA_TSEG2             0UL

// Nominal phase bit timing of 1Mbps at 20MHz MCAN clock
#define MCAN_NOM_BIT_TIMING_VALUE   (((uint32_t)MCAN_NOM_SJW << MCAN_NBTP_NSJW_S) | \
                                     ((uint32_t)MCAN_NOM_BRP << MCAN_NBTP_NBRP_S) | \
                                     ((uint32_t)MCAN_NOM_TSEG1 << MCAN_NBTP_NTSEG1_S) | \
                                     ((uint32_t)MCAN_NOM_TSEG2 << MCAN_NBTP_NTSEG2_S))

// Data phase bit timing of 2Mbps at 20MHz MCAN clock
#define MCAN_DATA_BIT_TIMING_VALUE  (((uint32_t)MCAN_DATA_SJW << MCAN_DBTP_DSJW_S) | \
                                     ((uint32_t)MCAN_DATA_BRP << MCAN_DBTP_DBRP_S) | \
                                     ((uint32_t)MCAN_DATA_TSEG1 << MCAN_DBTP_DTSEG1_S) | \
                                     ((uint32_t)MCAN_DATA_TSEG2 << MCAN_DBTP_DTSEG2_S))

//
// Device Clock Configure Defines
//
#define MCAN_CLOCKSRC_SKIP_XTAL_CONFIG 0U
#define MCAN_CLOCKSRC_XTAL             1U
#define MCAN_CLOCKSRC_AUXCLKIN         2U
#define MCAN_DIVIDER_BY_1              0U
#define DEVICE_X1_COUNT                (SYSCTL_X1CNT_X1CNT_M << SYSCTL_X1CNT_X1CNT_S)
#define DEVICE_CLOCK_SRC_XTAL          1UL
#define DEVICE_SYS_CLOCK_DIV_1         0U

//
// MCAN module configuration details
//
#define MCAN_MODULE_FDMODE_ENABLED         0x1UL
#define MCAN_MODULE_BRS_ENABLED            0x1UL
#define MCAN_MODULE_WAKEUP_ENABLED         0x1UL
#define MCAN_MODULE_AUTO_WAKEUP_ENABLED    0x1UL
#define MCAN_MODULE_TX_DELAY_COMP_ENABLED  0x1UL
#define MCAN_MODULE_MSG_RAM_WD_DISABLED    0x0UL
#define MCAN_MODULE_TX_DELAY_WINDOW_LENGTH 0x2UL
#define MCAN_MODULE_TX_DELAY_OFFSET        0x4UL

//
// MCAN operating configuration details
//
#define MCAN_CONFIG_BUS_MONITORING_DISABLED 0x0UL
#define MCAN_CONFIG_OP_MODE_NORMAL          0x0UL
#define MCAN_CONFIG_TIMESTAMP_DISABLED      0x0UL
#define MCAN_CONFIG_TIMEOUT_DISABLED        0x0UL
#define MCAN_CONFIG_REJECT_REMOTE_FRAMES    1UL
#define MCAN_CONFIG_REJECT_NONMATCH_FRAMES  2UL

//
// MCAN message RAM configuration details
//
#define APP_MCAN_STD_ID_FILT_START_ADDR          (0x0UL)
#define APP_MCAN_STD_ID_FILTER_NUM               (1UL)
#define APP_MCAN_EXT_ID_FILT_START_ADDR          (0x14UL)
#define APP_MCAN_EXT_ID_FILTER_NUM               (0UL)
#define APP_MCAN_TX_BUFF_START_ADDR              (0x80UL)
#define APP_MCAN_TX_BUFF_SIZE                    (2UL)
#define APP_MCAN_FIFO_1_START_ADDR               (0xC0UL)
#define APP_MCAN_FIFO_1_NUM                      (0UL)
#define APP_MCAN_TX_EVENT_START_ADDR             (0x100UL)
#define APP_MCAN_RX_BUFF_START_ADDR              (0x300UL)
#define APP_MCAN_FIFO_0_START_ADDR               (548UL)
#define APP_MCAN_FIFO_0_NUM                      (0UL)
#define APP_MCAN_FIFO_WATERMARK_DISABLED         (0UL)
#define APP_MCAN_FIFO_BLOCKING_MODE              (0UL)
#define APP_MCAN_FIFO_DISABLED                   (0UL)

//
// MCAN message filter configuration details
//
#define MCAN_RX_MSG_FILTER_ID              1U
#define MCAN_RX_MSG_FILTER_STORE_IN_BUF    0x7U
#define MCAN_RX_MSG_FILTER_2ND_ID_DISABLED 0x0U

//
// Tx/RX Message Buffers
// (TX ID = 2, RX ID = 1)
//
#define MCAN_TX_MSG_ID                  0x2UL
#define MCAN_RX_MSG_BUF0                0x0UL
#define MCAN_TX_MSG_BUF0                0x0UL
#define MCAN_TX_MSG_BUF1                0x1UL
#define MCAN_STD_ID_SHIFT               18U
#define MCAN_MSG_DLC_64BYTES            15UL
#define MCAN_TX_MSG_MARKER              0xAFUL
#define MCAN_TX_BITRATESWITCHING_ENABLE 1UL
#define MCAN_MSG_FD_FORMAT_ENABLE       1UL
#define MCAN_MSG_EVENT_FIFO_ENABLE      1UL

//
// Rx Message Buffer Indexes
//
#define MCAN_MSG_BUFFER_MAX_SIZE    64U

#define LOWER_KEY_OFFSET            0U
#define UPPER_KEY_OFFSET            1U

#define LOWER_BYTE1_NOM_TIMING      4U
#define LOWER_BYTE2_NOM_TIMING      5U
#define UPPER_BYTE1_NOM_TIMING      2U
#define UPPER_BYTE2_NOM_TIMING      3U

#define LOWER_BYTE1_DATA_TIMING     8U
#define LOWER_BYTE2_DATA_TIMING     9U
#define UPPER_BYTE1_DATA_TIMING     6U
#define UPPER_BYTE2_DATA_TIMING     7U

#define LOWER_BYTE1_ENTRY_ADDRESS   20U
#define LOWER_BYTE2_ENTRY_ADDRESS   21U
#define UPPER_BYTE1_ENTRY_ADDRESS   18U
#define UPPER_BYTE2_ENTRY_ADDRESS   19U

#define LOWER_FIRST_BLOCK_SIZE      22U
#define UPPER_FIRST_BLOCK_SIZE      23U

//
// Misc
//
#define MCAN_BYTE_MASK                0xFFU
#define MCAN_DWORD_SHIFT              16U
#define MCAN_2ND_WORD_INDEX           2U
#define MCAN_API_PASS                 ((int32_t) 0)
#define MCAN_PERIPHERAL_NOT_PRESENT   ((bool)false)
#define MCAN_RAM_INIT_NOT_DONE        ((uint32_t)0UL)
#define TX_MSG_PENDING                0UL

typedef struct
{
   uint16_t status;
   uint32_t address;
   uint16_t flashAPIError;
   uint32_t flashAPIFsmStatus;
}  StatusCode;
StatusCode statusCode;


//
// Globals
//
static MCAN_RxBufElement rxMsg;
static uint32_t mcanPollingTimeout;
static uint32_t mcanPollingFirstMsgTimeout;
static uint32_t mcanDefaultNominalBitTimingConfig;
static uint32_t mcanDefaultDataBitTimingConfig;
static uint32_t mcanDebugModeEnabled;
uint16_t msgBufferIndex;
uint32_t pllMultiplier, pllDivider;
const uint16_t sectSize = Sector2KB_u32length;

//
// Function Prototypes
//
static void MCAN_configureGPIOs(uint32_t bootMode);
static void MCAN_bootInitialization(uint32_t nominalBitTiming,
                                                     uint32_t dataBitTiming,
                                                     uint16_t mcanDivider,
                                                     uint16_t clockSelection,
                                                     uint16_t deviceSystemFreq);
static void MCAN_sendTwoMessages(void);
void MCAN_readMessage(uint32_t timeoutValue);
static uint32_t MCAN_receiveApplication(void);
extern void CopyApplication(MCAN_RxBufElement rxMsg);
uint32_t MCAN_getDataFromBuffer(MCAN_dataTypeSize dataTypeSize);
void setFlashAPIError(Fapi_StatusType status);
void exampleError();


/**
* MCAN_Boot - Run the MCAN bootloader setup with the specified GPIOs for the
*             requested MCAN boot mode
*
* Start MCAN Boot
*
* bootMode- the boot mode value selecting the various GPIO configurations for MCAN
* nominalBitTiming -
*                   0: Use bootloader defined nominal bit timing configuration
*            Non-Zero: Use contents of parameter to configure nominal bit timing
*                      (Contents should be value for MCAN NBTP register)
* dataBitTiming -
*                   0: Use bootloader defined data bit timing configuration
*            Non-Zero: Use contents of parameter to configure data bit timing
*                      (Contents should be value for MCAN DBTP register)
* deviceSystemFreq - The device system frequency in MHz
*                    (ex: 20 for 20MHz or 200 for 200MHz)
* mcanDivider - The value for the MCAN divider
*                   0 = /1
*                   1 = /2
*                   ...
*                  19 = /20
* clockSelection -
*                   0: Use INTOSC2 as clock source
*                   1: Use XTAL as clock source
*                   2: Use AUXCLKIN as clock source
*
*/
uint32_t MCAN_Boot(uint32_t bootMode, uint32_t nominalBitTiming,
                   uint32_t dataBitTiming, uint16_t deviceSystemFreq,
                   uint16_t mcanDivider, uint16_t clockSelection)
{

    //
    // Initialize globals to zero
    // (Excludes rxMsg as this will be
    //  populated upon message reception)
    //
    mcanPollingTimeout = 0UL;
    mcanPollingFirstMsgTimeout = 0UL;
    mcanDefaultNominalBitTimingConfig = 0UL;
    mcanDefaultDataBitTimingConfig = 0UL;
    mcanDebugModeEnabled = 0UL;


    //
    // Declare and initialize local variables
    //
    uint16_t i = 0;
    uint16_t fail = 0;
    uint32_t bankAddress = 0;

    //
    // Declare Fapi variables
    //
    Fapi_StatusType oReturnCheck;
    Fapi_FlashStatusWordType oFlashStatusWord;
    Fapi_FlashStatusType oFlashStatus;

    //
    // Assign timeout value
    // (Timeout is number of cycles required to meet time
    //  based on the system clock frequency)
    //
    // Formula: CPU cycles = (uS * MHz)/scaler
    //
    mcanPollingTimeout = (MCAN_POLLING_TIMEOUT * deviceSystemFreq)/MCAN_LOOP_CYCLE_SCALER;
    mcanPollingFirstMsgTimeout = (MCAN_POLLING_FIRST_MSG_TIMEOUT * deviceSystemFreq)/MCAN_FIRST_MSG_LOOP_CYCLE_SCALER;

    //
    // Assign default bit timing configuration
    //
    mcanDefaultNominalBitTimingConfig = MCAN_NOM_BIT_TIMING_VALUE;
    mcanDefaultDataBitTimingConfig = MCAN_DATA_BIT_TIMING_VALUE;


    //
    // Set up the GPIO mux for the chosen pinout
    //
    MCAN_configureGPIOs(bootMode);

    //
    // Set the user-provided bit timing register value if one was provided,
    // otherwise maintain default value
    //
    if(nominalBitTiming != 0UL)
    {
        mcanDefaultNominalBitTimingConfig = nominalBitTiming;
    }
    if(dataBitTiming != 0UL)
    {
        mcanDefaultDataBitTimingConfig = dataBitTiming;
    }

    //
    // FAPI initialization
    //
    Fapi_initializeAPI(FlashTech_CPU0_BASE_ADDRESS, 200);
    oReturnCheck = Fapi_setActiveFlashBank(Fapi_FlashBank0);

    if(oReturnCheck != Fapi_Status_Success)
    {
        exampleError();
    }

    //
    // Wait until FSM is done with the previous flash operation
    //
    while (Fapi_checkFsmForReady() != Fapi_Status_FsmReady){}
    oFlashStatus = Fapi_getFsmStatus();
    if(oFlashStatus != 3)
    {

        /* Clear the Status register */
        oReturnCheck = Fapi_issueAsyncCommand(Fapi_ClearStatus);
        //
        // Wait until status is cleared
        //
        while (Fapi_getFsmStatus() != 0) {}

        if(oReturnCheck != Fapi_Status_Success)
        {
            statusCode.status = CLEAR_STATUS_ERROR;
            statusCode.address = 0;
            setFlashAPIError(oReturnCheck);
            statusCode.flashAPIFsmStatus = 0; // not used here
            fail++;
        }
    }

    for(i = 0; i <= 4; i++){
        //
        // Set the bank address
        //
        bankAddress = Bzero_Sector0_start + (i * 0x20000);

        //
        // Disable erase/program protection
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        //
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, 0);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, 0);

        //
        // Erase bank
        //
        oReturnCheck = Fapi_issueBankEraseCommand((uint32 *)bankAddress);

        // Wait until FSM is done with erase bank operation
        while (Fapi_checkFsmForReady() != Fapi_Status_FsmReady)
        {
        }

        oFlashStatus = Fapi_getFsmStatus();

        if(oReturnCheck != Fapi_Status_Success || oFlashStatus != 3)
        {
             //
             // first fail
             //
             statusCode.status = BANK_ERASE_ERROR;
             //
             // Address is not needed for Bank Erase Error
             //
             statusCode.address = 0;
             setFlashAPIError(oReturnCheck);
             statusCode.flashAPIFsmStatus = oFlashStatus;
             fail++;
        }

        //
        // Perform blank check
        //
        oReturnCheck = Fapi_doBlankCheck((uint32_t *) (bankAddress),
                                                          sectSize*128,
                                                          &oFlashStatusWord);

        if (oReturnCheck != Fapi_Status_Success)
        {

            statusCode.status = BLANK_ERROR;
            statusCode.address = oFlashStatusWord.au32StatusWord[0];
            setFlashAPIError(oReturnCheck);
            statusCode.flashAPIFsmStatus = 0;

            fail++;
        }

    }

    // Verify that flash has been erased properly, else stop the program
    if (fail != 0) {
        exampleError();
    }

    //
    // Setup MCAN module, bit rate configuration, and message objects
    // Returns true if successful or false if unsuccessful (timeout)

    MCAN_bootInitialization(mcanDefaultNominalBitTimingConfig,
                               mcanDefaultDataBitTimingConfig,
                               mcanDivider,
                               clockSelection,
                               deviceSystemFreq);

    //
    // Requires debug MCAN bootmode to be configured
    // Testing Only: Send two tests frames if the boot mode says so
    //

    if(bootMode >= MCAN_BOOT_SENDTEST)
    {
        //
        // Enables debug mode which changes
        // waiting for first message to infinite loop
        //
        mcanDebugModeEnabled = 1UL;

        //
        // Note no action taken if error
        // occurs during transmission
        //
        MCAN_sendTwoMessages();
    }

    //
    // Wait to receive first message frame
    //
    MCAN_readMessage(mcanPollingFirstMsgTimeout);

    //
    // Decode bootloader key, reserved words,
    // and application entry address.
    //
    // Begin receiving and copying
    // application to RAM
    //
    // Return application entry address
    //
    return(MCAN_receiveApplication());
}

//
// MCAN_configureGPIOs - Configure the peripheral mux to connect MCAN-A to the
//                       chosen GPIOs
//
static void MCAN_configureGPIOs(uint32_t bootMode)
{
    uint16_t gpioTx;
    uint16_t gpioRx;
    uint32_t gpioTxPinConfig;
    uint32_t gpioRxPinConfig;

    //
    // Unlock the GPIO configuration registers
    //
    GPIO_unlockPortConfig(GPIO_PORT_A,0xFFFFFFFFUL);
    GPIO_unlockPortConfig(GPIO_PORT_B,0xFFFFFFFFUL);
    GPIO_unlockPortConfig(GPIO_PORT_C,0xFFFFFFFFUL);

    //
    // Decode the GPIO selection, then set up the mux and configure the inputs
    // for asynchronous qualification.
    //
    switch (bootMode)
    {

        case MCAN_BOOT_ALT1:
        case MCAN_BOOT_ALT1_SENDTEST:
            //
            // GPIO8 = MCANATX
            // GPIO10 = MCANARX
            //
            gpioTx = 8UL;
            gpioRx = 10UL;

            gpioTxPinConfig = GPIO_8_MCANA_TX;
            gpioRxPinConfig = GPIO_10_MCANA_RX;

            break;

        case MCAN_BOOT_ALT2:
        case MCAN_BOOT_ALT2_SENDTEST:
            //
            // GPIO19 = MCANATX
            // GPIO18 = MCANARX
            //
            gpioTx = 19UL;
            gpioRx = 18UL;

            gpioTxPinConfig = GPIO_19_MCANA_TX;
            gpioRxPinConfig = GPIO_18_MCANA_RX;


            break;

        case MCAN_BOOT_ALT3:
        case MCAN_BOOT_ALT3_SENDTEST:
            //
            // GPIO04 = MCANATX
            // GPIO05 = MCANARX
            //
            gpioTx = 4U;
            gpioRx = 5U;

            gpioTxPinConfig = GPIO_4_MCANA_TX;
            gpioRxPinConfig = GPIO_5_MCANA_RX;


            break;

        case MCAN_BOOT_ALT4:
        case MCAN_BOOT_ALT4_SENDTEST:
            //
            // GPIO74 = MCANATX
            // GPIO75 = MCANARX
            //
            gpioTx = 74UL;
            gpioRx = 75UL;

            gpioTxPinConfig = GPIO_74_MCANA_TX;
            gpioRxPinConfig = GPIO_75_MCANA_RX;


            break;

        case MCAN_BOOT:
        case MCAN_BOOT_SENDTEST:
        default:
            //
            // GPIO04 = MCANATX
            // GPIO10 = MCANARX
            //
            gpioTx = 4U;
            gpioRx = 10U;

            gpioTxPinConfig = GPIO_4_MCANA_TX;
            gpioRxPinConfig = GPIO_10_MCANA_RX;

            break;

    }

    //
    // Enable pull up on GPIOs pins
    //
    GPIO_setPadConfig(gpioTx,GPIO_PIN_TYPE_PULLUP);
    GPIO_setPadConfig(gpioRx,GPIO_PIN_TYPE_PULLUP);

    //
    // Set GPIO configuration for CAN
    //
    GPIO_setPinConfig(gpioTxPinConfig);
    GPIO_setPinConfig(gpioRxPinConfig);

    //
    // Configure GPIOs as async pins
    //
    GPIO_setQualificationMode(gpioTx,GPIO_QUAL_ASYNC);
    GPIO_setQualificationMode(gpioRx,GPIO_QUAL_ASYNC);
}

//
// MCAN_bootInitialization - Initialize the MCAN-A module and configure its bit clock
//                           and message objects
//
static void MCAN_bootInitialization(uint32_t nominalBitTiming,
                                                     uint32_t dataBitTiming,
                                                     uint16_t mcanDivider,
                                                     uint16_t clockSelection,
                                                     uint16_t deviceSystemFreq)
{
    MCAN_InitParams initParams = { 0U };
    MCAN_ConfigParams configParams = { 0U };
    MCAN_MsgRAMConfigParams msgRAMConfigParams = { 0U };
    MCAN_StdMsgIDFilterElement stdMsgFilterParams = { 0U };
    MCAN_BitTimingParams bitTimeParams = { 0U };
    volatile uint32_t timeoutCounter = 0UL;

    EALLOW;

    //
    // Turn off the clock to MCAN module
    //
    HWREG(CPUSYS_BASE + SYSCTL_O_PCLKCR10) &= (uint32_t)~SYSCTL_PCLKCR10_MCAN_A;

    //
    // Select XTAL for CAN clock
    //
    if(clockSelection == MCAN_CLOCKSRC_XTAL)
    {
        //
        // Turn on XTAL and select crystal mode
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_XTALCR) &= (uint16_t)~SYSCTL_XTALCR_OSCOFF;
        NOP_CYCLES(45);
        HWREGH(CLKCFG_BASE + SYSCTL_O_XTALCR) &= (uint16_t)~SYSCTL_XTALCR_SE;

        //
        // Wait for the X1 clock to saturate
        //
        HWREG(CLKCFG_BASE + SYSCTL_O_X1CNT) |= SYSCTL_X1CNT_CLR;
        while(HWREGH(CLKCFG_BASE + SYSCTL_O_X1CNT) != DEVICE_X1_COUNT)
        {
            if(timeoutCounter > mcanPollingTimeout)
            {
                //
                // Turn off XTAL
                //
                HWREGH(CLKCFG_BASE + SYSCTL_O_XTALCR) |= (uint16_t)SYSCTL_XTALCR_OSCOFF;
                EDIS;

                //
                // Exit upon error occurring
                //
                return;
            }
            else
            {
                timeoutCounter = timeoutCounter + 1UL;
            }
        }


        if(deviceSystemFreq == 20U)
        {
            //
            //Configure MULT and ODIV for SYSPLL of 200 MHZ
            //
            CPU1BROM_triggerSysPLLLock(CLK_XTAL, APLL_MULT_20, APLL_DIV_2);
        }

        else if(deviceSystemFreq == 25U)
        {
            //
            //Configure MULT and ODIV for SYSPLL of 200 MHZ
            //
            CPU1BROM_triggerSysPLLLock(CLK_XTAL, APLL_MULT_16, APLL_DIV_2);
        }

        CPU1BROM_switchToPLL(deviceSystemFreq);

        //
        // Set device clock divider to /1
        //
        EALLOW;
        HWREG(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) = DEVICE_SYS_CLOCK_DIV_1;
    }

    //
    // Set MCAN bit clock source to CPU1SYSCLK
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= (uint16_t)~SYSCTL_CLKSRCCTL2_MCANABCLKSEL_M;
    //HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= 0x01;

    //
    // User requested to use AUXCLKIN as MCAN clock source
    //
    if(clockSelection == MCAN_CLOCKSRC_AUXCLKIN)
    {
        HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) |= (uint16_t)MCAN_CLOCKSRC_AUXCLKIN << SYSCTL_CLKSRCCTL2_MCANABCLKSEL_S;
    }

    //
    // Set MCAN divider
    // (By default on boot is /1, so 20MHz XTAL = 20MHz clock to MCAN)
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_AUXCLKDIVSEL) &= (uint16_t)~SYSCTL_AUXCLKDIVSEL_MCANACLKDIV_M;
    NOP_CYCLES(45);
    HWREGH(CLKCFG_BASE + SYSCTL_O_AUXCLKDIVSEL) |= mcanDivider << SYSCTL_AUXCLKDIVSEL_MCANACLKDIV_S;

    //
    // Turn on the clock to the MCAN module
    //
    HWREG(CPUSYS_BASE + SYSCTL_O_PCLKCR10) |= SYSCTL_PCLKCR10_MCAN_A;
    EDIS;

    //
    // Setup MCAN configuration
    //
    // Initialization Details:
    // - FD operation mode
    // - Bit rate switching enabled
    // - Transmit pause disabled
    // - Edge filtering disabled
    // - Protocol exception handing enabled
    // - Automatic retransmission enabled
    // - Wakeup request enabled
    // - Emulation/debug suspend disabled
    // - Transmitter delay compensation enabled (recommended for > 1Mbps)
    // - Message RAM Watchdog counter disabled
    //
    initParams.fdMode         = MCAN_MODULE_FDMODE_ENABLED;
    initParams.brsEnable      = MCAN_MODULE_BRS_ENABLED;
    initParams.wkupReqEnable  = MCAN_MODULE_WAKEUP_ENABLED;
    initParams.autoWkupEnable = MCAN_MODULE_AUTO_WAKEUP_ENABLED;
    initParams.tdcEnable      = MCAN_MODULE_TX_DELAY_COMP_ENABLED;
    initParams.wdcPreload     = MCAN_MODULE_MSG_RAM_WD_DISABLED;
    initParams.tdcConfig.tdcf = MCAN_MODULE_TX_DELAY_WINDOW_LENGTH;
    initParams.tdcConfig.tdco = MCAN_MODULE_TX_DELAY_OFFSET;

    //
    // Configuration Details:
    // - BUs monitoring mode disabled
    // - Normal CAN operation mode
    // - Timeout continuous operation mode
    // - Timeout counter disabled
    // - Reject remote frames and non-matching frames
    //
    configParams.monEnable         = MCAN_CONFIG_BUS_MONITORING_DISABLED;
    configParams.asmEnable         = MCAN_CONFIG_OP_MODE_NORMAL;
    configParams.tsPrescalar       = MCAN_CONFIG_TIMESTAMP_DISABLED;
    configParams.tsSelect          = MCAN_CONFIG_TIMESTAMP_DISABLED;
    configParams.timeoutSelect     = (uint32_t)MCAN_TIMEOUT_SELECT_CONT;
    configParams.timeoutPreload    = MCAN_CONFIG_TIMEOUT_DISABLED;
    configParams.timeoutCntEnable  = MCAN_CONFIG_TIMEOUT_DISABLED;
    configParams.filterConfig.rrfs = MCAN_CONFIG_REJECT_REMOTE_FRAMES;
    configParams.filterConfig.rrfe = MCAN_CONFIG_REJECT_REMOTE_FRAMES;
    configParams.filterConfig.anfe = MCAN_CONFIG_REJECT_NONMATCH_FRAMES;
    configParams.filterConfig.anfs = MCAN_CONFIG_REJECT_NONMATCH_FRAMES;

    //
    // Message RAM Details:
    // - Standard ID filter number set to 1
    // - Extended ID filter number set to 0
    // - Set filter and TX buffer start addresses
    // - Set TX buffer size to 2
    // - TX FIFO/queue disabled
    // - TX watermark interrupt disabled
    // - RX FIFO 0/1 disabled
    // - RX watermark interrupt disabled
    // - Set buffer/FIFO element size to 64 bytes
    //
    msgRAMConfigParams.flssa                = APP_MCAN_STD_ID_FILT_START_ADDR;
    msgRAMConfigParams.lss                  = APP_MCAN_STD_ID_FILTER_NUM;
    msgRAMConfigParams.flesa                = APP_MCAN_EXT_ID_FILT_START_ADDR;
    msgRAMConfigParams.lse                  = APP_MCAN_EXT_ID_FILTER_NUM;
    msgRAMConfigParams.txStartAddr          = APP_MCAN_TX_BUFF_START_ADDR;
    msgRAMConfigParams.txBufNum             = APP_MCAN_TX_BUFF_SIZE;
    msgRAMConfigParams.txFIFOSize           = APP_MCAN_FIFO_DISABLED;
    msgRAMConfigParams.txBufMode            = APP_MCAN_FIFO_DISABLED;
    msgRAMConfigParams.txBufElemSize        = (uint32_t)MCAN_ELEM_SIZE_64BYTES;
    msgRAMConfigParams.txEventFIFOStartAddr = APP_MCAN_TX_EVENT_START_ADDR;
    msgRAMConfigParams.txEventFIFOSize      = APP_MCAN_TX_BUFF_SIZE;
    msgRAMConfigParams.txEventFIFOWaterMark = APP_MCAN_FIFO_WATERMARK_DISABLED;
    msgRAMConfigParams.rxFIFO0startAddr     = APP_MCAN_FIFO_0_START_ADDR;
    msgRAMConfigParams.rxFIFO0size          = APP_MCAN_FIFO_0_NUM;
    msgRAMConfigParams.rxFIFO0waterMark     = APP_MCAN_FIFO_WATERMARK_DISABLED;
    msgRAMConfigParams.rxFIFO0OpMode        = APP_MCAN_FIFO_BLOCKING_MODE;
    msgRAMConfigParams.rxFIFO1startAddr     = APP_MCAN_FIFO_1_START_ADDR;
    msgRAMConfigParams.rxFIFO1size          = APP_MCAN_FIFO_1_NUM;
    msgRAMConfigParams.rxFIFO1waterMark     = APP_MCAN_FIFO_WATERMARK_DISABLED;
    msgRAMConfigParams.rxFIFO1OpMode        = APP_MCAN_FIFO_BLOCKING_MODE;
    msgRAMConfigParams.rxBufStartAddr       = APP_MCAN_RX_BUFF_START_ADDR;
    msgRAMConfigParams.rxBufElemSize        = (uint32_t)MCAN_ELEM_SIZE_64BYTES;
    msgRAMConfigParams.rxFIFO0ElemSize      = (uint32_t)MCAN_ELEM_SIZE_64BYTES;
    msgRAMConfigParams.rxFIFO1ElemSize      = (uint32_t)MCAN_ELEM_SIZE_64BYTES;

    //
    // Standard Message Filter Details
    // - Filter accepts messages of ID 1
    // - Stores in Rx buffer, not FIFO
    // - 2nd ID filter disabled since using Rx buffer mode
    //
    stdMsgFilterParams.sfid2 = MCAN_RX_MSG_FILTER_2ND_ID_DISABLED;
    stdMsgFilterParams.sfid1 = MCAN_RX_MSG_FILTER_ID;
    stdMsgFilterParams.sfec  = MCAN_RX_MSG_FILTER_STORE_IN_BUF;
    stdMsgFilterParams.sft   = MCAN_RX_MSG_FILTER_2ND_ID_DISABLED;

    //
    // Configure bit timing for non-data phase
    //
    bitTimeParams.nomRatePrescalar  = ((nominalBitTiming & MCAN_NBTP_NBRP_M) >> MCAN_NBTP_NBRP_S);
    bitTimeParams.nomTimeSeg1       = ((nominalBitTiming & MCAN_NBTP_NTSEG1_M) >> MCAN_NBTP_NTSEG1_S);
    bitTimeParams.nomTimeSeg2       = ((nominalBitTiming & MCAN_NBTP_NTSEG2_M) >> MCAN_NBTP_NTSEG2_S);
    bitTimeParams.nomSynchJumpWidth = ((nominalBitTiming & MCAN_NBTP_NSJW_M) >> MCAN_NBTP_NSJW_S);

    //
    // Configure bit timing for data phase
    //
    bitTimeParams.dataRatePrescalar   = ((dataBitTiming & MCAN_DBTP_DBRP_M) >> MCAN_DBTP_DBRP_S);
    bitTimeParams.dataTimeSeg1        = ((dataBitTiming & MCAN_DBTP_DTSEG1_M) >> MCAN_DBTP_DTSEG1_S);
    bitTimeParams.dataTimeSeg2        = ((dataBitTiming & MCAN_DBTP_DTSEG2_M) >> MCAN_DBTP_DTSEG2_S);
    bitTimeParams.dataSynchJumpWidth  = ((dataBitTiming & MCAN_DBTP_DSJW_M) >> MCAN_DBTP_DSJW_S);

    //
    // Perform MCAN Initialization
    //

    //
    // Wait for MCAN RAM initialization
    // (MCAN RAM init is started when MCAN clock is enabled)
    //
    timeoutCounter = 0UL;
    while(MCAN_RAM_INIT_NOT_DONE == MCAN_isMemInitDone(MCAN_MSG_RAM_BASE))
    {
        if(timeoutCounter > mcanPollingTimeout)
        {
            //
            // Exit upon error occurring
            //
            return;
        }
        else
        {
            timeoutCounter = timeoutCounter + 1UL;
        }
    }

    //
    // Put MCAN into software init mode
    //
    MCAN_setOpMode(MCAN_MSG_RAM_BASE, (uint32_t)MCAN_OPERATION_MODE_SW_INIT);

    //
    // Wait for MCAN to enter init mode
    //
    timeoutCounter = 0UL;
    while((uint32_t)MCAN_OPERATION_MODE_SW_INIT != MCAN_getOpMode(MCAN_MSG_RAM_BASE))
    {
        if(timeoutCounter > mcanPollingTimeout)
        {
            //
            // Exit upon error occurring
            //
            return;
        }
        else
        {
            timeoutCounter = timeoutCounter + 1UL;
        }
    }

    //
    // Initialization MCAN module
    //
    if(MCAN_init(MCAN_MSG_RAM_BASE, &initParams) != MCAN_API_PASS)
    {
        //
        // Exit upon error occurring
        //
        return;
    }

    //
    // Configure various MCAN settings
    //
    if(MCAN_config(MCAN_MSG_RAM_BASE, &configParams) != MCAN_API_PASS)
    {
        //
        // Exit upon error occurring
        //
        return;
    }

    //
    // Configure MCAN bit timing
    //
    if(MCAN_setBitTime(MCAN_MSG_RAM_BASE, &bitTimeParams) != MCAN_API_PASS)
    {
        //
        // Exit upon error occurring
        //
        return;
    }

    //
    // Configure MCAN message RAM sections
    //
    if(MCAN_msgRAMConfig(MCAN_MSG_RAM_BASE, &msgRAMConfigParams) != MCAN_API_PASS)
    {
        //
        // Exit upon error occurring
        //
        return;
    }

    //
    // Configure standard message ID filter
    //
    MCAN_addStdMsgIDFilter(MCAN_MSG_RAM_BASE, 0UL, &stdMsgFilterParams);

    //
    // Put MCAN into normal operational mode
    //
    MCAN_setOpMode(MCAN_MSG_RAM_BASE, (uint32_t)MCAN_OPERATION_MODE_NORMAL);

    //
    // Wait for MCAN to enter normal operation mode
    //
    timeoutCounter = 0UL;
    while((uint32_t)MCAN_OPERATION_MODE_NORMAL != MCAN_getOpMode(MCAN_MSG_RAM_BASE))
    {
        if(timeoutCounter > mcanPollingTimeout)
        {
            //
            // Exit upon error occurring
            //
            return;
        }
        else
        {
            timeoutCounter = timeoutCounter + 1UL;
        }
    }

    return;
}

//
// MCAN_sendTwoMessages - Transmit two messages when debug MCAN boot
//                        configuration is selected
//
static void MCAN_sendTwoMessages(void)
{
    MCAN_TxBufElement txMsg = { 0U };
    uint32_t timeoutCounter = 0UL;

    //
    // Configure TX Message
    // - Message ID 2
    // - CAN-FD mode
    // - Data frame
    // - Standard ID
    // - Bit rate switching
    // - Event tracking and message marker enabled
    // - 64 bytes DLC
    //
    txMsg.id       = (((uint32_t)(MCAN_TX_MSG_ID)) << MCAN_STD_ID_SHIFT);
    txMsg.dlc      = MCAN_MSG_DLC_64BYTES;
    txMsg.fdf      = MCAN_MSG_FD_FORMAT_ENABLE;
    txMsg.efc      = MCAN_MSG_EVENT_FIFO_ENABLE;
    txMsg.mm       = MCAN_TX_MSG_MARKER;
    txMsg.brs      = MCAN_TX_BITRATESWITCHING_ENABLE;
    txMsg.data[0]  = 0xABU;
    txMsg.data[63] = 0xEFU;

    //
    // Setup and transmit message using buffer 0
    //
    MCAN_writeMsgRam(MCAN_MSG_RAM_BASE, (uint32_t)MCAN_MEM_TYPE_BUF, MCAN_TX_MSG_BUF0, &txMsg);
    (void)MCAN_txBufAddReq(MCAN_MSG_RAM_BASE, MCAN_TX_MSG_BUF0);

    //
    // Wait for message to transmit
    //
    while(TX_MSG_PENDING != HWREG(MCAN_MSG_RAM_BASE + MCAN_TXBRP))
    {
        if(timeoutCounter > mcanPollingTimeout)
        {
            //
            // Exit upon timeout
            //
            break;
        }
        else
        {
            timeoutCounter = timeoutCounter + 1UL;
        }
    }

    //
    // Setup and transmit message using buffer 1
    //
    MCAN_writeMsgRam(MCAN_MSG_RAM_BASE, (uint32_t)MCAN_MEM_TYPE_BUF, MCAN_TX_MSG_BUF1, &txMsg);
    (void)MCAN_txBufAddReq(MCAN_MSG_RAM_BASE, MCAN_TX_MSG_BUF1);

    //
    // Wait for message to transmit
    //
    timeoutCounter = 0UL;
    while(TX_MSG_PENDING != HWREG(MCAN_MSG_RAM_BASE + MCAN_TXBRP))
    {
        if(timeoutCounter > mcanPollingTimeout)
        {
            //
            // Exit upon timeout
            //
            break;
        }
        else
        {
            timeoutCounter = timeoutCounter + 1UL;
        }
    }
}

//
// MCAN_readMessage - Wait for a new message from the host
//                    and then read in the new message
//
void MCAN_readMessage(uint32_t timeoutValue)
{
    uint16_t i;
    volatile uint32_t timeoutCounter = 0UL;
    uint16_t newData;

    //
    // Wait to receive message frame
    //
    newData = HWREGH(MCAN_MSG_RAM_BASE + MCAN_NDAT1);
    while((newData & MCAN_NDAT1_ND0) != MCAN_NDAT1_ND0)
    {

        if(timeoutCounter > timeoutValue)
        {
            //
            // Exit upon error occurring
            //
            return;
            //SysCtl_resetDevice();
        }
        else
        {
            //
            // When debug mode is enabled, waiting for message
            // doesn't timeout
            //
            timeoutCounter = timeoutCounter + 1UL - mcanDebugModeEnabled;
            newData = HWREGH(MCAN_MSG_RAM_BASE + MCAN_NDAT1);
        }

        newData = HWREGH(MCAN_MSG_RAM_BASE + MCAN_NDAT1);
    }

    //
    // Upon successful receiving, read message
    //
    i = 0U;
    while(i < 64U)
    {
     //MCAN_MSG_RAM_BASE + 0x300U, rxMsg.data, 64 bytes
     rxMsg.data[i] = *(uint16_t *)(MCAN_MSG_RAM_BASE + 0x308U + i) & 0xFFU;
     rxMsg.data[i+1U] = ((*(uint16_t *)(MCAN_MSG_RAM_BASE + 0x308U + i)) >> 0x8 ) & 0xFFU;
     i+=2U;
    }

    //
    // Clear message new data
    //
    HWREGH(MCAN_MSG_RAM_BASE + MCAN_NDAT1) |= MCAN_NDAT1_ND0;

}

//
// MCAN_receiveApplication - Decodes the data from the first message to
//                           validate the application key, re-configure
//                           the bit timing if requested, get the app
//                           entry address, and copy the remaining
//                           app data to Flash
//
static uint32_t MCAN_receiveApplication(void)
{
    uint32_t entryAddress;
    uint32_t nomBitTiming = 0UL;
    uint32_t dataBitTiming = 0UL;
    uint16_t deviceSystemFreq = 25;
    
    //
    // Check for valid key (0x08AA)
    //
    if((BUILD_WORD(rxMsg.data[LOWER_KEY_OFFSET],
                   rxMsg.data[UPPER_KEY_OFFSET])) == BROM_EIGHT_BIT_HEADER)
    {
        //
        // Parse reserved words for custom bit timing
        //
        nomBitTiming = BUILD_DWORD(rxMsg.data[LOWER_BYTE1_NOM_TIMING],
                                   rxMsg.data[LOWER_BYTE2_NOM_TIMING],
                                   rxMsg.data[UPPER_BYTE1_NOM_TIMING],
                                   rxMsg.data[UPPER_BYTE2_NOM_TIMING]);
        dataBitTiming = BUILD_DWORD(rxMsg.data[LOWER_BYTE1_DATA_TIMING],
                                    rxMsg.data[LOWER_BYTE2_DATA_TIMING],
                                    rxMsg.data[UPPER_BYTE1_DATA_TIMING],
                                    rxMsg.data[UPPER_BYTE2_DATA_TIMING]);

        //
        // If bit timing set, re-init MCAN with new bit timing
        //
        // Note clock source is set to INTOSC to skip re-setting
        // the XTAL. Clock source will still continue to be XTAL
        // even with this option based in.
        //
        if((nomBitTiming != 0UL) || (dataBitTiming != 0UL))
        {
            //
            // If NOM or DATA bit timing isn't set, use default value
            //
            if(nomBitTiming == 0UL)
            {
                nomBitTiming = mcanDefaultNominalBitTimingConfig;
            }
            if(dataBitTiming == 0UL)
            {
                dataBitTiming = mcanDefaultDataBitTimingConfig;
            }

            MCAN_bootInitialization(nomBitTiming, dataBitTiming,
                                   MCAN_DIVIDER_BY_1, MCAN_CLOCKSRC_SKIP_XTAL_CONFIG,
                                   deviceSystemFreq);
        }
            //
            // Get app entry address
            //
            entryAddress = BUILD_DWORD(rxMsg.data[LOWER_BYTE1_ENTRY_ADDRESS],
                                       rxMsg.data[LOWER_BYTE2_ENTRY_ADDRESS],
                                       rxMsg.data[UPPER_BYTE1_ENTRY_ADDRESS],
                                       rxMsg.data[UPPER_BYTE2_ENTRY_ADDRESS]);

            //
            // Continue receiving messages and copying
            // data to Flash until app transfer is complete
            //
            CopyApplication(rxMsg);
    }

    return(entryAddress);
}



//
// MCAN_getDataFromBuffer - Reads data from the 64 byte RX buffer.
//                          If the buffer doesn't have enough data
//                          left to fulfill the request, the remaining
//                          data will be read out and the next
//                          message will be read to refill the buffer
//
uint32_t MCAN_getDataFromBuffer(MCAN_dataTypeSize dataTypeSize)
{
    uint16_t i;
    uint32_t data = 0;
    uint16_t dataShift = 0;
    uint16_t numberOfBytesRead = 0;
    uint16_t numberOfBytesRemaining;

    //
    // Current message buffer doesn't have enough data
    // to meet current request, must grab remaining from
    // buffer and read in new message data
    //
    if(((uint16_t)dataTypeSize + msgBufferIndex) > MCAN_MSG_BUFFER_MAX_SIZE)
    {
        //
        // Grab remaining data from current message buffer
        //
        numberOfBytesRemaining = (MCAN_MSG_BUFFER_MAX_SIZE - msgBufferIndex);
        for(i = 0U; i < numberOfBytesRemaining; i++)
        {
            //
            // For DWORD reads, data stream provides the bytes
            // in the order BB AA DD CC where data needs to be adjusted to
            // 0xAABBCCDD
            //
            if(i == MCAN_2ND_WORD_INDEX)
            {
                data = (data << MCAN_DWORD_SHIFT);
                dataShift = 0U;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)MCAN_BYTE_MASK) << dataShift);
            msgBufferIndex = msgBufferIndex + 1;
            dataShift += 8;
            numberOfBytesRead = numberOfBytesRead + 1;
        }

        //
        // Read in next message.
        //
        MCAN_readMessage(mcanPollingTimeout);

        //
        // Finish getting data from new message buffer
        //
        msgBufferIndex = 0;
        for(i = 0; i < ((uint16_t)dataTypeSize - numberOfBytesRead); i++)
        {
            //
            // For DWORD reads, data stream provides the bytes
            // in the order BB AA DD CC where data needs to be adjusted to
            // 0xAABBCCDD
            //
            if((i + numberOfBytesRead) == MCAN_2ND_WORD_INDEX)
            {
                data = (data << MCAN_DWORD_SHIFT);
                dataShift = 0;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)MCAN_BYTE_MASK) << dataShift);
            msgBufferIndex = msgBufferIndex + 1U;
            dataShift = dataShift + 8U;
        }
    }
    //
    // Current message buffer still has enough data to
    // return the amount of requested data
    //
    else
    {
        for(i = 0U; i < (uint16_t)dataTypeSize; i++)
        {
            //
            // For DWORD reads, data stream provides the bytes
            // in the order BB AA DD CC where data needs to be adjusted to
            // 0xAABBCCDD
            //
            if(i == MCAN_2ND_WORD_INDEX)
            {
                data = (data << MCAN_DWORD_SHIFT);
                dataShift = 0U;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)MCAN_BYTE_MASK) << dataShift);
            msgBufferIndex = msgBufferIndex + 1U;
            dataShift = dataShift + 8U;
        }
    }

    return(data);
}

void CPU1BROM_triggerSysPLLLock(uint32_t clkSource, uint32_t multiplier, uint32_t divider)
{
    pllMultiplier = multiplier;
    pllDivider = divider;

    EALLOW;

    //
    // Bypass PLL from SYSCLK
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLCLKEN;

    //
    // Delay of at least 120 OSCCLK cycles required post PLL bypass
    //
    NOP_CYCLES(120);

    //
    // Use INTOSC2 (~10 MHz) as the main PLL clock source
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL1) &= ~SYSCTL_CLKSRCCTL1_OSCCLKSRCSEL_M;
    NOP_CYCLES(45);
    HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL1) |= (clkSource & SYSCTL_CLKSRCCTL1_OSCCLKSRCSEL_M);

    //
    // Delay of at least 300 OSCCLK cycles after clock source change
    //
    NOP_CYCLES(150);
    NOP_CYCLES(150);

    //
    // Turn off PLL and delay for power down
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;

    //
    // Delay 60 cycles to power down
    //
    NOP_CYCLES(60);

    //
    // Set PLL Multiplier and Output Clock Divider (ODIV)
    //
    HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) =
                     ((HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) &
                     ~(SYSCTL_SYSPLLMULT_ODIV_M | SYSCTL_SYSPLLMULT_IMULT_M)) |
                              (divider << SYSCTL_SYSPLLMULT_ODIV_S) |
                              (multiplier << SYSCTL_SYSPLLMULT_IMULT_S));

    //
    // Enable the sys PLL
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLEN;

    //
    // 200 Cycles after enabling PLL
    //
    NOP_CYCLES(200);

    EDIS;
}

uint16_t CPU1BROM_switchToPLL(uint32_t pllInputClockMhz)
{
	uint16_t count = 1024; // timeout
	uint16_t dccStatus;
	uint32_t dccCnt0Seed, dccCnt1Seed, dccValid0Seed;


    //
    // Setup DCC Values
    //
    dccCnt0Seed = 104U;
    dccValid0Seed = 32U;

    //
    // + below is to convert bit field values to actual divider values
    //
	// Conterseed1 = window * (Fclk1/Fclk0)
	// window - 120, fclk0 is 10Mhz
	dccCnt1Seed = (120UL * (((pllInputClockMhz * pllMultiplier)/(pllDivider + 1UL)) / 10UL));

	//
	// Wait for the SYSPLL lock counter
	//
	while(((HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLSTS) &
			SYSCTL_SYSPLLSTS_LOCKS) == 0U) && (count != 0U))
	{
		count--;
	}

	//
	// Using DCC to verify the PLL clock
	//
	SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_DCC0);
	dccStatus = BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_INTOSC2,
											  (DCC_Count1ClockSource)DCC_COUNT1SRC_PLL,
											  dccCnt0Seed, dccCnt1Seed, dccValid0Seed);
	SysCtl_disablePeripheral(SYSCTL_PERIPH_CLK_DCC0);

	//
    // If DCC failed to verify PLL clock is running correctly, update status
	// and power down PLL
    //
    if(ERROR == dccStatus)
    {
        //
        // Turn off PLL and delay for power down
        //
        EALLOW;
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;
        EDIS;

        //
        // Delay 120 cycles
        //
        NOP_CYCLES(120);
    }
	else
	{
	    //
	    // Switch sysclk to PLL clock
	    //
		EALLOW;
	    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLCLKEN;
        EDIS;
        
        //
        // ~200 PLLSYSCLK delay to allow voltage regulator to stabilize
        //
        NOP_CYCLES(120);
    }
    return (dccStatus);
}

uint16_t BROMDCC_verifySingleShotClock(DCC_Count0ClockSource clk0src,
                                       DCC_Count1ClockSource clk1src, uint32_t dccCounterSeed0,
                                       uint32_t dccCounterSeed1, uint32_t dccValidSeed0)
{
    uint32_t j = dccCounterSeed1;
    uint16_t status;

    //
    // Clear DONE and ERROR flags
    //
    EALLOW;
    HWREGH(DCC0_BASE + DCC_O_STATUS) = 3U;
    EDIS;

    //
    // Disable DCC
    //
    DCC_disableModule(DCC0_BASE);

    //
    // Disable Error Signal
    //
    DCC_disableErrorSignal(DCC0_BASE);

    //
    // Disable Done Signal
    //
    DCC_disableDoneSignal(DCC0_BASE);

    //
    // Configure Clock Source0 to whatever set as a clock source for PLL
    //
    DCC_setCounter0ClkSource(DCC0_BASE, clk0src);

    //
    // Configure Clock Source1 to PLL
    //
    DCC_setCounter1ClkSource(DCC0_BASE, clk1src);

    //
    // Configure COUNTER-0, COUNTER-1 & Valid Window
    //
    DCC_setCounterSeeds(DCC0_BASE, dccCounterSeed0, dccValidSeed0,
                        dccCounterSeed1);

    //
    // Enable Single Shot mode
    //
    DCC_enableSingleShotMode(DCC0_BASE, DCC_MODE_COUNTER_ZERO);

    //
    // Enable DCC to start counting
    //
    DCC_enableModule(DCC0_BASE);

    //
    // Wait until Error or Done Flag is generated or timeout
    //
    while(((HWREGH(DCC0_BASE + DCC_O_STATUS) &
           (DCC_STATUS_ERR | DCC_STATUS_DONE)) == 0U) && (j != 0U))
    {
		// j is decremented to give enough timeout for HW to complete 
		// the comparison. The result will be determined from the values
		// in status register.
       j--;
    }

    //
    // Returns NO_ERROR if DCC completes without error
    //
    if((HWREGH(DCC0_BASE + DCC_O_STATUS) &
            (DCC_STATUS_ERR | DCC_STATUS_DONE)) == DCC_STATUS_DONE)
	{
		status = NO_ERROR;
	}
	else
	{
		status = ERROR;		
	}

    return status;
}


// Make sure to check for breaks after case statements if editing this function
void setFlashAPIError(Fapi_StatusType status)
{

    switch(status)
    {
        case Fapi_Error_AsyncIncorrectDataBufferLength: {
            statusCode.flashAPIError = INCORRECT_DATA_BUFFER_LENGTH;
            break;
        } 

        case (Fapi_Error_AsyncIncorrectEccBufferLength): {
            statusCode.flashAPIError = INCORRECT_ECC_BUFFER_LENGTH;
            break;
        } 

        case Fapi_Error_AsyncDataEccBufferLengthMismatch: {
            statusCode.flashAPIError = DATA_ECC_BUFFER_LENGTH_MISMATCH;
            break;
        } 

        case Fapi_Error_FlashRegsNotWritable: {
            statusCode.flashAPIError = FLASH_REGS_NOT_WRITABLE;
            break;
        } 
            
        case Fapi_Error_FeatureNotAvailable: {
            statusCode.flashAPIError = FEATURE_NOT_AVAILABLE;
            break;
        } 
        
        case Fapi_Error_InvalidAddress: {
            statusCode.flashAPIError = INVALID_ADDRESS;
            break;
        }
        
        case Fapi_Error_InvalidCPUID: {
            statusCode.flashAPIError = INVALID_CPUID;
            break;
        }

        case Fapi_Error_Fail: {
            statusCode.flashAPIError = FAILURE;
            break;
        }

        case Fapi_Error_OtpChecksumMismatch: {
            statusCode.flashAPIError = OTP_CHECKSUM_MISMATCH;
            break;
        }

        case Fapi_Error_InvalidDelayValue: {
            statusCode.flashAPIError = INVALID_DELAY;
            break;
        }

        case Fapi_Error_InvalidHclkValue: {
            statusCode.flashAPIError = INVALID_HCLK;
            break;
        }
        
        case Fapi_Error_InvalidCpu: {
            statusCode.flashAPIError = INVALID_CPU;
            break;
        }

        case Fapi_Error_InvalidBank: {
            statusCode.flashAPIError = INVALID_BANK;
            break;
        }

        case Fapi_Error_InvalidReadMode: {
            statusCode.flashAPIError = INVALID_READ_MODE;
            break;
        }

        default: {
            statusCode.status = NOT_RECOGNIZED;
            break;
        }
    }
}

void exampleError()
{
    asm(" ESTOP0");
}

//
// End of File
//
