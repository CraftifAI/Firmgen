//###########################################################################
//
// FILE:   cpu1brom_boot_modes.h
//
// TITLE:  Contains boot modes and entry addresses
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

#ifndef BROM_BOOT_MODES_H
#define BROM_BOOT_MODES_H

//
// BootROM System Clock (10MHz)
//
#define BOOTROM_SYSCLK         10000000UL

//
// Boot Mode Values
//
#define PARALLEL_BOOT           0x00U   //0-4,10-12 - 13,14
#define PARALLEL_BOOT_ALT1      0x20U   //89,90,58-62,88 - 91,92

#define SCI_BOOT                0x01U   //GPIO12; GPIO13
#define SCI_BOOT_ALT1           0x21U   //GPIO84; GPIO85
#define SCI_BOOT_ALT2           0x41U   //GPIO36; GPIO35
#define SCI_BOOT_ALT3           0x61U   //GPIO42; GPIO43
#define SCI_BOOT_ALT4           0x81U   //GPIO65; GPIO64
#define SCI_BOOT_ALT5           0xA1U   //GPIO29; GPIO28
#define SCI_BOOT_ALT6           0xC1U   //GPIO8; GPIO9

#define CAN_BOOT                0x02U   //GPIO59; GPIO58
#define CAN_BOOT_ALT1           0x22U   //GPIO04; GPIO05
#define CAN_BOOT_ALT2           0x42U   //GPIO19; GPIO18
#define CAN_BOOT_ALT3           0x62U   //GPIO37, GPIO36
#define CAN_BOOT_ALT4           0x82U   //GPIO63, GPIO62
#define CAN_BOOT_SENDTEST       0xA2U   //GPIO59; GPIO58
#define CAN_BOOT_ALT1_SENDTEST  0xC2U   //GPIO04; GPIO05
#define CAN_BOOT_ALT2_SENDTEST  0xE2U   //GPIO19; GPIO18


#define FLASH_BOOT              0x03U   //BANK0 sector 0 (Default)
#define FLASH_BOOT_ALT1         0x23U   //BANK0 end of sector 127
#define FLASH_BOOT_ALT2         0x43U   //BANK1 sector 0
#define FLASH_BOOT_ALT3         0x63U   //BANK2 sector 0
#define FLASH_BOOT_ALT4         0x83U   //BANK3 sector 0
#define FLASH_BOOT_ALT5         0xA3U   //BANK4 sector 0
#define FLASH_BOOT_ALT6         0xC3U   //BANK4 end of sector 127

#define WAIT_BOOT               0x04U   //with WDOG enabled
#define WAIT_BOOT_ALT1          0x24U   //without WDOG enabled

#define RAM_BOOT                0x05U

#define SPI_CONTROLLER_BOOT         0x06U   //GPIO 58,59,34,35
#define SPI_CONTROLLER_BOOT_ALT1    0x26U   //GPIO 198,203,204,205
#define SPI_CONTROLLER_BOOT_ALT2    0x46U   //GPIO 16,17,18,19
#define SPI_CONTROLLER_BOOT_ALT3    0x66U   //GPIO 54,55,56,57

#define I2C_CONTROLLER_BOOT         0x07U   //GPIO0(SDA), GPIO1(SCL)
#define I2C_CONTROLLER_BOOT_ALT1    0x27U   //GPIO42(SDA), GPIO43(SCL)
#define I2C_CONTROLLER_BOOT_ALT2    0x47U   //GPIO91(SDA), GPIO92(SCL)
#define I2C_CONTROLLER_BOOT_ALT3    0x67U   //GPIO104(SDA), GPIO105(SCL)

