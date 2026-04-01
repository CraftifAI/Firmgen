//###########################################################################
//
// FILE:   cpu1brom_utils.h
//
// TITLE:  Common Macros Used and Helper APIs
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

#ifndef CPU1BROM_UTILS_H
#define CPU1BROM_UTILS_H

//
// Includes
//
#include "hw_nmi.h"
#include "sysctl.h"
#include "hw_memcfg.h"

//
// Function Prototypes
//

extern bool readNoError;

//
//Convert integer to asm NOP repeat command
//
#define CYCLES_TO_ASM(x)                              " RPT #" #x " || NOP"


//
//Macro to execute 'x' number of NOPs
//
#define NOP_CYCLES(x)                                 asm(CYCLES_TO_ASM(x))

//
//Macro for debug label
//
#define DEBUGLABEL(name) asm(" .global TSTLBL_" #name "\n" \
                             "TSTLBL_" #name ":")

//
// Macros to build 16b and 32b words
//
#define BUILD_WORD(LOW, HIGH) (((uint16_t)HIGH << 8U) | (uint16_t)LOW)
#define BUILD_DWORD(LOW1, LOW2, HIGH1, HIGH2) (((uint32_t)HIGH2 << 24U) | \
                                               ((uint32_t)HIGH1 << 16U) | \
                                               ((uint32_t)LOW2 << 8U) | (uint32_t)LOW1)

#define TI_OTP_ADDR_DIEID0              (0x72000UL)
#define TI_OTP_ADDR_DIEID1              (0x72002UL)
#define TI_OTP_ADDR_DIEID2              (0x72004UL)
#define TI_OTP_ADDR_DIEID3              (0x72006UL)
#define REVID_MASK                      0x1FUL

#define ERROR                           1U
#define NO_ERROR                        0U


//
// Set ERRORSTS pin
//
static inline void triggerERRORSTSPin(void)
{
    asm("   ESTOP0");

    //
    // Trigger ERRORSTS
    //
    EALLOW;
    HWREGH(NMI_BASE + NMI_O_ERRORSTSFRC) |= NMI_ERRORSTSFRC_ERROR;
    EDIS;
}

/**
* Double Read - 16 Bit
* Reads value in given address twice. Triggers ERRORSTS pin
* and watchdog reset in case of mismatch
*
* @param address Memory Address of desired location
* @param reset Flag to indicate the action to be taken on mismatch - whether to
*              reset the device or set error flag.
* @return uint16_t Result of read operation
*
* Design: \ref did_safety_register_doubleread_interface
* Requirement: REQ_TAG(C2000BROM-195)
*/
static inline uint16_t regDoubleRead_16(uint32_t address, bool reset)
{
    //
    //Regular HWREGH read
    //
    volatile uint16_t valueRead = HWREGH(address);

    //
    //Read value again
    //
    volatile uint16_t valueInternal = HWREGH(address);

    //
    //Check if both read attempts yield same value
    //
    if(valueInternal != valueRead)
    {
        if(reset)
        {
            asm("   ESTOP0");

            //
            //Trigger ERRORSTS pin upon failure
            //
            SysCtl_resetDevice();
        }
        else
        {
            readNoError = readNoError && false;
            valueRead = 0;
        }
    }

    return valueRead;
}

/**
* Double Read - 32 Bit
* Reads value in given address twice. Triggers ERRORSTS pin
* and watchdog reset in case of mismatch
*
* NOTE: Did not make this API Static as it is being accessed in
*       cpubrom_Init_Boot.asm as well as other .c files
* @param address Memory Address of desired location
* @param reset Flag to indicate the action to be taken on mismatch - whether to
*              reset the device or set error flag.
* @return uint32_t Result of read operation
*
* Design: \ref did_safety_register_doubleread_interface did_safety_double_read
*
* Requirement: REQ_TAG(C2000BROM-195), REQ_TAG(C2000BROM-276)
*/
static inline uint32_t regDoubleRead_32(uint32_t address, bool reset)
{
    //
    //Regular HWREGH read
    //
    volatile uint32_t valueRead = HWREG(address);

    //
    //Read value again
    //
    volatile uint32_t valueInternal = HWREG(address);

    if(valueInternal != valueRead)
    {
        if(reset)
        {
            asm("   ESTOP0");

            //
            //Trigger ERRORSTS pin upon failure
            //
            SysCtl_resetDevice();
        }
        else
        {
            readNoError = readNoError && false;
            valueRead = 0;
        }
    }

    return valueRead;
}

