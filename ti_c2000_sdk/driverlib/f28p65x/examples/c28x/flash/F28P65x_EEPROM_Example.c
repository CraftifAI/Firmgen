
//#############################################################################
//
// FILE:   F28P65x_EEPROM_Example.c
//
//#############################################################################


#include "EEPROM_Config.h"
#include <stdio.h>

extern uint32 Flash_CPUScaleFactor;

void main(void)
{
    uint16 EEPROMConfigCheck;
    uint32 i;
    Fapi_StatusType oReturnCheck;


    // Initialize device clock and peripherals
    // Copy the Flash initialization code from Flash to RAM
    // Copy the Flash API from Flash to RAM
    // Configure Flash wait-states, fall back power mode, performance features
    // and ECC
    Device_init();

    // Initialize GPIO
    Device_initGPIO();

    // Initialize PIE and clear PIE registers. Disables CPU interrupts.
    Interrupt_initModule();

    // Initialize the PIE vector table with pointers to the shell Interrupt
    // Service Routines (ISR).
    Interrupt_initVectorTable();

    // Enable Global Interrupt (INTM) and realtime interrupt (DBGM)
    EINT;
    ERTM;


    // This should be done in Main file, but currently the example will fail
    // without this initialization.
    // At 200MHz, execution wait-states for external oscillator is 4. Modify the
    // wait-states when the system clock frequency is changed.
    Flash_initModule(FLASH0CTRL_BASE, FLASH0ECC_BASE, 4);

    EALLOW;
#ifdef CPU1
    IPC_claimFlashSemaphore(IPC_FLASHSEM_OWNER_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK0, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK1, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK2, SYSCTL_CPUSEL_CPU1);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK3, SYSCTL_CPUSEL_CPU2);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK4, SYSCTL_CPUSEL_CPU2);

    // Pump access must be gained by the core using pump semaphore
#elif  defined(CPU2)
    IPC_claimFlashSemaphore(IPC_FLASHSEM_OWNER_CPU2);
#endif

    // NOTE: This will be different for every device
    // This is set up for F28P650DK9
    // Initialize the Flash API by providing the Flash register base address
    // and operating frequency(in MHz).
    // This function is required to initialize the Flash API based on System
    // frequency before any other Flash API operation can be performed.
    // This function must also be called whenever System frequency or RWAIT is
    // changed.   
    oReturnCheck = Fapi_initializeAPI(FlashTech_CPU0_BASE_ADDRESS,
                                      DEVICE_SYSCLK_FREQ/1000000U);

    if(oReturnCheck != Fapi_Status_Success)
    {
        // Check Flash API documentation for possible errors
        Sample_Error();
    }

    // Initialize the Flash banks and FMC for erase and program operations.
    // Fapi_setActiveFlashBank() function sets the Flash banks and FMC for
    // further Flash operations to be performed on the banks.
    oReturnCheck = Fapi_setActiveFlashBank(Fapi_FlashBank0);

    if(oReturnCheck != Fapi_Status_Success)
    {
        // Check Flash API documentation for possible errors
        Sample_Error();
    }

    // Configure EEPROM
    EEPROMConfigCheck = EEPROM_Config_Check();

    // Check if EEPROM configuration is valid
    // 0 indicates proper configuration
    // See user guide for more detailed return codes
    if (EEPROMConfigCheck == 0xFFFF || EEPROMConfigCheck == 0xEEEE || EEPROMConfigCheck == 0xCCCC) {

        Sample_Error();

    }


    // Declare loop limit that will allow the EEPROM to show full functionality (fill, erase, fill)
    uint32 Loop_Limit;

    // If programming single byte, find correct pointer and set test data
    #ifdef _64_BIT_MODE

    // Declare Write Buffer
    uint16 Write_Buffer[4] =  {0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF};

    // Initialize Read Buffer
    uint16 Read_Buffer[4] = {0};

    EEPROM_Get_64_Bit_Data_Address();

    // (NUM_EEPROM_SECTORS * FLASH_SECTOR_SIZE) / 4 would allow EEPROM to be filled once, so we double it
    // to display full functionality
    Loop_Limit = (NUM_EEPROM_SECTORS * FLASH_SECTOR_SIZE) / 2;

    // If programming multiple bytes, set example data in Write_Buffer
    #else

    // Declare Write Buffer
    uint16 Write_Buffer[EEPROM_PAGE_DATA_SIZE] = {[0 ... EEPROM_PAGE_DATA_SIZE-1] = 0xFFFF};

    // Initialize Read Buffer
    uint16 Read_Buffer[DATA_SIZE] = {0};

    // Fill up the write buffer with filler data
    for(i=0;i<DATA_SIZE;i++)
    {
        Write_Buffer[i] = i;
    }

    // Initialize loop limit that will allow the EEPROM to show full functionality (fill, erase, fill)
    Loop_Limit = NUM_EEPROM_BANKS * NUM_EEPROM_PAGES * 2;

    #endif

    // Begin loop to write data to emulated EEPROM
    for(i=0;i<Loop_Limit;i++)
    {

        // If programming single byte, call single byte function
        #ifdef _64_BIT_MODE
        Write_Buffer[0] = 1;
        Write_Buffer[1] = 2;
        Write_Buffer[2] = 3;
        Write_Buffer[3] = 4;

        EEPROM_Program_64_Bits(4, Write_Buffer);
        EEPROM_Read(Read_Buffer);

        #else // If programming multiple bytes, call write function
        EEPROM_Write(Write_Buffer);
        EEPROM_Read(Read_Buffer);
        #endif
    }

#ifdef PAGE_MODE
    EEPROM_GetValidBank(1);                 // Find Current Bank and Current Page
    EEPROM_UpdatePageStatus();              // Update Page Status of previous page
    EEPROM_UpdateBankStatus();              // Update Bank Status of current and previous bank 
#endif

    // Release the pump access
    IPC_releaseFlashSemaphore();

    Example_Done();

}



