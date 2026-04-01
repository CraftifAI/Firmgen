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

#define CAN_BOOT                0x02U   //GPIO59; GPIO58
#define CAN_BOOT_ALT1           0x22U   //GPIO04; GPIO05
#define CAN_BOOT_ALT2           0x42U   //GPIO19; GPIO18
#define CAN_BOOT_ALT3           0x62U   //GPIO37, GPIO36
#define CAN_BOOT_ALT4           0x82U   //GPIO63, GPIO62
#define CAN_BOOT_SENDTEST       0xA2U   //GPIO59; GPIO58
#define CAN_BOOT_ALT1_SENDTEST  0xC2U   //GPIO04; GPIO05
#define CAN_BOOT_ALT2_SENDTEST  0xE2U   //GPIO19; GPIO18

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
