//###########################################################################
//
// FILE:    flash_kernel_ex4_boot.c
//
// TITLE:   Boot loader shared functions
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
#include "cpu1bootrom.h"
#include "cpu1brom_utils.h"



#include "FlashTech_F28P65x_C28x.h"
#include "f28p65x_kernel_commands_cpu1.h"
#include "flash_programming_f28p65x.h"

#include "driverlib.h"

#define BUFFER_SIZE               0x200

#define LOWER_FIRST_BLOCK_SIZE      22U
#define UPPER_FIRST_BLOCK_SIZE      23U
//
// GetWordData is a pointer to the function that interfaces to the peripheral.
// Each loader assigns this pointer to it's particular GetWordData function.
//
uint16fptr GetWordData;

//
// Function prototypes
//
uint32_t GetLongData();
void CopyData(void);
void CopyApplication(MCAN_RxBufElement rxMsg);
void ReadReservedFn(void);
extern void sharedErase(uint32_t sectors);
extern uint32_t MCAN_getDataFromBuffer(MCAN_dataTypeSize dataTypeSize);
extern uint16_t msgBufferIndex;
void setFlashAPIError(Fapi_StatusType status);
void exampleError(Fapi_StatusType status);

//
// 128 sectors.
//
unsigned char erasedAlready[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};

//
// CopyApplication - This routine copies multiple blocks of data from the host
//                   to the specified Flash locations. It is assumed that the
//                   application is linked to Flash correctly and that the image is
//                   128 bit aligned. Errors from the Flash API are not currently
//                   being relayed to the host.
//
//                   Multiple blocks of data are copied until a block
//                   size of 00 00 is encountered.
//

