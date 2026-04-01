//###########################################################################
//
// FILE:   bootloader_mcan.h
//
// TITLE:  MCAN Bootloader Header
//
//###########################################################################
// $TI Release:  $
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

#ifndef BOOTLOADER_MCAN_H
#define BOOTLOADER_MCAN_H

//
// Includes
//

#include <stdint.h>
#include "inc/hw_types.h"
#include "inc/hw_memmap.h"
#include "inc/hw_mcan.h"
#include "mcan.h"
#include "gpio.h"
#include "pin_map.h"
#include "sysctl.h"
#include "cpu1brom_boot_modes.h"
#include "cpu1bootrom.h"


//
// Defines
//
#define MCAN_BOOT_XTAL_FREQ                  20U   // 20MHz
#define MCAN_BOOT_MCAN_DIVIDE_BY_1           0U    // divide by 1
#define MCAN_BOOT_MCAN_DIVIDE_BY_10          9U    // divide by 6

#define MCAN_BOOT_DEFAULT_NOM_BIT_TIMING     0x0U
#define MCAN_BOOT_DEFAULT_DATA_BIT_TIMING    0x0U

//
// Polling Timeout Define
// (Values in uS)
//
#define MCAN_POLLING_TIMEOUT              11000UL     // ~11ms (covering a max of 220Mhz range)
#define MCAN_POLLING_FIRST_MSG_TIMEOUT    10000000UL  // ~10sec
#define MCAN_LOOP_CYCLE_SCALER            20UL        // Value indicating "average" CPU cycles
                                                      // per while loop which includes a 32b
                                                      // comparison and counter increment
#define MCAN_FIRST_MSG_LOOP_CYCLE_SCALER  25UL        // Value indicating "average" CPU cycles
                                                      // per while loop which includes a 32b
                                                      // comparison,counter increment, and
                                                      // register read

//
// MCAN Clocking and Bit Rate Defines for MCAN Clock = 20 MHz
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

// Nominal phase bit timing of 1Mbps at 20 MHz MCAN clock
#define MCAN_NOM_BIT_TIMING_VALUE   (((uint32_t)MCAN_NOM_SJW << MCAN_NBTP_NSJW_S) | \
                                       ((uint32_t)MCAN_NOM_BRP << MCAN_NBTP_NBRP_S) | \
                                       ((uint32_t)MCAN_NOM_TSEG1 << MCAN_NBTP_NTSEG1_S) | \
                                       ((uint32_t)MCAN_NOM_TSEG2 << MCAN_NBTP_NTSEG2_S))

// Data phase bit timing of 2Mbps at 20 MHz MCAN clock
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
#define MCAN_DIVIDER_BY_6              5U
#define DEVICE_X1_COUNT                (SYSCTL_X1CNT_X1CNT_M << SYSCTL_X1CNT_X1CNT_S)
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

//
// Enum
//
typedef enum
{
    MCAN_DATA_SIZE_16BITS = 2U,
    MCAN_DATA_SIZE_32BITS = 4U
}MCAN_dataTypeSize;

typedef enum
{
    MCAN_BOOT_STATUS_FAIL = 0xA5A5U,
    MCAN_BOOT_STATUS_PASS = 0x5A5AU
}MCAN_bootloaderStatus;

//
//Function Prototype
//
extern uint32_t MCAN_Boot(uint32_t bootMode, uint32_t nominalBitTiming,
                          uint32_t dataBitTiming, uint16_t deviceSystemFreq,
                          uint16_t mcanDivider, uint16_t clockSelection);

#endif // BOOTLOADER_CAN_H
