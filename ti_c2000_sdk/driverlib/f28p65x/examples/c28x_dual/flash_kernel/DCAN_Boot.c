//###########################################################################
//
// FILE:    DCAN_Boot.c
//
// TITLE:   DCAN (CAN) Bootloader
//
// Functions involved in running CAN bootloader
//
//
//###########################################################################
// $TI Release: $
// 
// 
// C2000Ware v6.00.00.00
//
// Copyright (C) 2024 Texas Instruments Incorporated - http://www.ti.com
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
#include "can.h"
#include "ex_5_cpu1bootrom.h"
#include "ex_5_cpu1brom_utils.h"
#include "inc/hw_memmap.h"
#include "inc/hw_sysctl.h"
#include "inc/hw_can.h"
#include "inc/hw_gpio.h"
#include "ex_5_bootloader_can_timing.h"
#include "ex_5_bootloader_can.h"
#include "FlashTech_F28P65x_C28x.h"
#include "ex_5_f28p65x_kernel_commands_cpu1.h"
#include "flash_programming_f28p65x.h"

//
// Defines
//
#define SELECT_EXTERNAL_OSC    0x1UL
#define CLOCK_DIVIDER_1        0x0U
#define CAN_DISABLE_PARITY     0x5UL
#define CAN_ENABLE_PARITY      0xAUL
#define CAN_11_BIT_ID_S        18U
#define CAN_RX_MSG_ID          0x1UL
#define CAN_TX_MSG_ID          0x2UL
#define CAN_MSG_OBJ_1          0x1U
#define CAN_MSG_OBJ_2          0x2U
#define CAN_DLC_SIZE           0x8U
#define DCAN_MAX_PAYLOAD_BYTES  8U

//from hw_memmap.h
#define CAN_MSG_RAM_BASE         0x00049000U
#define BLANK_ERROR              0x2000
#define VERIFY_ERROR             0x3000
#define PROGRAM_ERROR            0x4000

// from sysct.h
#define SYSCTL_O_DC1             0x12U

//
// Rx Message Buffer Indexes
//
#define DCAN_MSG_BUFFER_MAX_SIZE    8U

#define LOWER_KEY_OFFSET            0U
#define UPPER_KEY_OFFSET            1U

#define LOWER_BYTE1_NOM_TIMING      4U
#define LOWER_BYTE2_NOM_TIMING      5U
#define UPPER_BYTE1_NOM_TIMING      2U
#define UPPER_BYTE2_NOM_TIMING      3U

#define LOWER_BYTE1_ENTRY_ADDRESS   4U
#define LOWER_BYTE2_ENTRY_ADDRESS   5U
#define UPPER_BYTE1_ENTRY_ADDRESS   2U
#define UPPER_BYTE2_ENTRY_ADDRESS   3U

#define LOWER_FIRST_BLOCK_SIZE      6U
#define UPPER_FIRST_BLOCK_SIZE      7U

//
// Misc
//
#define DCAN_BYTE_MASK                0xFFU
#define DCAN_DWORD_SHIFT              16U
#define DCAN_2ND_WORD_INDEX           2U

//
// The key value for RAM initialization
//
#define CAN_RAM_INIT_KEY           (0xAU)

typedef enum
{
    DCAN_DATA_SIZE_16BITS = 2U,
    DCAN_DATA_SIZE_32BITS = 4U
}DCAN_dataTypeSize;


/**
 * \brief  Structure for DCAN Rx Buffer element.
 */
typedef struct
{
    uint16_t  data[DCAN_MAX_PAYLOAD_BYTES];
    /**< Data bytes.
     *   Only first dlc number of bytes are valid.
     */
}DCAN_RxBufElement;

typedef struct
{
   uint16_t status;
   uint32_t address;
   uint32_t data;
   uint16_t flashAPIError;
   uint32_t flashAPIFsmStatus;
}  StatusCode;
StatusCode statusCode;