#define MCAN_BOOT                0x08U   //GPIO4  (Tx); GPIO10 (Rx)
#define MCAN_BOOT_ALT1           0x18U   //GPIO8  (Tx); GPIO10 (Rx)
#define MCAN_BOOT_ALT2           0x28U   //GPIO19 (Tx); GPIO18 (Rx)
#define MCAN_BOOT_ALT3           0x38U   //GPIO4  (Tx); GPIO5  (Rx)
#define MCAN_BOOT_ALT4           0x48U   //GPIO74 (Tx); GPIO75 (Rx)
#define MCAN_BOOT_SENDTEST       0x58U   //GPIO4  (Tx); GPIO10 (Rx)
#define MCAN_BOOT_ALT1_SENDTEST  0x68U   //GPIO8  (Tx); GPIO10 (Rx)
#define MCAN_BOOT_ALT2_SENDTEST  0x78U   //GPIO19 (Tx); GPIO18 (Rx)
#define MCAN_BOOT_ALT3_SENDTEST  0x88U   //GPIO4  (Tx); GPIO5  (Rx)
#define MCAN_BOOT_ALT4_SENDTEST  0x98U   //GPIO74 (Tx); GPIO75 (Rx)

#define USB_BOOT                 0x09U    //GPIO42(USB0DM) and GPIO43(USB0DP)

#define SECURE_FLASH_BOOT              0x0AU   //BANK0 sector 0 (Default)
#define SECURE_FLASH_BOOT_ALT2         0x4AU   //BANK1 sector 0
#define SECURE_FLASH_BOOT_ALT3         0x6AU   //BANK2 sector 0
#define SECURE_FLASH_BOOT_ALT4         0x8AU   //BANK3 sector 0
#define SECURE_FLASH_BOOT_ALT5         0xAAU   //BANK4 sector 0

//
// FWU Flash Boot Options
//
#define FWU_FLASH_BOOT                  0x0BU
#define FWU_FLASH_BOOT_ALT1             0x2BU
#define FWU_FLASH_BOOT_ALT2             0x4BU
#define FWU_FLASH_BOOT_ALT3             0x6BU

#define BOOTMODE_MASK                  0x0FU   //mask to know which kind of boot mode we are in
//
// Entry Addresses
//
#define FLASH_ENTRY_POINT       0x00080000UL
#define FLASH_ENTRY_POINT_ALT1  0x0009FFF0UL
#define FLASH_ENTRY_POINT_ALT2  0x000A0000UL
#define FLASH_ENTRY_POINT_ALT3  0x000C0000UL
#define FLASH_ENTRY_POINT_ALT4  0x000E0000UL
#define FLASH_ENTRY_POINT_ALT5  0x00100000UL
#define FLASH_ENTRY_POINT_ALT6  0x0011FFF0UL

#define RAM_ENTRY_POINT         0x000000U       //M0 start address

#define FWU_FLASH_ENTRY_POINT_BANK0          0x00080000UL
#define FWU_FLASH_ENTRY_POINT_BANK1          0x000A0000UL
#define FWU_FLASH_ENTRY_POINT_BANK2          0x000C0000UL
#define FWU_FLASH_ENTRY_POINT_BANK3          0x000E0000UL
#define FWU_FLASH_ENTRY_POINT_BANK4          0x00100000UL

#define FWU_FLASH_ENTRY_POINT_ALT1_BANK0     0x0008FFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT1_BANK1     0x000AFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT1_BANK2     0x000CFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT1_BANK3     0x000EFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT1_BANK4     0x0010FFF0UL

#define FWU_FLASH_ENTRY_POINT_ALT2_BANK0     0x00090000UL
#define FWU_FLASH_ENTRY_POINT_ALT2_BANK1     0x000B0000UL
#define FWU_FLASH_ENTRY_POINT_ALT2_BANK2     0x000D0000UL
#define FWU_FLASH_ENTRY_POINT_ALT2_BANK3     0x000F0000UL
#define FWU_FLASH_ENTRY_POINT_ALT2_BANK4     0x00110000UL

#define FWU_FLASH_ENTRY_POINT_ALT3_BANK0     0x0009FFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT3_BANK1     0x000BFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT3_BANK2     0x000DFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT3_BANK3     0x000FFFF0UL
#define FWU_FLASH_ENTRY_POINT_ALT3_BANK4     0x0011FFF0UL

//
// Misc
//
#define BROM_EIGHT_BIT_HEADER   0x08AAU

//
// Bootloader function pointer
//
typedef uint16_t (*uint16fptr)(void);
extern  uint16fptr GetWordData;

//
// Function Prototypes
//
extern uint32_t GetLongData(void);
extern void CopyData(void);
extern void ReadReservedFn(void);

#endif // BROM_BOOT_MODES_H