/**
* Read after Write - 16 Bit
* Reads value of register after performing write operation on it.
* Triggers ERRORSTS pin and watchdog reset in case of mismatch.
*
* @param address Memory Address of desired location
* @param mask    Mask value to write to specific bitfield
* @param value   Value to be written into location
* @param reset Flag to indicate the action to be taken on mismatch - whether to
*              reset the device or set error flag.
*
* Design: \ref did_safety_register_readback_afterwrite_interface did_safety_read_after_write
* Requirement: REQ_TAG(C2000BROM-194), REQ_TAG(C2000BROM-277)
*/
static inline void regReadAfterWrite_16(uint32_t address, uint16_t mask, uint16_t value, bool reset)
{
    //
    //Write Value to Register
    //
    HWREGH(address) = (value & mask);

    //
    //Read from memory
    //
    uint16_t valueInternal = (HWREGH(address) & mask);

    if(valueInternal != (value & mask))
    {
        if(reset)
        {
            asm("   ESTOP0");

            //
            //Trigger ERRORSTS pin upon failure
            //
            SysCtl_resetDevice();
        }
        else
        {
            readNoError = readNoError && false;
        }
    }

    return;
}

/**
* Read after Write - 32 Bit
* Reads value of register after performing write operation on it.
* Triggers ERRORSTS pin and watchdog reset in case of mismatch.
*
* @param address Memory Address of desired location
* @param mask    Mask value to write to specific bitfield
* @param value Value to be written into location
* @param reset Flag to indicate the action to be taken on mismatch - whether to
*              reset the device or set error flag.
*
* Design: \ref did_safety_register_readback_afterwrite_interface
* Requirement: REQ_TAG(C2000BROM-194)
*/
static inline void regReadAfterWrite_32(uint32_t address, uint32_t mask, uint32_t value, bool reset)
{
    //
    //Write Value to Register
    //
    HWREG(address) = (value & mask);

    //
    //Read from memory
    //
    uint32_t valueInternal = (HWREG(address) & mask);

    if(valueInternal != (value & mask))
    {
        if(reset)
        {
            asm("   ESTOP0");

            //
            //Trigger ERRORSTS pin upon failure
            //
            SysCtl_resetDevice();
        }
        else
        {
            readNoError = readNoError && false;
        }
    }

    return;
}

/**
* Read after Write - 32 Bit
* Reads value of register after performing write operation on it.
* Triggers ERRORSTS pin and watchdog reset in case of mismatch.
*
* @param address Memory Address of desired location
* @param mask    Mask value to write to specific bitfield
* @param value Value to be written into location
* @param reset Flag to indicate the action to be taken on mismatch - whether to
*              reset the device or set error flag.
*
* Design: \ref did_safety_register_readback_afterwrite_interface
* Requirement: REQ_TAG(C2000BROM-194)
*/
static inline void regReadAfterWrite_32bp(uint32_t address, uint32_t mask, uint32_t value, bool reset)
{
    //
    //Write Value to Register
    //
    HWREG_BP(address) = (value & mask);

    //
    //Read from memory
    //
    uint32_t valueInternal = (HWREG_BP(address) & mask);

    if(valueInternal != (value & mask))
    {
        if(reset)
        {
            asm("   ESTOP0");

            //
            //Trigger ERRORSTS pin upon failure
            //
            SysCtl_resetDevice();
        }
        else
        {
            readNoError = readNoError && false;
        }
    }

    return;
}

#endif