//*****************************************************************************
//
//! The following are values that can be passed to SysCtl_isPeripheralPresent()
//! as the \e peripheral parameter.
//
//*****************************************************************************
typedef enum
{
    // DC11
    SYSCTL_PERIPH_PRESENT_CANA   = 0x000AU,
} SysCtl_PeripheralDC;

//
// Function Prototypes
//
extern void CopyApplication(DCAN_RxBufElement rxMsg, uint32_t* WE_Protection_Mask_A,
                            uint32_t* WE_Protection_Mask_B, uint32_t* WE_Protection_OTP_Mask);
static uint32_t DCAN_receiveApplication(uint32_t* WE_Protection_Mask_A, 
                                        uint32_t* WE_Protection_Mask_B, uint32_t* WE_Protection_OTP_Mask);
static void DCAN_SendWordData(uint16_t data);
static void DCAN_readMessage();
uint32_t DCAN_getDataFromBuffer(DCAN_dataTypeSize dataTypeSize);
static void DCAN_Boot_GPIO(uint32_t bootMode);
static void DCAN_Boot_Init(uint32_t btrReg,
                           uint16_t switchToXTAL,
                           uint16_t XTAL_frequency);
static uint16_t DCAN_GetWordData(void);
static void DCAN_ParseReservedWords(void);
void setFlashAPIError(Fapi_StatusType status);
void exampleError();
void fsm_clearStatus();

//
// Globals
//
uint16_t msgBufferIndex;
uint32_t pllMultiplier, pllDivider;
static DCAN_RxBufElement rxMsg;
const uint16_t sectSize = Sector2KB_u32length;

// getWordData is a pointer to the function that interfaces to the peripheral.
// Each loader assigns this pointer to it's particular getWordData function.
//
uint16fptr getWordData;


