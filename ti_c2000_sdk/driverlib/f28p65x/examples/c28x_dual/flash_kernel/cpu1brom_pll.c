//###########################################################################
//
// FILE:    cpu1brom_pll.c
//
// TITLE:   PLL Enable and Power up Functions
//
//###########################################################################
// $TI Release: $
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

//
// Included Files
//
#include "cpu1bootrom.h"
#include "cpu1brom_utils.h"

extern uint32_t CPU1BROM_bootStatus;
uint32_t pllMultiplier, pllDivider;

void CPU1BROM_disablePLL(void)
{
    //
    // Bypass PLL (if enabled)
    //
    if(0UL != (HWREG(CPUSYS_BASE + SYSCTL_O_RESC) &
              ((uint32_t)SYSCTL_RESC_POR | (uint32_t)SYSCTL_RESC_XRSN )))
    {
        EALLOW;

        // bypass PLL here
        if((HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) & SYSCTL_SYSPLLCTL1_PLLCLKEN) != 0UL)
        {
            HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLCLKEN;
            //
            // Delay 120 cycles after bypass
            //
            NOP_CYCLES(120);
        }

        //
        // Set PLL multiplier to 0x0
        //
        HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) = 0;

        //
        // Set the divider to /1
        //
        HWREG(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) = 0;

        //
        // Turn off PLL and delay for power down
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;

        //
        // Delay 60 cycles after power down
        //
        NOP_CYCLES(60);

        EDIS;
    }
}

/**
* CPU1BROM_triggerSysPLLLock - Power up and lock the SYS PLL.
* The "divider" configured in this routine is PLL Output Divider (ODIV)
* and not "SYSCLKDIVSEL".
*
*
* \brief PLL Lock function
*
* Design: \ref did_trigger_apll_lock_usecase did_enable_pll_lock_algo
*              did_pll_lock_fail_status_algo
* Requirement: REQ_TAG(C2000BROM-214), REQ_TAG(C2000BROM-164)
*
* PLL Lock function
*
*/
void CPU1BROM_triggerSysPLLLock(uint32_t clkSource, uint32_t multiplier, uint32_t divider)
{
	pllMultiplier = multiplier;
	pllDivider = divider;

	EALLOW;

    //
    // Bypass PLL from SYSCLK
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLCLKEN;

    //
    // Delay of at least 120 OSCCLK cycles required post PLL bypass
    //
    NOP_CYCLES(120);

    //
    // Use INTOSC2 (~10 MHz) as the main PLL clock source
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL1) &= ~SYSCTL_CLKSRCCTL1_OSCCLKSRCSEL_M;
	NOP_CYCLES(45);
	HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL1) |= (clkSource & SYSCTL_CLKSRCCTL1_OSCCLKSRCSEL_M);

    //
    // Delay of at least 300 OSCCLK cycles after clock source change
    //
	NOP_CYCLES(150);
    NOP_CYCLES(150);

	//
	// Turn off PLL and delay for power down
	//
	HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;

    //
    // Delay 60 cycles to power down
    //
	NOP_CYCLES(60);

    //
    // Set PLL Multiplier and Output Clock Divider (ODIV)
    //
    HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) =
                     ((HWREG(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) &
                     ~(SYSCTL_SYSPLLMULT_ODIV_M | SYSCTL_SYSPLLMULT_IMULT_M)) |
                              (divider << SYSCTL_SYSPLLMULT_ODIV_S) |
                              (multiplier << SYSCTL_SYSPLLMULT_IMULT_S));

	//
	// Enable the sys PLL
	//
	HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLEN;

	//
	// 200 Cycles after enabling PLL
	//
	NOP_CYCLES(200);

    EDIS;
}

