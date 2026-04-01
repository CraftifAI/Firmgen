//###########################################################################
//
// FILE:   bootloader_can_timing.h
//
// TITLE:  CAN Timing Definitions.
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

#ifndef BOOTLOADER_CAN_TIMING_H_
#define BOOTLOADER_CAN_TIMING_H_

//
// CAN bit timing settings for running CAN at 100kbps with a 20MHz crystal
//
#define CAN_CALC_BITRATE   1000000UL
#define CAN_CALC_CANCLK    20000000UL
#define CAN_CALC_BITTIME   20UL

//
//                          CAN bit timing parameters
//This code does compile-time calculation of CAN bit timing parameters based on user-provided
//input settings. There are some error checks at the end, but you (the user) are ultimately
//responsible for verifying and testing the final register value. See section 12 of the CAN
//user guide in your F28M35x, F28M36x, or F28x7x TRM for an explanation of CAN timings and
//associated requirements. The main requirements are:
//
//1. One CAN bit must take between 8 and 25 time quanta.
//2. Tseg1 and Tseg2 must fit into their respective register fields.
//3. The synchronization jump width cannot be longer than either phase segment.
//
//The calculated bit timing register values is #defined as CAN_CALC_BTRREG.

//There are three user-controlled parameters that must be #defined before #including this header.
//They are (with examples):
//
//Bit rate  (bits/sec)  CAN bit rate -- must match the rest of the CAN network
//CANCLK    (hertz)     CAN module peripheral clock frequency before the bit rate prescaler
//Bit time  (Tq)        Number of time quanta per bit; the ratio of prescaled bit clock frequency to bit rate
//#define CAN_CALC_BITRATE  50000L
//#define CAN_CALC_CANCLK   10000000L
//#define CAN_CALC_BITTIME  20

//Other parameters are defined according to the instructions in section 12.2.1
//of the CAN spec. Exception: This code rounds PhSeg1 up and PhSeg2 down since a
//later sample time works better on most networks. This is implemented by rounding
//down PhSeg2 first, then computing PhSeg1 as Tphase - PhSeg2. Calculating in this
//order also lets us limit Tseg2 to 8 so it will fit in the register.
//
//BRP       -           Bit rate prescaler, which divides down CANCLK to provide the bit clock
//Tsync     (Tq)        Synchronization time delay: Fixed at 1 Tq by the CAN protocol
//Tprop     (Tq)        Propagation delay: <2 microseconds for a <1000 foot bus; assumed to be <2 Tq
//Tphase    (Tq)        Phase compensation buffer for oscillator tolerance: The remainder of the bit time
//PhSeg1    (Tq)        Phase segment 1: Pre-sample buffer; half of the phase time, rounded up (<=16)
//PhSeg2    (Tq)        Phase segment 2: Post-sample buffer; half of the phase time, rounded down (<=8)
//
#define CAN_CALC_BRP       (CAN_CALC_CANCLK / (CAN_CALC_BITRATE * CAN_CALC_BITTIME))
#define CAN_CALC_TSYNC     1UL
#define CAN_CALC_TPROP     2UL
#define CAN_CALC_TPHASE    (CAN_CALC_BITTIME - (CAN_CALC_TSYNC + CAN_CALC_TPROP))
#define CAN_CALC_PHSEG2    (((CAN_CALC_TPHASE / 2UL) <= 8UL) ? (CAN_CALC_TPHASE / 2UL) : 8UL)
#define CAN_CALC_PHSEG1    (CAN_CALC_TPHASE - CAN_CALC_PHSEG2)

//
//These parameters are then converted into register values. All parameters
//are -1 in the register:
//
//Tseg1     (Tq)        Everything between sync and sample: Tprop + PhSeg1; MUST BE LESS THAN 16
//Tseg2     (Tq)        Everything between sample and next time: PhSeg2; MUST BE LESS THAN 8
//Tsjw      (Tq)        Synchronization jump width: The lesser of 4 and PhSeg1/2
//BRPEREG    -          Upper 4 bits of bit prescaler; SUBTRACT ONE FROM BRP BEFORE COMPUTING THIS
//BRPREG     -          Lower 6 bits of bit prescaler; SUBTRACT ONE FROM BRP BEFORE COMPUTING THIS
//
#define CAN_CALC_TSEG1     ((CAN_CALC_PHSEG1 + CAN_CALC_TPROP) - 1UL)
#define CAN_CALC_TSEG2     (CAN_CALC_PHSEG2 - 1UL)
#define CAN_CALC_TSJW      (((CAN_CALC_PHSEG2 > 4UL) ? 4UL : CAN_CALC_PHSEG2) - 1UL)
#define CAN_CALC_BRPEREG   ((CAN_CALC_BRP - 1UL) / 64UL)
#define CAN_CALC_BRPREG    ((CAN_CALC_BRP - 1UL) % 64UL)

//
//The final result -- a value to write to the bit timing register
//
#define CAN_CALC_BTRREG        ((CAN_CALC_BRPEREG<<16U) | (CAN_CALC_TSEG2<<12U) | (CAN_CALC_TSEG1<<8U) | (CAN_CALC_TSJW<<6U) | CAN_CALC_BRPREG) // 0<<16 | 0x1110 << 12 | 0x1111 << 8 | 0x100 << 6 | 0x111 = 1110 1111 0100 0111

//
// Error and sanity checks
//
#if (CAN_CALC_BITRATE > 1000000)
    #error "Can config error: Bit rate cannot exceed 1000000 Hz!
#endif
#if (CAN_CALC_BITTIME < 8)
    #error "CAN config error: Bit time cannot be less than 8 time quanta!"
#elif (CAN_CALC_BITTIME > 25)
    #error "CAN config error: Bit time cannot exceed 25 time quanta!"
#endif
#if (CAN_CALC_BRP * CAN_CALC_BITTIME * CAN_CALC_BITRATE != CAN_CALC_CANCLK)
    #error "CAN config error: These settings require a fractional prescaler! (CAN_CALC_BITRATE * CAN_CLK_BITTIME) must evenly divide CAN_CALC_CANCLK.
#endif
#if (CAN_CALC_TSEG1 >= 16)
    #error "CAN config error: Tseg1 register value cannot be greater than 15! Something is broken in the calculation routine."
#endif
#if (CAN_CALC_TSEG2 >= 8)
    #error "CAN config error: Tseg2 register value cannot be greater than 7! Something is broken in the calculation routine."
#endif
#if (CAN_CALC_TSJW > CAN_CALC_PHSEG1 || CAN_CALC_TSJW > CAN_CALC_PHSEG2)
    #error "CAN config error: Tsjw cannot be greater than PhSeg1 or PhSeg2! Something is broken in the calculation routine."
#endif

#endif // BOOTLOADER_CAN_TIMING_H_

//
// End of file
//