/**
* DCAN_Boot - Run the CAN bootloader setup with the specified GPIOs for the
*             requested CAN boot mode
*
*
* \brief CAN Boot
*
* Design: did_can_boot_algo did_boot_first_instance_algo)
* Requirement: REQ_TAG(C2000BROM-208), REQ_TAG(C2000BROM-210)
*
* Start CAN Boot
*
*/
uint32_t DCAN_Boot(uint32_t bootMode, uint32_t bitTimingRegValue,
                          uint16_t switchToXTAL, uint16_t XTAL_frequency,
                          uint16_t numBanksToErase, uint32_t* flashBanksToErase, 
                          uint32_t* WE_Protection_Mask_A, uint32_t* WE_Protection_Mask_B,
                          uint32_t* WE_Protection_OTP_Mask)
{
    //
    // Fapi variables
    //
    Fapi_StatusType oReturnCheck;
    Fapi_FlashStatusWordType oFlashStatusWord;

    //
    // Initialize local variables
    //
    uint16_t i = 0;
    uint16_t fail = 0;
    uint32_t bankAddress = 0;
    
    //
    //Temporary variable as MISRA does not let arguments be modified.
    //
    uint32_t timingValue = bitTimingRegValue;

    //
    // Assign the CAN data reader function to the global
    // function pointer for loading data.
    //
    GetWordData = &DCAN_GetWordData;

    //
    // Set up the GPIO mux for the chosen pinout
    //
    DCAN_Boot_GPIO(bootMode);

    //
    // Set up the CAN to receive data. Pass the user-provided bit timing
    // register value if one was provided, otherwise pass the default for
    // 100 kbps and based on 20 MHz CAN clock.
    //


    if(bitTimingRegValue == 0U)
    {
        timingValue = CAN_CALC_BTRREG;
    }

    DCAN_Boot_Init(timingValue, switchToXTAL, XTAL_frequency);

    //
    // FAPI initialization
    //
    Fapi_initializeAPI(FlashTech_CPU0_BASE_ADDRESS, 200);
    oReturnCheck = Fapi_setActiveFlashBank(Fapi_FlashBank0);

    if(oReturnCheck != Fapi_Status_Success)
    {
        exampleError();
    }
    
    for(i = 0; i < numBanksToErase; i++){

        void fsm_clearStatus();

        // Allocate the current flash bank being erased to CPU1
        SysCtl_FlashBank flashBankToAllocate;

        switch (flashBanksToErase[i])
        {
            case 0: flashBankToAllocate = SYSCTL_FLASH_BANK0;
                    break;
            case 1: flashBankToAllocate = SYSCTL_FLASH_BANK1;
                    break;
            case 2: flashBankToAllocate = SYSCTL_FLASH_BANK2;
                    break;
            case 3: flashBankToAllocate = SYSCTL_FLASH_BANK3;
                    break;
            case 4: flashBankToAllocate = SYSCTL_FLASH_BANK4;
                    break;
            default: fail++;
        }

        if (fail != 0)
        {
            exampleError();
        }

        // Allocate the current flash bank being erased to CPU1
        SysCtl_allocateFlashBank(flashBankToAllocate, SYSCTL_CPUSEL_CPU1);


        //
        // Set the bank address
        //
        bankAddress = Bzero_Sector0_start + (flashBanksToErase[i] * 0x20000);

        //
        // Disable erase/program protection
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        //
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_Mask_A[i]);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_Mask_B[i]);

        //
        // Erase bank
        //
        oReturnCheck = Fapi_issueBankEraseCommand((uint32 *)bankAddress);

        // Wait until FSM is done with erase bank operation
        while (Fapi_checkFsmForReady() != Fapi_Status_FsmReady)
        {
        }

        fsm_clearStatus();

        //
        // Perform blank check on CMDWEPROTA Mask sectors
        //
        uint32_t j;
        for (j = 0; j < 32; j++) 
        {
            // If sector has WE protection disabled
            if (!(0x1 & (WE_Protection_Mask_A[i] >> j))) 
            {
                uint32_t sectorAddress = bankAddress + (j * Sector2KB_u16length);
                oReturnCheck = Fapi_doBlankCheck((uint32_t*)sectorAddress,
                                                sectSize, &oFlashStatusWord);

                if (oReturnCheck != Fapi_Status_Success)
                {

                    statusCode.status = BLANK_ERROR;
                    statusCode.address = oFlashStatusWord.au32StatusWord[0];
                    statusCode.data = oFlashStatusWord.au32StatusWord[1];
                    setFlashAPIError(oReturnCheck);
                    statusCode.flashAPIFsmStatus = 0;

                    fail++;
                }
            }  
        }

        // Verify that flash has been erased properly, else stop the program
        if (fail != 0) {
            exampleError();
        }

        //
        // Perform blank check on CMDWEPROTB Mask sectors
        //
        for (j = 0; j < 12; j++)
        {
            // If sector has WE protection disabled
            if (!(0x1 & (WE_Protection_Mask_B[i] >> j))) 
            {
                uint32_t sectorAddress = bankAddress + (((j*8) + 32) * Sector2KB_u16length);
                oReturnCheck = Fapi_doBlankCheck((uint32_t*)sectorAddress,
                                                (sectSize*8), &oFlashStatusWord);

                if (oReturnCheck != Fapi_Status_Success)
                {
                    statusCode.status = BLANK_ERROR;
                    statusCode.address = oFlashStatusWord.au32StatusWord[0];
                    statusCode.data = oFlashStatusWord.au32StatusWord[1];
                    setFlashAPIError(oReturnCheck);
                    statusCode.flashAPIFsmStatus = 0;

                    fail++;
                }
            } 
        }
    }

    // Verify that flash has been erased properly, else stop the program
    if (fail != 0) {
        exampleError();
    }

    //
    // Testing Only: Send two tests frames if the boot mode says so
    //
    if(bootMode >= CAN_BOOT_SENDTEST)
    {
        DCAN_SendWordData(0x0320U);
        DCAN_SendWordData(0xf280U);
    }

    DCAN_readMessage();

    // return appEntryAddress;
    return(DCAN_receiveApplication(WE_Protection_Mask_A, WE_Protection_Mask_B, WE_Protection_OTP_Mask));
}