/**
* \brief Switch to PLL output
*
* Design: \ref did_safety_switch_to_pll_clock_usecase
* Requirement: REQ_TAG(C2000BROM-215), REQ_TAG(C2000BROM-164)
*
* PLL Lock function
*
*/
uint16_t CPU1BROM_switchToPLL(uint32_t pllInputClockMhz)
{
	uint16_t count = 1024; // timeout
	uint16_t dccStatus;
	uint32_t dccCnt0Seed, dccCnt1Seed, dccValid0Seed;


    //
    // Setup DCC Values
    //
    dccCnt0Seed = 104U;
    dccValid0Seed = 32U;

    //
    // + below is to convert bit field values to actual divider values
    //
	// Conterseed1 = window * (Fclk1/Fclk0)
	// window - 120, fclk0 is 10Mhz
	dccCnt1Seed = (120UL * (((pllInputClockMhz * pllMultiplier)/(pllDivider + 1UL)) / 10UL));

	//
	// Wait for the SYSPLL lock counter
	//
	while(((HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLSTS) &
			SYSCTL_SYSPLLSTS_LOCKS) == 0U) && (count != 0U))
	{
		count--;
	}

	//
	// Using DCC to verify the PLL clock
	//
	SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_DCC0);
	dccStatus = BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_INTOSC2,
											  (DCC_Count1ClockSource)DCC_COUNT1SRC_PLL,
											  dccCnt0Seed, dccCnt1Seed, dccValid0Seed);
	SysCtl_disablePeripheral(SYSCTL_PERIPH_CLK_DCC0);

	//
    // If DCC failed to verify PLL clock is running correctly, update status
	// and power down PLL
    //
    if(ERROR == dccStatus)
    {
        //
        // Turn off PLL and delay for power down
        //
        EALLOW;
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;
        EDIS;

        //
        // Delay 120 cycles
        //
        NOP_CYCLES(120);
    }
	else
	{
	    //
	    // Switch sysclk to PLL clock
	    //
		EALLOW;
	    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLCLKEN;
        EDIS;
        
        //
        // ~200 PLLSYSCLK delay to allow voltage regulator to stabilize
        //
        NOP_CYCLES(120);
    }
    return (dccStatus);
}

uint16_t BROMDCC_verifySingleShotClock(DCC_Count0ClockSource clk0src,
                                       DCC_Count1ClockSource clk1src, uint32_t dccCounterSeed0,
                                       uint32_t dccCounterSeed1, uint32_t dccValidSeed0)
{
    uint32_t j = dccCounterSeed1;
    uint16_t status;

    //
    // Clear DONE and ERROR flags
    //
    EALLOW;
    HWREGH(DCC0_BASE + DCC_O_STATUS) = 3U;
    EDIS;

    //
    // Disable DCC
    //
    DCC_disableModule(DCC0_BASE);

    //
    // Disable Error Signal
    //
    DCC_disableErrorSignal(DCC0_BASE);

    //
    // Disable Done Signal
    //
    DCC_disableDoneSignal(DCC0_BASE);

    //
    // Configure Clock Source0 to whatever set as a clock source for PLL
    //
    DCC_setCounter0ClkSource(DCC0_BASE, clk0src);

    //
    // Configure Clock Source1 to PLL
    //
    DCC_setCounter1ClkSource(DCC0_BASE, clk1src);

    //
    // Configure COUNTER-0, COUNTER-1 & Valid Window
    //
    DCC_setCounterSeeds(DCC0_BASE, dccCounterSeed0, dccValidSeed0,
                        dccCounterSeed1);

    //
    // Enable Single Shot mode
    //
    DCC_enableSingleShotMode(DCC0_BASE, DCC_MODE_COUNTER_ZERO);

    //
    // Enable DCC to start counting
    //
    DCC_enableModule(DCC0_BASE);

    //
    // Wait until Error or Done Flag is generated or timeout
    //
    while(((HWREGH(DCC0_BASE + DCC_O_STATUS) &
           (DCC_STATUS_ERR | DCC_STATUS_DONE)) == 0U) && (j != 0U))
    {
		// j is decremented to give enough timeout for HW to complete 
		// the comparison. The result will be determined from the values
		// in status register.
       j--;
    }

    //
    // Returns NO_ERROR if DCC completes without error
    //
    if((HWREGH(DCC0_BASE + DCC_O_STATUS) &
            (DCC_STATUS_ERR | DCC_STATUS_DONE)) == DCC_STATUS_DONE)
	{
		status = NO_ERROR;
	}
	else
	{
		status = ERROR;		
	}

    return status;
}

