//###########################################################################
//
// FILE:    flash_kernel_ex5_can_flash_kernel.c
//
// TITLE:   MCAN Flash Kernel Example for F28003x
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
#include "ex_5_cpu1bootrom.h"
#include "device.h"
#include "gpio.h"
#include "driverlib.h"

//
// Globals
//
extern uint32_t DCAN_Boot(uint32_t bootMode, uint32_t bitTimingRegValue,
                          uint16_t switchToXTAL, uint16_t XTAL_frequency,
                          uint16_t numBanksToErase, uint32_t* flashBanksToErase, 
                          uint32_t* WE_Protection_Mask_A,uint32_t* WE_Protection_Mask_B,
                          uint32_t* WE_Protection_OTP_Mask);

//
// main - This is an example code demonstrating F021 Flash API usage.
//        This code is in RAM
//
uint32_t main(void)
{

    //
    // Initialize device clock and peripherals
    Device_init();

    //
    // Call Flash Initialization to setup flash waitstates. This function must
    // reside in RAM.
    //
    Flash_initModule(FLASH0CTRL_BASE, FLASH0ECC_BASE, DEVICE_FLASH_WAITSTATES);

    //
    // Initialize GPIO
    //
    Device_initGPIO();

    //
    // Initialize DCAN and wait to receive messages
    //
    uint32_t Application_Flash_Banks[5] = {0,1,2,3,4};
    uint32_t WE_Protection_A_Masks[5] = {0,0,0,0,0};
    uint32_t WE_Protection_B_Masks[5] = {0,0,0,0,0};
    uint32_t WE_Protection_OTP_Masks[5] = {0,0,0,0,0};

    return DCAN_Boot(CAN_BOOT_ALT1_SENDTEST, 0, 1, 25, 5, Application_Flash_Banks, 
                     WE_Protection_A_Masks, WE_Protection_B_Masks, WE_Protection_OTP_Masks);
}

//
// End of File
//