/**
* DCAN_Boot_GPIO - Configure the peripheral mux to connect CAN-A to the
*                  chosen GPIOs
*
*
* \brief CAN Boot GPIO select
*
* Design: did_can_boot_algo
* Requirement: REQ_TAG(C2000BROM-206)
*
* Start I2C Boot
*
*/
static void DCAN_Boot_GPIO(uint32_t bootMode)
{
    uint32_t gpioTx;
    uint32_t gpioRx;
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

        case CAN_BOOT_ALT1:
        case CAN_BOOT_ALT1_SENDTEST:
            //
            // GPIO04 = CANATX
            // GPIO05 = CANARX
            //
            gpioTx = 4UL;
            gpioRx = 5UL;
            gpioTxPinConfig = GPIO_4_CANA_TX;
            gpioRxPinConfig = GPIO_5_CANA_RX;

            break;

        case CAN_BOOT_ALT2:
        case CAN_BOOT_ALT2_SENDTEST:
            //
            // GPI19 = CANATX
            // GPI18 = CANARX
            //
            gpioTx = 19UL;
            gpioRx = 18UL;
            gpioTxPinConfig = GPIO_19_CANA_TX;
            gpioRxPinConfig = GPIO_18_CANA_RX;

            break;

        case CAN_BOOT_ALT3:
            //
            // GPIO37 = CANARX
            // GPIO36 = CANATX
            //
            gpioTx = 37UL;
            gpioRx = 36UL;
            gpioTxPinConfig = GPIO_37_CANA_TX;
            gpioRxPinConfig = GPIO_36_CANA_RX;

            break;

        case CAN_BOOT_ALT4:
            //
            // GPIO63 = CANARX
            // GPIO62 = CANATX
            //
            gpioTx = 63UL;
            gpioRx = 62UL;
            gpioTxPinConfig = GPIO_63_CANA_TX;
            gpioRxPinConfig = GPIO_62_CANA_RX;


            break;

        case CAN_BOOT:
        case CAN_BOOT_SENDTEST:
        default:
            //
            // GPIO59 = CANATX
            // GPIO58 = CANARX
            //
            gpioTx = 59UL;
            gpioRx = 58UL;
            gpioTxPinConfig = GPIO_59_CANA_TX;
            gpioRxPinConfig = GPIO_58_CANA_RX;

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
// DCAN_Boot_Init - Initialize the CAN-A module and configure its bit clock
//                  and message objects
//
static void DCAN_Boot_Init(uint32_t btrReg,
                           uint16_t switchToXTAL,
                           uint16_t XTAL_frequency)
{
    //
    // Select XTAL for CAN clock
    //
    EALLOW;
    if(switchToXTAL == CAN_BOOT_USE_XTAL)
    {
	    //Turn on XTAL and select crystal mode
	    HWREGH(CLKCFG_BASE + SYSCTL_O_XTALCR) &= (uint16_t)~SYSCTL_XTALCR_OSCOFF;
		NOP_CYCLES(45);
	    HWREGH(CLKCFG_BASE + SYSCTL_O_XTALCR) &= (uint16_t)~SYSCTL_XTALCR_SE;

	    //Wait for the X1 clock to saturate
	    HWREG(CLKCFG_BASE + SYSCTL_O_X1CNT) |= SYSCTL_X1CNT_CLR;
	    while(HWREGH(CLKCFG_BASE + SYSCTL_O_X1CNT) != 0x7FFU) {;}

	    //
	    //Choose PLL multipliers based on XTAL frequency for CAN CLK of 20 MHz
	    //
        if(XTAL_frequency == 20U)
        {
            //
            //Configure MULT and ODIV for SYSPLL of 200 MHz
            //
            CPU1BROM_triggerSysPLLLock(CLK_XTAL, APLL_MULT_20, APLL_DIV_2);
        }

        else if(XTAL_frequency == 25U)
        {
            //
            //Configure MULT and ODIV for SYSPLL of 200 MHz
            //
            CPU1BROM_triggerSysPLLLock(CLK_XTAL, APLL_MULT_16, APLL_DIV_2);
        }

        else
        {
            //
            //Empty for MISRA C
            //
        }

        if(NO_ERROR != CPU1BROM_switchToPLL(XTAL_frequency))
        {
            //
            // Connect CAN to Crystal Clock if PLL Switch fails
            //
            EALLOW;
            HWREG(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= (uint32_t)~SYSCTL_CLKSRCCTL2_CANABCLKSEL_M;
            NOP_CYCLES(45);
            HWREG(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) |= (uint32_t)(1U << SYSCTL_CLKSRCCTL2_CANABCLKSEL_S);

            //
            // Set Sysclkdiv to 1 (CAN CLK running at 20/25 MHz XTAL)
            //
            HWREGH(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) = CAN_CLK_DIVIDE_BY_1;
        }

        else
        {
            //
            // Connect CAN to SYS PLL
            //
            EALLOW;
            HWREG(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= (uint32_t)~SYSCTL_CLKSRCCTL2_CANABCLKSEL_M;
            NOP_CYCLES(45);
            HWREG(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) |= (uint32_t)(0U << SYSCTL_CLKSRCCTL2_CANABCLKSEL_S);

            //
            // Set Sysclkdiv to 10 (CAN CLK running at 20 MHz SYS PLL)
            //
            HWREGH(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) = CAN_CLK_DIVIDE_BY_10;
        }

    }

    //
    // Turn on the clock to the DCAN-A module
    //
    HWREG(CPUSYS_BASE + SYSCTL_O_PCLKCR10) |= SYSCTL_PCLKCR10_CAN_A;
    EDIS;

    //
    // Put the CAN module into initialization mode, then issue a software reset
    // via the self-clearing SWR bit.
    //
    HWREG_BP(CANA_BASE + CAN_O_CTL) = (uint32_t)((CAN_DISABLE_PARITY << CAN_CTL_PMD_S) |
                                      CAN_CTL_INIT);
    NOP_CYCLES(16);
    EALLOW;
    HWREG_BP(CANA_BASE + CAN_O_CTL) |= CAN_CTL_SWR;
    EDIS;
    NOP_CYCLES(16);

    //
    // Initialize the CAN message RAM
    //
    HWREG_BP(CANA_BASE + CAN_O_RAM_INIT) = CAN_RAM_INIT_CAN_RAM_INIT |
                                           CAN_RAM_INIT_KEY;
    while((HWREG_BP(CANA_BASE + CAN_O_RAM_INIT) & CAN_RAM_INIT_RAM_INIT_DONE) !=
          CAN_RAM_INIT_RAM_INIT_DONE)
    {
    }

    //
    // Enable config register access, set up the bit timing, and make sure
    // parity stays enabled.
    //
    HWREG_BP(CANA_BASE + CAN_O_CTL) = (uint32_t)((CAN_ENABLE_PARITY << CAN_CTL_PMD_S) |
                                      CAN_CTL_CCE | CAN_CTL_INIT);
    HWREG_BP(CANA_BASE + CAN_O_BTR) = btrReg;

    //
    // Set up a receive message object via interface 1, then transfer it to the
    // message RAM.
    //
    HWREG_BP(CANA_BASE + CAN_O_IF1ARB) = (uint32_t)(CAN_IF1ARB_MSGVAL | 
                               ((uint32_t)(CAN_RX_MSG_ID << CAN_11_BIT_ID_S)));
    HWREG_BP(CANA_BASE + CAN_O_IF1MCTL) = CAN_IF1MCTL_EOB | CAN_DLC_SIZE;
    HWREG_BP(CANA_BASE + CAN_O_IF1MSK) = (uint32_t)(CAN_IF1MSK_MSK_M |
                                                    CAN_IF1MSK_MDIR |
                                                    CAN_IF1MSK_MXTD);
    HWREG_BP(CANA_BASE + CAN_O_IF1CMD) = (uint32_t)(CAN_IF1CMD_DIR |
                                                 CAN_IF1CMD_MASK |
                                                 CAN_IF1CMD_ARB |
                                                 CAN_IF1CMD_CONTROL |
                                                 CAN_IF1CMD_CLRINTPND |
                                                 CAN_MSG_OBJ_1);

    while((HWREGH(CANA_BASE + CAN_O_IF1CMD) & CAN_IF1CMD_BUSY) == CAN_IF1CMD_BUSY)
    {
    }

    //
    // Set up a transmit object via interface 2 for debug, then transfer it to
    // the message RAM.
    //
    HWREG_BP(CANA_BASE + CAN_O_IF2ARB) = (uint32_t)(CAN_IF2ARB_MSGVAL | CAN_IF2ARB_DIR |
                                  ((uint32_t)(CAN_TX_MSG_ID << CAN_11_BIT_ID_S)));
    HWREG_BP(CANA_BASE + CAN_O_IF2MCTL) = CAN_IF2MCTL_EOB | CAN_DLC_SIZE;
    HWREG_BP(CANA_BASE + CAN_O_IF2MSK) = (uint32_t)(CAN_IF2MSK_MSK_M |
                                                    CAN_IF2MSK_MDIR |
                                                    CAN_IF2MSK_MXTD);
    HWREG_BP(CANA_BASE + CAN_O_IF2CMD) = (uint32_t)(CAN_IF2CMD_DIR |
                                                    CAN_IF2CMD_MASK |
                                                    CAN_IF2CMD_ARB |
                                                    CAN_IF2CMD_CONTROL |
                                                    CAN_IF2CMD_CLRINTPND |
                                                    CAN_MSG_OBJ_2);

    while((HWREGH(CANA_BASE + CAN_O_IF2CMD) & CAN_IF2CMD_BUSY) == CAN_IF2CMD_BUSY)
    {
    }

    //
    // Leave initialization mode and disable timing register access and
    // automatic retransmission.
    //
    HWREGH(CANA_BASE + CAN_O_CTL) &= (uint16_t)(~(CAN_CTL_CCE | CAN_CTL_INIT));
}

//
// DCAN_ParseReservedWords - Parse the eight reserved words and check whether
//                           there's a new bit timing register value in the
//                           first pair.
//
static void DCAN_ParseReservedWords(void)
{
    uint32_t newBtrReg;

    //
    // Read the new bit timing value
    //
    newBtrReg = BUILD_DWORD(rxMsg.data[LOWER_BYTE1_NOM_TIMING],
                            rxMsg.data[LOWER_BYTE2_NOM_TIMING],
                            rxMsg.data[UPPER_BYTE1_NOM_TIMING],
                            rxMsg.data[UPPER_BYTE2_NOM_TIMING]);

    //
    // Skip the rest of the reserved words
    //
    DCAN_readMessage();

    //
    // If a new bit timing value was received, switch to the new settings
    //CAN Divider: Set to div by 1 but does not get set as switchToXTAL = 0
    //
    if(newBtrReg != 0x00000000UL)
    {
        DCAN_Boot_Init(newBtrReg, 0U, 0U);
    }
}

//
// DCAN_receiveApplication - Decodes the data from the first message to
//                           validate the application key, re-configure
//                           the bit timing if requested. Second message
//                           to ignore reserved words. Third message will
//                           get the app entry address, and copy the 
//                           remaining app data to Flash.
//
static uint32_t DCAN_receiveApplication(uint32_t* WE_Protection_Mask_A, uint32_t* WE_Protection_Mask_B, uint32_t* WE_Protection_OTP_Mask)
{
    uint32_t entryAddress;

    //
    // Check for valid key (0x08AA). If not, bypass the bootloader.
    //
    if((BUILD_WORD(rxMsg.data[LOWER_KEY_OFFSET],
                   rxMsg.data[UPPER_KEY_OFFSET])) == BROM_EIGHT_BIT_HEADER)
    {
        //
        // Parse reserved words for custom bit timing
        // If bit timing set, re-init DCAN with new bit timing
        DCAN_ParseReservedWords();

        //
        // Get app entry address
        //
        DCAN_readMessage();
        entryAddress = BUILD_DWORD(rxMsg.data[LOWER_BYTE1_ENTRY_ADDRESS],
                                   rxMsg.data[LOWER_BYTE2_ENTRY_ADDRESS],
                                   rxMsg.data[UPPER_BYTE1_ENTRY_ADDRESS],
                                   rxMsg.data[UPPER_BYTE2_ENTRY_ADDRESS]);

        //
        // Continue receiving messages and copying
        // data to Flash until app transfer is complete
        //
        CopyApplication(rxMsg, WE_Protection_Mask_A, WE_Protection_Mask_B, WE_Protection_OTP_Mask);
    }

    return(entryAddress);
}

//
// DCAN_readMessage - Wait for a new message from the host
//                    and then read in the new message
//
void DCAN_readMessage()
{
    volatile uint32_t timeoutCounter = 0UL;

    //
    // Wait for a new CAN message to be received in mailbox 1
    //
    while((HWREG_BP(CANA_BASE + CAN_O_NDAT_21) & CAN_MSG_OBJ_1) != CAN_MSG_OBJ_1)
        {
        }

    //
    // Read the message object via IF1 and return the data
    //

    HWREG_BP(CANA_BASE + CAN_O_IF1CMD) = (uint32_t)(CAN_IF1CMD_TXRQST |
                                                        CAN_IF1CMD_DATA_A |
                                                        CAN_IF1CMD_DATA_B |
                                                        CAN_MSG_OBJ_1);


    NOP_CYCLES(2);

    while((HWREGH(CANA_BASE + CAN_O_IF1CMD) & CAN_IF1CMD_BUSY) == CAN_IF1CMD_BUSY)
        {
        }

    //
    // Copy the data to the receive buffer for message
    //
    CAN_readDataReg(rxMsg.data, CANA_BASE + CAN_O_IF1DATA, 8);

    return;
}

//
// DCAN_getDataFromBuffer - Reads data from the 8 byte RX buffer.
//                          If the buffer doesn't have enough data
//                          left to fulfill the request, the remaining
//                          data will be read out and the next
//                          message will be read to refill the buffer
//
uint32_t DCAN_getDataFromBuffer(DCAN_dataTypeSize dataTypeSize)
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
    if(((uint16_t)dataTypeSize + msgBufferIndex) > DCAN_MSG_BUFFER_MAX_SIZE)
    {
        //
        // Grab remaining data from current message buffer
        //
        numberOfBytesRemaining = (DCAN_MSG_BUFFER_MAX_SIZE - msgBufferIndex);
        for(i = 0U; i < numberOfBytesRemaining; i++)
        {
            //
            // For DWORD reads, data stream provides the bytes
            // in the order BB AA DD CC where data needs to be adjusted to
            // 0xAABBCCDD
            //
            if(i == DCAN_2ND_WORD_INDEX)
            {
                data = (data << DCAN_DWORD_SHIFT);
                dataShift = 0U;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)DCAN_BYTE_MASK) << dataShift);
            msgBufferIndex = msgBufferIndex + 1;
            dataShift += 8;
            numberOfBytesRead = numberOfBytesRead + 1;
        }

        //
        // Read in next message.
        //
        DCAN_readMessage();

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
            if((i + numberOfBytesRead) == DCAN_2ND_WORD_INDEX)
            {
                data = (data << DCAN_DWORD_SHIFT);
                dataShift = 0;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)DCAN_BYTE_MASK) << dataShift);
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
            if(i == DCAN_2ND_WORD_INDEX)
            {
                data = (data << DCAN_DWORD_SHIFT);
                dataShift = 0U;
            }

            data |= (((uint32_t)rxMsg.data[msgBufferIndex] & (uint32_t)DCAN_BYTE_MASK) << dataShift);
            msgBufferIndex = msgBufferIndex + 1U;
            dataShift = dataShift + 8U;
        }
    }

    return(data);
}