//*****************************************************************************
//
// Enables the SYS PLL
//
// Parameters:
//  multiplier - Requested multiplier for SYS PLL configuration
//  divider    - Required divider for SYS PLL configuration
//
// This function enables, configures, and switches to the SYS PLL.
//
// Return: Returns 0x0 upon successfully enabling and switching to SYS PLL.
//         Returns 0xFFFF if SYS PLL doesn't successfully enable
//
//*****************************************************************************
uint16_t CPU1BROM_enableSysPLL(uint16_t multiplier, uint16_t divider)
{
    uint32_t entryAddress = 0xFFFFFFFFUL;
    uint16_t i, j;
    uint16_t timeout = 512U;
    uint32_t dccCnt0Seed, dccCnt1Seed, dccValid0Seed;
    uint16_t dccStatus = 0U;
    uint16_t dividerLocal = divider;

    //
    // Setup DCC Values
    //
    dccCnt0Seed = 94U;
    dccCnt1Seed = (100UL * multiplier);
    dccValid0Seed = 12U;

    //
    // Use INTOSC2 (~10 MHz) as the main PLL clock source
    //
    EALLOW;
    HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL1) &= ~SYSCTL_CLKSRCCTL1_OSCCLKSRCSEL_M;
    EDIS;

    //
    // 300 Cycles delay required after changing clock source
    //
    NOP_CYCLES(150);
    NOP_CYCLES(150);

    //
    // Attempt to Lock the PLL five times. This helps ensure a successful start.
    //
    for(i = 0U; i < 5U; i++)
    {
        EALLOW;

        //
        // Turn off PLL and delay for power down
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) &= ~SYSCTL_SYSPLLCTL1_PLLEN;
        NOP_CYCLES(120); //Delay 120 cycles

        //
        // Write multiplier
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLMULT) = multiplier;

        //
        // Enable the sys PLL
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLEN;

        //
        // 200 NOPs to enable SYSPLL
        //
        NOP_CYCLES(200);

        EDIS;

        //
        // Set time out to 512 cycles
        //
        j = timeout;

        //
        // Wait for the SYSPLL lock counter
        //
        while(((HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLSTS) &
                SYSCTL_SYSPLLSTS_LOCKS) == 0U) && (j != 0U))
        {
            j--;
        }

        //
        // Padding lock time before taking DCC measurement by 512 cycles
        //
        j = timeout;
        while(j != 0U)
        {
            j--;
        }

        //
        // Using DCC to verify the PLL clock
        //
        SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_DCC0);
        NOP_CYCLES(5);
        dccStatus = BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_INTOSC2,
                                                  (DCC_Count1ClockSource)DCC_COUNT1SRC_PLL,
                                                  dccCnt0Seed, dccCnt1Seed, dccValid0Seed);
        //
        // Clear DCC Status flags and disable DCC
        //
        EALLOW;
        HWREGH(DCC0_BASE + DCC_O_STATUS) = (DCC_STATUS_ERR | DCC_STATUS_DONE);
        EDIS;
        SysCtl_disablePeripheral(SYSCTL_PERIPH_CLK_DCC0);

        //
        // Break the Loop since PLL is running correctly
        //
        if(dccStatus == NO_ERROR)
        {
            break;
        }
    }


    //
    // Convert divider value to value required for register
    //
    switch(dividerLocal)
    {
        case 2U: dividerLocal = 1U;
        break;
        case 4U: dividerLocal = 2U;
        break;
        case 6U: dividerLocal = 3U;
        break;
        default: dividerLocal = 0U; //divider == 1
        break;
    }

    EALLOW;

    //
    // Set PLLSYSCLKDIV
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) = ((HWREGH(CLKCFG_BASE + SYSCTL_O_SYSCLKDIVSEL) &
                                                    ~SYSCTL_SYSCLKDIVSEL_PLLSYSCLKDIV_M) | dividerLocal);

    //
    // Turn on PLL clock
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_SYSPLLCTL1) |= SYSCTL_SYSPLLCTL1_PLLCLKEN;
    NOP_CYCLES(200); //Delay 25 cycles
    EDIS;

    return(BROM_PLL_CONFIG_SUCCESS);
}

