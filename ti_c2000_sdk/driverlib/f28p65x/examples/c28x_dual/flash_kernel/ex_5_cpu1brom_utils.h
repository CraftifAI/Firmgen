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
//Convert integer to asm NOP repeat command
//
#define CYCLES_TO_ASM(x)                              " RPT #" #x " || NOP"


//
//Macro to execute 'x' number of NOPs
//
#define NOP_CYCLES(x)                                 asm(CYCLES_TO_ASM(x))

//
// Macros to build 16b and 32b words
//
#define BUILD_WORD(LOW, HIGH) (((uint16_t)HIGH << 8U) | (uint16_t)LOW)
#define BUILD_DWORD(LOW1, LOW2, HIGH1, HIGH2) (((uint32_t)HIGH2 << 24U) | \
                                               ((uint32_t)HIGH1 << 16U) | \
                                               ((uint32_t)LOW2 << 8U) | (uint32_t)LOW1)

#define ERROR                           1U
#define NO_ERROR                        0U

#endif