void CopyApplication(MCAN_RxBufElement rxMsg)
{
    struct HEADER {
      uint16_t BlockSize;
      uint32_t DestAddr;
    } BlockHeader;

    uint16_t i = 0;
    uint16_t j = 0;
    uint16_t k = 0;
    uint16_t fail = 0;
    uint16_t wordsWritten = 0;

    //Fapi_StatusType oReturnCheck;
    Fapi_FlashStatusWordType oFlashStatusWord;
    Fapi_FlashStatusType oFlashStatus;

    //
    // wordData: Stores a word of data
    //
    uint16_t wordData;

    //
    // miniBuffer: Useful for 4-word access to flash
    //
    uint16_t miniBuffer[8];

    //
    // Buffer: Used to program data to flash
    //
    uint16_t Buffer[BUFFER_SIZE];

    //
    // Error return variable
    //
    Fapi_StatusType oReturnCheck;

    //
    // Get the size in words of the first block
    //
    BlockHeader.BlockSize = BUILD_WORD(rxMsg.data[LOWER_FIRST_BLOCK_SIZE],
                                       rxMsg.data[UPPER_FIRST_BLOCK_SIZE]);

    //
    // Set the message buffer index for reading next stream data
    //
    msgBufferIndex = UPPER_FIRST_BLOCK_SIZE + 1U;

    //
    // While the block size is > 0 flash the data
    // to the DestAddr.  There is no error checking
    // as it is assumed the DestAddr is a valid
    // memory location
    //
    while(BlockHeader.BlockSize != (uint16_t)0x0000)
    {
            //Fapi_StatusType oReturnCheck;
            //Fapi_FlashStatusWordType oFlashStatusWord;
            //Fapi_FlashStatusType oFlashStatus;

            BlockHeader.DestAddr = MCAN_getDataFromBuffer(MCAN_DATA_SIZE_32BITS);

            //
            // Iterate through the block of data in order to program the data
            // in flas
            //
            for (i = 0; i < BlockHeader.BlockSize; i += 0)
            {
                //
                // If the size of the block of data is less than the size of the buffer,
                // then fill the buffer with the block of data and pad the remaining
                // elements
                //
                if (BlockHeader.BlockSize < BUFFER_SIZE)
                {
                    //
                    // Receive the block of data one word at a time and place it in
                    // the buffer
                    //
                    for (j = 0; j < BlockHeader.BlockSize; j++)
                    {
                        //
                        // Receive one word of data
                        //
                        wordData = (uint16_t)(MCAN_getDataFromBuffer(MCAN_DATA_SIZE_16BITS));

                        //
                        // Put the word of data in the buffer
                        //
                        Buffer[j] = wordData;

                        //
                        // Increment i to keep track of how many words have been received
                        //
                        i++;
                    }

                    //
                    // Pad the remaining elements of the buffer
                    //
                    for (j = BlockHeader.BlockSize; j < BUFFER_SIZE; j++)
                    {
                        //
                        // Put 0xFFFF into the current element of the buffer. OxFFFF is equal to erased
                        // data and has no effect
                        //
                        Buffer[j] = 0xFFFF;
                    }
                }

                //
                // Block is to big to fit into our buffer so we must program it in
                // chunks
                //
                else
                {
                    //
                    // Less than one BUFFER_SIZE left
                    //
                    if ((BlockHeader.BlockSize - i) < BUFFER_SIZE)
                    {
                        //
                        // Fill Buffer with rest of data
                        //
                        for (j = 0; j < BlockHeader.BlockSize - i; j++)
                        {
                           //
                           // Receive one word of data
                           //
                           wordData = (uint16_t)(MCAN_getDataFromBuffer(MCAN_DATA_SIZE_16BITS));

                           //
                           // Put the word of data into the current element of Buffer
                           //
                           Buffer[j] = wordData;
                        }

                        //
                        // Increment i outside here so it doesn't affect loop above
                        //
                        i += j;

                        //
                        // Fill the rest with 0xFFFF
                        //
                        for (; j < BUFFER_SIZE; j++)
                        {
                           Buffer[j] = 0xFFFF;
                        }
                    }
                    else
                    {
                        //
                        // Fill up like normal, up to BUFFER_SIZE
                        //
                        for (j = 0; j < BUFFER_SIZE; j++)
                        {
                           wordData = (uint16_t)(MCAN_getDataFromBuffer(MCAN_DATA_SIZE_16BITS));
                           Buffer[j] = wordData;
                           i++;
                        }
                    }
                }

                //
                // Fill miniBuffer with the data in Buffer in order to program the data
                // to flash; miniBuffer takes data from Buffer, 88 words at a time.
                //
                for (k = 0; k < (BUFFER_SIZE / 8); k++)
                {
                    miniBuffer[0] = Buffer[k * 8 + 0];
                    miniBuffer[1] = Buffer[k * 8 + 1];
                    miniBuffer[2] = Buffer[k * 8 + 2];
                    miniBuffer[3] = Buffer[k * 8 + 3];
                    miniBuffer[4] = Buffer[k * 8 + 4];
                    miniBuffer[5] = Buffer[k * 8 + 5];
                    miniBuffer[6] = Buffer[k * 8 + 6];
                    miniBuffer[7] = Buffer[k * 8 + 7];

                    //
                    // check that all the words have not already been written
                    //
                    if (wordsWritten < BlockHeader.BlockSize)
                    {
                        if(fail == 0)
                        {
                            //
                            //program 8 words at once, 128-bits
                            //
                            //

                            //
                            // Disable erase/program protection
                            // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
                            // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
                            // sectors 40-47, etc
                            //
                           Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, 0);
                           Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, 0);

                            oReturnCheck = Fapi_issueProgrammingCommand(
                                    (uint32_t *) BlockHeader.DestAddr, miniBuffer,
                                    sizeof(miniBuffer), 0, 0, Fapi_AutoEccGeneration);

                            while (Fapi_checkFsmForReady() == Fapi_Status_FsmBusy);

                            oFlashStatus = Fapi_getFsmStatus();

                            if (oReturnCheck != Fapi_Status_Success || oFlashStatus != 3)
                            {
                                fail++;
                            }
                        }

                        for (j = 0; j < 8; j += 2)
                        {
                            uint32_t toVerify = miniBuffer[j + 1];
                            toVerify = toVerify << 16;
                            toVerify |= miniBuffer[j];
                            if(fail == 0)
                            {
                                oReturnCheck = Fapi_doVerify(
                                        (uint32_t *) (BlockHeader.DestAddr + j), 1,
                                        (uint32_t *) (&toVerify), &oFlashStatusWord);
                                if (oReturnCheck != Fapi_Status_Success)
                                {
                                    fail++;
                                }
                            }
                        } //for j; for Fapi_doVerify

                    } //check if all the words are not already written
                    BlockHeader.DestAddr += 0x8;
                    wordsWritten += 0x8;
                } //for(int k); loads miniBuffer with Buffer elements
        }
        //
        // Get the size of the next block
        //
        BlockHeader.BlockSize = (uint16_t)(MCAN_getDataFromBuffer(MCAN_DATA_SIZE_16BITS));
        wordsWritten = 0;
    }
    return;
}

//
// getLongData -    Fetches a 32-bit value from the peripheral
//                  input stream.
//

uint32_t GetLongData(void)
{
    uint32_t longData;

    //
    // Fetch the upper 1/2 of the 32-bit value
    //
    longData = ( (uint32_t)(*GetWordData)() << 16);

    //
    // Fetch the lower 1/2 of the 32-bit value
    //
    longData |= (uint32_t)(*GetWordData)();

    return longData;
}

//
// Read_ReservedFn -    Reads 8 reserved words in the header.
//                      None of these reserved words are used by the
//                      this boot loader at this time, they may be used in
//                      future devices for enhancments.  Loaders that use
//                      these words use their own read function.
//

void ReadReservedFn(void)
{
    uint16_t i;

    //
    // Read and discard the 8 reserved words.
    //
    for(i = 1; i <= 8; i++)
    {
       GetWordData();
    }
    return;
}

