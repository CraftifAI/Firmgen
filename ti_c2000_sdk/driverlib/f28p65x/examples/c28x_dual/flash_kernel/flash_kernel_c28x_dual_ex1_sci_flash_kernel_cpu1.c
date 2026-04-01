//###########################################################################
//
// FILE:    flash_kernel_c28x_dual_ex1_sci_flash_kernel_cpu1.c
//
// TITLE:  Flash Programming Solution using SCI.
//
//! \addtogroup driver_example_list
//! <h1> Flash Programming Solution using SCI </h1>
//!
//! In this example, we set up a UART connection with a host using SCI, receive
//! commands for CPU1 to perform which then sends ACK, NAK, and status packets
//! back to the host after receiving and completing the tasks.  This kernel has
//! the ability to program, verify, unlock, reset, and run an application.
//! Each command either expects no data from the command packet
//! or specific data relative to the command.
//!
//! In this example, we set up a UART connection with a host using SCI, receive
//! an application for CPU01 in -sci8 ascii format to run on the device and
//! program it into Flash.
//!
//
//###########################################################################
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

#include <flash_kernel_c28x_dual_ex1_erase_cpu1.h>
#include <string.h>
#include "flash_programming_f28p65x.h" // Flash API example header file
#include "FlashTech_F28P65x_C28x.h"
#include "cpu1bootrom.h"
#include "device.h"

//
// Function Prototypes
//
void exampleError(Fapi_StatusType status);
void initFlashSectors(void);
void SeizeFlashPump_kernels(void);
extern uint32_t sciGetFunction(uint32_t  BootMode);
extern void sciaFlush(void);

//
// Main
//
uint32_t main(void)
{
    //
    // flush SCIA TX port by waiting while it is busy, driverlib.
    //
    sciaFlush();

    //
    // initialize device and GPIO, driverlib.
    //
    Device_init();
    Device_initGPIO();

    //
    // init interrupt and vectorTable, drivelib.
    //
    Interrupt_initModule();
    Interrupt_initVectorTable();

    //
    // Enable Global Interrupt (INTM) and realtime interrupt (DBGM)
    //
    EINT;
    ERTM;

    //
    // At 200MHz, execution wait-states for external oscillator is 4. Modify the
    // wait-states when the system clock frequency is changed.
    //
    Flash_initModule(FLASH0CTRL_BASE, FLASH0ECC_BASE, 4);

    //
    // Pump access must be gained by the core using pump semaphore
    //
    EALLOW;
#ifdef CPU1
    IPC_claimFlashSemaphore(IPC_FLASHSEM_OWNER_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK0, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK1, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK2, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK3, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK4, SYSCTL_CPUSEL_CPU1);

    //
    // Pump access must be gained by the core using pump semaphore
    //
#elif  defined(CPU2)
    IPC_claimFlashSemaphore(IPC_FLASHSEM_OWNER_CPU2);
#endif

    //
    // initialize flash_sectors, fapi + driverlib
    //
    initFlashSectors();

    uint32_t EntryAddr;

    //
    // parameter SCI_BOOT_ALT5 for GPIO28 (RX),29 (TX)
    //
    EntryAddr = sciGetFunction(SCI_BOOT_ALT5);
    return(EntryAddr);
}

//
// Init_Flash_Sectors - Initialize flash API and active flash bank sectors
//
void initFlashSectors(void)
{
    EALLOW;
    Fapi_StatusType oReturnCheck;

    oReturnCheck = Fapi_initializeAPI(FlashTech_CPU0_BASE_ADDRESS, 200);
    if(oReturnCheck != Fapi_Status_Success)
    {
        exampleError(oReturnCheck);
    }

    oReturnCheck = Fapi_setActiveFlashBank(Fapi_FlashBank0);
    if(oReturnCheck != Fapi_Status_Success)
    {
        exampleError(oReturnCheck);
    }
    EDIS;
}



//
// exampleError - For this example, if an error is found just stop here
//
#ifdef __TI_COMPILER_VERSION__
    #if __TI_COMPILER_VERSION__ >= 15009000
        #pragma CODE_SECTION(exampleError,".TI.ramfunc");
    #else
        #pragma CODE_SECTION(exampleError,"ramfuncs");
    #endif
#endif

void exampleError(Fapi_StatusType status)
{
    //
    // Error code will be in the status parameter
    //
    __asm("    ESTOP0");
}

//
// End of file
//