//
// DCAN_GetWordData - Read 16 bits from an incoming DCAN message sent to ID #1.
//                    If no message has been received, wait for one to arrive.
//
static uint16_t DCAN_GetWordData(void)
{
    //
    // Wait for a new CAN message to be received in mailbox 1
    //
    while((HWREG_BP(CANA_BASE + CAN_O_NDAT_21) & CAN_MSG_OBJ_1) != CAN_MSG_OBJ_1)
    {
    }

    //
    // Read the message object via IF1 and return the data
    //
    HWREG_BP(CANA_BASE + CAN_O_IF1CMD) = (uint32_t)(CAN_IF1CMD_TXRQST |
                                                    CAN_IF1CMD_DATA_A |
                                                    CAN_IF1CMD_DATA_B |
                                                    CAN_MSG_OBJ_1);

    NOP_CYCLES(2);

    while((HWREGH(CANA_BASE + CAN_O_IF1CMD) & CAN_IF1CMD_BUSY) == CAN_IF1CMD_BUSY)
    {
    }

    return(HWREGH(CANA_BASE + CAN_O_IF1DATA));
}

//
// DCAN_SendWordData - Send a CAN message to ID #2 for external testing and
//                     data rate measurement. Wait for the transmission to
//                     complete.
//
static void DCAN_SendWordData(uint16_t data)
{
    HWREG_BP(CANA_BASE + CAN_O_IF2DATA) = data;
    HWREG_BP(CANA_BASE + CAN_O_IF2CMD) = (uint32_t)(CAN_IF2CMD_DIR |
                                                    CAN_IF2CMD_TXRQST |
                                                    CAN_IF2CMD_DATA_A |
                                                    CAN_IF2CMD_DATA_B |
                                                    CAN_MSG_OBJ_2);

    NOP_CYCLES(255);

    while((HWREGH(CANA_BASE + CAN_O_IF2CMD) & CAN_IF2CMD_BUSY) == CAN_IF2CMD_BUSY)
    {
    }

    while((HWREGH(CANA_BASE + CAN_O_TXRQ_21) & CAN_MSG_OBJ_2) == CAN_MSG_OBJ_2)
    {
    }
}

