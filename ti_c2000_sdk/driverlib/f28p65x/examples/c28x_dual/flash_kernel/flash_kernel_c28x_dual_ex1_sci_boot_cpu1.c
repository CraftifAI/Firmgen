//###########################################################################
//
// FILE:    flash_kernel_c28x_dual_ex1_sci_boot_cpu1.c
//
// TITLE:   SCI Boot mode functions
//! <h1> SCI Boot mode functions </h1>
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
#include "Types.h"
#include "driverlib.h"
#include "cpu1bootrom.h"
#include "f28p65x_kernel_commands_cpu1.h"


//
// Globals
//
typedef struct
{
   uint16_t status;
   uint32_t address;
   uint16_t flashAPIError;
   uint32_t flashAPIFsmStatus;
}  StatusCode;
extern StatusCode statusCode;

//
// getWordData is a pointer to the function that interfaces to the peripheral.
// Each loader assigns this pointer to it's particular getWordData function.
//
uint16fptr getWordData;

//
// Function Prototypes
//
extern uint16_t sciaGetWordData(void);
extern uint16_t sciaGetWordDataCPU2(void);
extern void copyData(void);
uint32_t getLongData(void);
extern void readReservedFn(void);
extern void exampleError(Fapi_StatusType status);
uint32_t sciLoadApplication(uint32_t BootMode);
void sciLoadCPU2Kernel(void);
void copyDataRAM(void);

//
// sciLoadApplication  - This module is the main SCI boot routine.
//            It will load code via the SCI-A port.
//
//            It will return a entry point address back
//            to the InitBoot routine which in turn calls
//            the ExitBoot routine.
//
uint32_t sciLoadApplication(uint32_t BootMode)
{
    statusCode.status = NO_ERROR;
    statusCode.address = 0x12346578;
    statusCode.flashAPIError = NO_ERROR;
    statusCode.flashAPIFsmStatus = 0;

    uint32_t EntryAddr;

    //
    // Assign GetWordData to the SCI-A version of the
    // function. GetWordData is a pointer to a function.
    //
    getWordData = sciaGetWordData;

    //
    // If the KeyValue was invalid, abort the load
    // and return the flash entry point.
    //
    if (sciaGetWordData() != 0x08AA)
    {
        statusCode.status = VERIFY_ERROR;
        statusCode.address = FLASH_ENTRY_POINT;
        statusCode.flashAPIError = NO_ERROR;
        statusCode.flashAPIFsmStatus = 0;
    }

    readReservedFn(); //reads and discards 8 reserved words

    EntryAddr = getLongData();

    copyData();

    uint16_t x = 0;
    for(x = 0; x < 32676; x++){}
    for(x = 0; x < 32676; x++){}

    return EntryAddr;
}

void sciLoadCPU2Kernel(void)
{
    statusCode.status = NO_ERROR;
    statusCode.address = 0x12346578;

    //uint32_t EntryAddr;

    //
    // Assign GetWordData to the SCI-A version of the
    // function. GetWordData is a pointer to a function.
    //
    getWordData = sciaGetWordDataCPU2;

    //
    // If the KeyValue was invalid, abort the load
    // and return the flash entry point.
    //
    if (sciaGetWordDataCPU2() != 0x08AA)
    {
        statusCode.status = VERIFY_ERROR;
        statusCode.address = FLASH_ENTRY_POINT;
        statusCode.flashAPIError = NO_ERROR;
        statusCode.flashAPIFsmStatus = 0;
    }

    readReservedFn(); //reads and discards 8 reserved words

    getLongData();

    copyDataRAM();

    uint16_t x = 0;
    for(x = 0; x < 32676; x++){}
    for(x = 0; x < 32676; x++){}

}

void copyDataRAM(void)
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
    BlockHeader.BlockSize = (*getWordData)();

    //
    // While the block size is > 0 copy the data
    // to the DestAddr.  There is no error checking
    // as it is assumed the DestAddr is a valid
    // memory location
    //
    while(BlockHeader.BlockSize != (uint16_t)0x0000U)
    {
        BlockHeader.DestAddr = getLongData();

        for(i = 1; i <= BlockHeader.BlockSize; i++)
        {
            wordData = (*getWordData)();
            *(uint16_t *)BlockHeader.DestAddr = wordData;
            BlockHeader.DestAddr+=1U;
        }

        //
        // Get the size of the next block
        //
        BlockHeader.BlockSize = (*getWordData)();
    }

    return;
}

//
// End of file
//