//*****************************************************************************
//
// Enables the AUX PLL
//
// Parameters:
//  multiplier - Requested multiplier for AUX PLL configuration
//  divider    - Required divider for AUX PLL configuration
//
// This function enables, configures, and switches to the AUX PLL.
//
// Return: Returns 0x0 upon successfully enabling and switching to AUX PLL.
//         Returns 0xFFFF if AUX PLL doesn't successfully enable
//
//*****************************************************************************
uint16_t CPU1BROM_enableAuxPLL(uint32_t clkSource, uint16_t multiplier, uint32_t odiv,
                               uint16_t divider, uint32_t dccCnt0Seed,
                               uint32_t dccValid0Seed, uint32_t dccCnt1Seed)
{
    uint32_t entryAddress = 0xFFFFFFFFUL;
    uint16_t i, j;
    uint16_t timeout = 512U;
    uint16_t dccStatus = 0U;
    uint16_t dividerLocal = divider;

    //
    // CPU1 Patch/Escape Point 9
    //
    if(SW_PATCH_POINT_KEY == swPatchKey)
    {
        entryAddress = CPU1BROM_TI_OTP_ESCAPE_POINT_9;
        if((entryAddress < 0xFFFFU) && (entryAddress > 0x0000U))
        {
            //
            // If OTP is programmed, then call OTP patch function
            //
            EXECUTE_ESCAPE_POINT(entryAddress);
        }
    }

    EALLOW;

    //
    // Turn off AUXPLL and delay for it to power down.
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_AUXPLLCTL1) &= ~SYSCTL_AUXPLLCTL1_PLLEN;

    //
    // 60 cycle delay after powering down PLL
    //
    NOP_CYCLES(60);

    EDIS;

    //
    // Use XTAL as the aux PLL clock source
    //

    if(CLK_XTAL == clkSource)
    {
        EALLOW;
        HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= ~SYSCTL_CLKSRCCTL2_AUXOSCCLKSRCSEL_M;

        //
        // 300 cycles after clock source is changed
        //
        NOP_CYCLES(150);
        NOP_CYCLES(150);

        HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) |= 1U;
        EDIS;
    }

    else
    {
        EALLOW;
        HWREGH(CLKCFG_BASE + SYSCTL_O_CLKSRCCTL2) &= ~SYSCTL_CLKSRCCTL2_AUXOSCCLKSRCSEL_M;

        //
        // 300 cycles after clock source is changed
        //
        NOP_CYCLES(150);
        NOP_CYCLES(150);


        EDIS;
    }

    //
    // Lock the PLL five times. This helps ensure a successful start.
    //
    for(i = 0U; i < 5U; i++)
    {
        EALLOW;

        //
        // Write multiplier and odiv
        //
        HWREG(CLKCFG_BASE + SYSCTL_O_AUXPLLMULT) =
                         ((HWREG(CLKCFG_BASE + SYSCTL_O_AUXPLLMULT) &
                         ~(SYSCTL_AUXPLLMULT_ODIV_M | SYSCTL_AUXPLLMULT_IMULT_M)) |
                                  (odiv << SYSCTL_AUXPLLMULT_ODIV_S) |
                                  (multiplier << SYSCTL_AUXPLLMULT_IMULT_S));

        //
        // Enable AUXPLL
        //
        HWREGH(CLKCFG_BASE + SYSCTL_O_AUXPLLCTL1) |= SYSCTL_AUXPLLCTL1_PLLEN;


        //
        // 200 cycles after PLL enable
        //
        NOP_CYCLES(200);

        EDIS;

        //
        // set time out to 512 cycles
        //
        j = timeout;

        //
        // Wait for the AUXPLL lock counter
        //
        while(((HWREGH(CLKCFG_BASE + SYSCTL_O_AUXPLLSTS) &
                SYSCTL_AUXPLLSTS_LOCKS) != 1U)  && (j != 0U))
        {
            j--;
        }

        //
        // Padding lock time before taking DCC measurement by 512 cycles
        //
        j = timeout;
        while(j != 0U)
        {
            j--;
        }

        //
        // Using DCC to verify the PLL clock
        //
        SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_DCC0);
        NOP_CYCLES(5);
		if(clkSource == CLK_INTOSC2)
		{
			dccStatus = BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_INTOSC2,
                                                  (DCC_Count1ClockSource)DCC_COUNT1SRC_AUXPLL,
                                                  dccCnt0Seed, dccCnt1Seed, dccValid0Seed);
		}
		else if(clkSource == CLK_XTAL)
		{
			dccStatus = BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_XTAL,
									  (DCC_Count1ClockSource)DCC_COUNT1SRC_AUXPLL,
									  dccCnt0Seed, dccCnt1Seed, dccValid0Seed);
		}
		else
		{
			// to avoid misra
		}
	
        //
        // Clear DCC Status flags and disable DCC
        //
        EALLOW;
        HWREGH(DCC0_BASE + DCC_O_STATUS) = (DCC_STATUS_ERR | DCC_STATUS_DONE);
        EDIS;
        SysCtl_disablePeripheral(SYSCTL_PERIPH_CLK_DCC0);

        //
        // Break the Loop since PLL is running correctly
        //
        if(dccStatus == NO_ERROR)
        {
            break;
        }
    }

    //
    // If DCC failed to verify PLL clock is running correctly, return error
    //
    if(dccStatus == ERROR)
    {
        //
        // Update boot status indicating failure
        //
        CPU1BROM_bootStatus &= ~CPU1_BOOTROM_PLL_ENABLE_SUCCESS;

        //
        // Turn off AUXPLL and delay for it to power down.
        //
        EALLOW;
        HWREGH(CLKCFG_BASE + SYSCTL_O_AUXPLLCTL1) &= ~SYSCTL_AUXPLLCTL1_PLLEN;
        EDIS;


        //
        // 60 cycles after PLL power down
        //
        NOP_CYCLES(60);



        return(BROM_PLL_CONFIG_ERROR);
    }

    //
    // Convert divider value to value required for register
    //
    switch(dividerLocal)
    {
        case 2U: dividerLocal = 1U;
        break;
        case 4U: dividerLocal = 2U;
        break;
        case 5U: dividerLocal = 5U;
        break;
        case 6U: dividerLocal = 6U;
        break;
        case 8U: dividerLocal = 3U;
        break;
        default: dividerLocal = 0U; //divider == 1
        break;
    }

    EALLOW;

    //
    // Set AUXPLLDIV
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_AUXCLKDIVSEL) = ((HWREGH(CLKCFG_BASE + SYSCTL_O_AUXCLKDIVSEL) &
                                                    ~SYSCTL_AUXCLKDIVSEL_AUXPLLDIV_M) | dividerLocal);

    //
    // Turn on aux PLL clock
    //
    HWREGH(CLKCFG_BASE + SYSCTL_O_AUXPLLCTL1) |= SYSCTL_AUXPLLCTL1_PLLCLKEN;

    //
    // 200 cycles after PLL enable
    //
    NOP_CYCLES(200);

    EDIS;

    return(BROM_PLL_CONFIG_SUCCESS);
}

