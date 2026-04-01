//###########################################################################
//
// FILE:    flash_kernel_ex5_boot.c
//
// TITLE:   Boot loader shared functions
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
#include "ex_5_cpu1brom_utils.h"
#include "FlashTech_F28P65x_C28x.h"
#include "ex_5_f28p65x_kernel_commands_cpu1.h"
#include "flash_programming_f28p65x.h"
#include "driverlib.h"

#define BUFFER_SIZE               0x8
#define DCAN_MAX_BUFFER_SIZE        8U
#define LOWER_FIRST_BLOCK_SIZE      6U
#define UPPER_FIRST_BLOCK_SIZE      7U
#define DCAN_MAX_PAYLOAD_BYTES      8U

#define DCSM_OTP_Z1_LNKPTR_START 0x78000
#define DCSM_OTP_Z1_LNKPTR_END   0x78020

#define DCSM_OTP_Z2_LNKPTR_START 0x78200
#define DCSM_OTP_Z2_LNKPTR_END   0x78220

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

//
// GetWordData is a pointer to the function that interfaces to the peripheral.
// Each loader assigns this pointer to it's particular GetWordData function.
//
uint16fptr GetWordData;

//
// miniBuffer: Useful for 4-word access to flash
//
uint16_t miniBuffer[4];

//
// Function prototypes
//
uint32_t GetLongData();
void CopyData(void);
void CopyApplication(DCAN_RxBufElement rxMsg, uint32_t* WE_Protection_Mask_A, 
                        uint32_t* WE_Protection_Mask_B, uint32_t* WE_Protection_OTP_Mask);
void ReadReservedFn(void);
void DCSM_OTP_Write(uint32_t destAddr, uint16_t* miniBuffer, uint32_t WE_Protection_Mask_UO);
extern void sharedErase(uint32_t sectors);
uint16_t DCAN_GetWordData(void);
extern uint32_t DCAN_getDataFromBuffer(DCAN_dataTypeSize dataTypeSize);
extern uint16_t msgBufferIndex;
void Example_Error();

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