//
// CopyData - This routine copies multiple blocks of data from the host
//            to the specified RAM locations.  There is no error
//            checking on any of the destination addresses.
//            That is because it is assumed all addresses and block size
//            values are correct.
//
//            Multiple blocks of data are copied until a block
//            size of 00 00 is encountered.
//
void CopyData(void)
{
    struct HEADER {
        uint32_t DestAddr;        
        uint16_t BlockSize;
    } BlockHeader;

    uint16_t wordData;
    uint16_t i;

    //
    // Get the size in words of the first block
    //
    BlockHeader.BlockSize = (*GetWordData)();

    //
    // While the block size is > 0 copy the data
    // to the DestAddr.  There is no error checking
    // as it is assumed the DestAddr is a valid
    // memory location
    //
    while(BlockHeader.BlockSize != (uint16_t)0x0000U)
    {
        BlockHeader.DestAddr = GetLongData();

        for(i = 1; i <= BlockHeader.BlockSize; i++)
        {
            wordData = (*GetWordData)();
            *(uint16_t *)BlockHeader.DestAddr = wordData;
            BlockHeader.DestAddr+=1U;
        }

        //
        // Get the size of the next block
        //
        BlockHeader.BlockSize = (*GetWordData)();
    }

    return;
}

/**
* CPU1BROM_triggerSysPLLLock - Power up and lock the SYS PLL.
* The "divider" configured in this routine is PLL Output Divider (ODIV)
* and not "SYSCLKDIVSEL".
*
*
* \brief PLL Lock function
*
* Design: \ref did_trigger_apll_lock_usecase did_enable_pll_lock_algo
*              did_pll_lock_fail_status_algo
* Requirement: REQ_TAG(C2000BROM-214), REQ_TAG(C2000BROM-164)
*
* PLL Lock function
*
*/
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

/**
* \brief Switch to PLL output
*
* Design: \ref did_safety_switch_to_pll_clock_usecase
* Requirement: REQ_TAG(C2000BROM-215), REQ_TAG(C2000BROM-164)
*
* PLL Lock function
*
*/
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

void fsm_clearStatus() 
{
    //
    // Fapi variables
    //
    Fapi_StatusType oReturnCheck;
    Fapi_FlashStatusType oFlashStatus;

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
            exampleError();
        }
    }
}


//
// End of File
//