//*****************************************************************************
// Uses DCC0 to verify the XTAL frequency using INTOSC2 as reference clock
// Input parameters:
//      freq - XTAL frequency to be verified
//      sysclk_freq - Frequency of device system clock
// Return value:
//      True on successful verification of XTAL frequency
//      False if frequency is outside 5% tolerance
// Notes:
//      Tolerance is aggregate of INTOSC2, XTAL and DCC tolerances
//*****************************************************************************
bool Device_verifyXTAL(float32_t freq, float32_t freq_sysclk)
{
    float32_t total_error;
    float32_t window;
    float32_t count0;
    float32_t valid;
    float32_t count1;
    bool retval;

    // Async. error (In Clock0 cycles)
    if(freq > 10.0F)
    {
        total_error = ((2.0F * (freq / 10.0F)) +
                                 (2.0F * (freq_sysclk / freq)));
    }
    else
    {
        total_error = (2.0F + (2.0F * (freq_sysclk / freq)));
    }

    total_error += 8.0F;      // Digitization error = 8 Clock0 cycles

    window = total_error / (0.01F * 1.0F);      // Window in Clock0 cycles

    // Error due to variance in clock frequency =
    // window * (Allowable Frequency Tolerance (in %) / 100)
    total_error += window * (4.0F / 100.0F);

    // DCC counter configuration
    count0 = (window - total_error);
    valid  = (2.0F * total_error);
    count1 = (window * (10.0F / freq));

    SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_DCC0);    //Enable DCC0 clock
    NOP_CYCLES(5);  //Insert at-least 5 cycles delay after enabling the peripheral clock

    // Configure XTAL as CLKSRC0 and INTOSC2 as CLKSRC1
    // Fclk0 = XTAL frequency (input parameter)
    // Fclk1 = INTOSC2 frequency = 10MHz +/-3%
    // Configuring DCC error tolerance of +/-1%
    // XTAL tolerance of up to +/-1%
    if(BROMDCC_verifySingleShotClock((DCC_Count0ClockSource)DCC_COUNT0SRC_XTAL,
                                     (DCC_Count1ClockSource)DCC_COUNT1SRC_INTOSC2,
                                     (uint32_t)count0, (uint32_t)count1, (uint32_t)valid) == NO_ERROR)
    {
        retval = true;
    }
    else
    {
        retval = false;
    }
    SysCtl_disablePeripheral(SYSCTL_PERIPH_CLK_DCC0);
    return(retval);
}

//
// End of File
//