void CopyApplication(DCAN_RxBufElement rxMsg, uint32_t* WE_Protection_Mask_A, 
                        uint32_t* WE_Protection_Mask_B, uint32_t* WE_Protection_OTP_Mask)
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

            BlockHeader.DestAddr = DCAN_getDataFromBuffer(DCAN_DATA_SIZE_32BITS);

            // Find current bank to use appropriate protection masks
            uint16_t currentBank = 0;
            uint16_t otpFlag = 0;

            if (BlockHeader.DestAddr >= FlashBank0StartAddress && BlockHeader.DestAddr < FlashBank1StartAddress) 
            {
                currentBank = 0;
            } else if (BlockHeader.DestAddr >= FlashBank1StartAddress && BlockHeader.DestAddr < FlashBank2StartAddress)
            {
                currentBank = 1;
            } else if (BlockHeader.DestAddr >= FlashBank2StartAddress && BlockHeader.DestAddr < FlashBank3StartAddress)
            {
                currentBank = 2;
            } else if (BlockHeader.DestAddr >= FlashBank3StartAddress && BlockHeader.DestAddr < FlashBank4StartAddress)
            {
                currentBank = 3;
            } else if (BlockHeader.DestAddr >= FlashBank4StartAddress && BlockHeader.DestAddr <= FlashBank4EndAddress)
            {
                currentBank = 4;
            }
            // Bank 0 OTP
            else if ((BlockHeader.DestAddr >= BANK0_OTP_START) && (BlockHeader.DestAddr <= BANK0_OTP_END))
            {
                otpFlag = 1;
                currentBank = 0;
            }
            // Bank 1 OTP
            else if ((BlockHeader.DestAddr >= BANK1_OTP_START) && (BlockHeader.DestAddr <= BANK1_OTP_END))
            {
                otpFlag = 1;
                currentBank = 1;
            } 
            // Bank 2 OTP
            else if ((BlockHeader.DestAddr >= BANK2_OTP_START) && (BlockHeader.DestAddr <= BANK2_OTP_END)) 
            {
                otpFlag = 1;
                currentBank = 2;
            }
            // Bank 3 OTP
            else if ((BlockHeader.DestAddr >= BANK3_OTP_START) && (BlockHeader.DestAddr <= BANK3_OTP_END)) 
            {
                otpFlag = 1;
                currentBank = 3;
            }
            // Bank 4 OTP
            else if ((BlockHeader.DestAddr >= BANK4_OTP_START) && (BlockHeader.DestAddr <= BANK4_OTP_END)) 
            {
                otpFlag = 1;
                currentBank = 4;
            } 
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
                        wordData = (uint16_t)(DCAN_getDataFromBuffer(DCAN_DATA_SIZE_16BITS));

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
                // Block is too big to fit into our buffer so we must program it in
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
                           wordData = (uint16_t)(DCAN_getDataFromBuffer(DCAN_DATA_SIZE_16BITS));

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
                           wordData = (uint16_t)(DCAN_getDataFromBuffer(DCAN_DATA_SIZE_16BITS));
                           Buffer[j] = wordData;
                           i++;
                        }
                    }
                }

                // If writing to 512 bits of DCSM OTP containing link pointers, write 64-bits at a time
                if ((BlockHeader.DestAddr >= DCSM_OTP_Z1_LNKPTR_START) && (BlockHeader.DestAddr < DCSM_OTP_Z1_LNKPTR_END) || 
                (BlockHeader.DestAddr >= DCSM_OTP_Z2_LNKPTR_START) && (BlockHeader.DestAddr < DCSM_OTP_Z2_LNKPTR_END))
                {

                    //
                    // Fill miniBuffer with the data in Buffer in order to program the data
                    // to flash; miniBuffer takes data from Buffer, 88 words at a time.
                    //
                    for (k = 0; k < (BUFFER_SIZE / 4); k++)
                    {
                        uint16_t bufferOffset = k * 4;

                        miniBuffer[0] = Buffer[bufferOffset + 0];
                        miniBuffer[1] = Buffer[bufferOffset + 1];
                        miniBuffer[2] = Buffer[bufferOffset + 2];
                        miniBuffer[3] = Buffer[bufferOffset + 3];

                        //
                        // check that all the words have not already been written
                        //
                        if (wordsWritten < BlockHeader.BlockSize)
                        {
                            
                            uint16_t DCSM_OTP_Mask = WE_Protection_OTP_Mask[0];
                            DCSM_OTP_Write(BlockHeader.DestAddr, miniBuffer, DCSM_OTP_Mask);

                        } //check if all the words are not already written
                        BlockHeader.DestAddr += 0x4;
                        wordsWritten += 0x4;
                    } //for(int k); loads miniBuffer with Buffer elements
                } 
                else 
                {
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

                            // Set appropriate protection mask based on where we are writing
                            if (otpFlag)
                            {
                                Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROT_UO, WE_Protection_OTP_Mask[currentBank]);
                            } else 
                            {
                                //
                                // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
                                // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
                                // sectors 40-47, etc
                                //
                                Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_Mask_A[currentBank]);
                                Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_Mask_B[currentBank]);

                            }


                            oReturnCheck = Fapi_issueProgrammingCommand(
                                    (uint32_t *) BlockHeader.DestAddr, Buffer,
                                    sizeof(Buffer), 0, 0, Fapi_AutoEccGeneration);

                            while (Fapi_checkFsmForReady() == Fapi_Status_FsmBusy);

                            oFlashStatus = Fapi_getFsmStatus();

                            if (oReturnCheck != Fapi_Status_Success || oFlashStatus != 3)
                            {
                                fail++;
                            }
                        }
                        // Only verify if we are not writing to DCSM
                        if (!((BlockHeader.DestAddr >= (BANK0_OTP_START + 0x20)) && (BlockHeader.DestAddr <= BANK0_OTP_END))) 
                        {
                            oReturnCheck = Fapi_doVerify((uint32_t *) (BlockHeader.DestAddr), 4, (uint32_t*) Buffer, &oFlashStatusWord);
                            if (oReturnCheck != Fapi_Status_Success) 
                            {
                                fail++;
                            }
                        }

                    } //check if all the words are not already written
                    BlockHeader.DestAddr += 0x8;
                    wordsWritten += 0x8;
                }

                

        }
        //
        // Get the size of the next block
        //
        BlockHeader.BlockSize = (uint16_t)(DCAN_getDataFromBuffer(DCAN_DATA_SIZE_16BITS));
        wordsWritten = 0;
    }
    return;
}

// DCSM_OTP_Write is used to write 64-bits to DCSM OTP 
// This is needed because we cannot write 128 or 512 bits to the link pointers
void DCSM_OTP_Write(uint32_t destAddr, uint16_t* miniBuffer, uint32_t WE_Protection_Mask_UO)
{
    Fapi_FlashStatusType oFlashStatus;

    //
    // Error return variable
    //
    Fapi_StatusType oReturnCheck;

    //
    //program 4 words at once, 64-bits
    //
    //

    //
    // Disable erase/program protection
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROT_UO, WE_Protection_Mask_UO);

    oReturnCheck = Fapi_issueProgrammingCommand((uint32_t *) destAddr, miniBuffer,
                                                4, 0, 0, Fapi_AutoEccGeneration);

    while (Fapi_checkFsmForReady() == Fapi_Status_FsmBusy);

    oFlashStatus = Fapi_getFsmStatus();

    if (oReturnCheck != Fapi_Status_Success || oFlashStatus != 3)
    {
        Example_Error();
    }  
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

void Example_Error()
{
    asm(" ESTOP0");
}
