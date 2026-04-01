
//#############################################################################
//
// FILE:   F28P65x_EEPROM.c
//
//#############################################################################

#include "EEPROM_Config.h"                   // Include EEPROM Config

// Global Variables
uint16 *Bank_Pointer;
uint16 *Page_Pointer;
uint16 *Sector_End;
uint32 WE_Protection_A_Mask;
uint32 WE_Protection_B_Mask;
uint32 Bank_Size;
uint16 Bank_Counter = 0;
uint16 Page_Counter = 0;
uint16 Bank_Status[8] = {0};
uint16 Page_Status[8] = {0};
uint16 Erase_Inactive_Unit = 0;
uint16 Erase_Blank_Check = 0;
uint16 NUM_EEPROM_SECTORS;
uint16 Empty_EEPROM = 1;


// Edit this to select Flash Sector location for EEPROM Emulation
// NOTE: INSERT FIRST AND LAST SECTOR NUMBERS ONLY
// Example: To use sectors 1-10, insert {1,10}
// Example: To only use sector 1, insert {1,1}
uint16 FIRST_AND_LAST_SECTOR[2] = {1,1};


//######################### EEPROM_Config ############################
// The purpose of this function is to configure Write/Erase protection masks
// used by the Flash API and check inputs in EEPROM_Config.h for validity.
// If an invalid configuration is encountered, a non-zero code is returned.

int EEPROM_Config_Check(void){

    // FATAL ERRORS

    // Check if using Flash Bank 0 for EEPROM Emulation. If Flash API is
    // running from Flash Bank 0, the code will not execute. User can
    // change the linker command file as needed to do EEPROM on Flash Bank 0
    if (FLASH_BANK_SELECT == FlashBank0StartAddress)
    {
        return 0xFFFF;
    }

    // If using Bank 2
    else if (FLASH_BANK_SELECT == FlashBank2StartAddress) // If using Bank 2
    {
        #if !defined(F28P65xDKx) && !defined(F28P65xSKx) && !defined(F28P65xSHx)   // Verify that appropriate device is being used
            return 0xFFFF;
        #endif
    } 

    else if (FLASH_BANK_SELECT == FlashBank3StartAddress) // If using Bank 3
    {
        #if !defined(F28P65xDKx) && !defined(F28P65xSKx)   // Verify that appropriate device is being used
            return 0xFFFF;
        #endif
    } 
    else if (FLASH_BANK_SELECT == FlashBank4StartAddress) // If using Bank 4
    {
        #if !defined(F28P65xDKx) && !defined(F28P65xSKx) && !defined(F28P65xSHx)   // Verify that appropriate device is being used
            return 0xFFFF;
        #endif
    }

    // Derive the number of Flash sectors for EEPROM emulation
    NUM_EEPROM_SECTORS = FIRST_AND_LAST_SECTOR[1] - FIRST_AND_LAST_SECTOR[0] + 1;

    // If invalid amount of EEPROM sectors defined, return error code
    if (NUM_EEPROM_SECTORS > NUM_FLASH_SECTORS || NUM_EEPROM_SECTORS == 0)
    {
        return 0xEEEE;
    }

    if (NUM_EEPROM_SECTORS > 1)
    {

        // Check if SECTOR_NUMBERS is sorted in increasing order and doesn't have duplicates
        if (FIRST_AND_LAST_SECTOR[1] <= FIRST_AND_LAST_SECTOR[0])
        {
            return 0xEEEE;
        }

        // Check if SECTOR_NUMBERS contains invalid sector
        if (FIRST_AND_LAST_SECTOR[0] > NUM_FLASH_SECTORS - 1)
        {
            return 0xEEEE;
        }
        if (FIRST_AND_LAST_SECTOR[1] > NUM_FLASH_SECTORS - 1 || FIRST_AND_LAST_SECTOR[1] < 1)
        {
            return 0xEEEE;
        }
    } else // If only one sector, validate it is input properly
    {

        // Verify that the only sector is valid
        if (FIRST_AND_LAST_SECTOR[0] > NUM_FLASH_SECTORS - 1) {
            return 0xEEEE;
        }
    }


#ifdef PAGE_MODE

    // Calculate size of each EEPROM Bank (16 bit words)
    Bank_Size = 8 + ((EEPROM_PAGE_DATA_SIZE + 8) * NUM_EEPROM_PAGES);

    // Calculate amount of available space (16 bit words)
    uint32 Available_Words = NUM_EEPROM_SECTORS * FLASH_SECTOR_SIZE;

    // Check if size of EEPROM Banks and Pages will fit in EEPROM sectors
    if (Bank_Size * NUM_EEPROM_BANKS > Available_Words)
    {
        return 0xCCCC;
    }

#endif

    // WARNINGS (Not fatal errors)

    uint16 Warning_Flags = 0;

#ifdef PAGE_MODE
    // Notify for extra space (more than one bank leftover)
    if (Available_Words - (Bank_Size * NUM_EEPROM_BANKS ) >= Bank_Size)
    {
        Warning_Flags += 1;
    }

    // Notify if Page is less than 8 16-bit words
    // This wastes Flash space because the _64_BIT_MODE could be used to achieve the same effect; configuration should be optimized.
    // However, emulation can continue.
    if (EEPROM_PAGE_DATA_SIZE < 5)
    {
        Warning_Flags += 2;
    }
#endif

    // If not using a multiple of 8 sectors in the 32-127 range, write protection cannot be properly configured.
    // In sectors 32-127, the write protection mask used by the API assigns 8 sectors to each of the first 12 bits.
    // Therefore, unused sectors in-between multiples of 8 should not be used for critical data.
    // Example: Using sectors 32-35 requires write protection to be disabled for sectors 32-39.
    //          Thus, any data stored in sectors 36-39 is not protected. See the Flash API User Guide for more details.

    // If using any sectors from 32-127
    if (FIRST_AND_LAST_SECTOR[1] > 31) {

        // If all sectors use protection mask B
        if (FIRST_AND_LAST_SECTOR[0] > 31)
        {
            // If using less than 8 sectors
            if (NUM_EEPROM_SECTORS < 8) {
                Warning_Flags += 4;
            } else {
                // If sectors don't include all sectors represented by 1 bit in the protection mask
                if ((FIRST_AND_LAST_SECTOR[0] % 8) != 0 || ((FIRST_AND_LAST_SECTOR[1] + 1) % 8 != 0))
                {
                    Warning_Flags += 4;
                }
            }
        } else
        { // If only last sector is using protection mask B

            // If not a multiple of 8
            if ((FIRST_AND_LAST_SECTOR[1] + 1) % 8 != 0)
            {
                Warning_Flags += 4;
            }
        }
    }

    // Reset Bank Pointer to beginning of EEPROM Unit
    RESET_BANK_POINTER;

    // Configure Write/Erase Protection Masks used by the Flash API
    uint64 WE_Protection_AB_Mask = Configure_Protection_Masks(FIRST_AND_LAST_SECTOR, NUM_EEPROM_SECTORS);

    // Assign individual protection masks accordingly
    WE_Protection_A_Mask = 0xFFFFFFFF ^ (uint32)WE_Protection_AB_Mask;
    WE_Protection_B_Mask = 0x00000FFF ^ WE_Protection_AB_Mask >> 32;

    // Erase the EEPROM sectors before programming
    EEPROM_Erase();

    return Warning_Flags;

}
//######################### EEPROM_Config ############################

//######################### Configure_Protection_Masks ############################
// This function calculates the Write/Erase Protection masks needed by the Flash API.
// The masks are used to set CMDWEPROTA and CMDWEPROTB registers
// CMDWEPROTA is applicable for sectors 0-31
// Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
// a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
// sectors 40-47, etc
// Detailed documentation can be found in the F28P65x Flash API Reference Guide

uint64 Configure_Protection_Masks(uint16* Sector_Numbers, uint16 Num_EEPROM_Sectors)
{
    // Initialize a variable to store the bits indicating which sectors need to have write/erase
    // protection disabled. The lower 32 bits will represent CMDWEPROTA and the upper 32 bits
    // will represent CMDWEPROTB.
    uint64 Protection_Mask_Sectors = 0;

    // If we have more than one Flash Sector
    if (Num_EEPROM_Sectors > 1)
    {

        uint64 Unshifted_Sectors;
        uint16 Shift_Amount;

        // If all sectors use Mask A
        if (Sector_Numbers[0] < 32 && Sector_Numbers[1] < 32)
        {

            // Configure Mask A
            Unshifted_Sectors = (uint64) 1 << Num_EEPROM_Sectors;
            Unshifted_Sectors -= 1;
            Protection_Mask_Sectors |= (Unshifted_Sectors << Sector_Numbers[0]);

        }// If all sectors use Mask B
        else if (Sector_Numbers[0] > 31 && Sector_Numbers[1] > 31)
        {

            // Configure Mask B
            Shift_Amount = ((Sector_Numbers[1] - 32)/8) - ((Sector_Numbers[0] - 32)/8) + 1;
            Unshifted_Sectors = (uint64) 1 << Shift_Amount;
            Unshifted_Sectors -= 1;
            Protection_Mask_Sectors |= (Unshifted_Sectors << ((Sector_Numbers[0] - 32)/8));
            Protection_Mask_Sectors = Protection_Mask_Sectors << 32;

        } else // If both Masks A and B need to be configured
        {

            // Configure Mask B
            Shift_Amount = ((Sector_Numbers[1] - 32)/8) + 1;
            Unshifted_Sectors = (uint64) 1 << Shift_Amount;
            Unshifted_Sectors -= 1;
            Protection_Mask_Sectors |= Unshifted_Sectors;
            Protection_Mask_Sectors = Protection_Mask_Sectors << 32;

            // Configure Mask A
            Unshifted_Sectors = (uint64) 1 << ((32 - Sector_Numbers[0]) + 1);
            Unshifted_Sectors -= 1;
            Protection_Mask_Sectors |= (Unshifted_Sectors << Sector_Numbers[0]);

        }


    } else { // If only using 1 Flash Sector

        if(Sector_Numbers[0] < 32)
        {
            Protection_Mask_Sectors |= ((uint64) 1 << Sector_Numbers[0]);
        } else
        {
            Protection_Mask_Sectors |= ((uint64) 1 << ((Sector_Numbers[0] - 32)/8));
            Protection_Mask_Sectors = Protection_Mask_Sectors << 32;
        }

    }

    return Protection_Mask_Sectors;

}
//######################### Configure_Protection_Masks ############################

//######################### ClearFSMStatus ############################
//  This function clears the status (STATCMD, similar to FMSTAT of the previous
//  devices) of the previous flash operation.
//  Note: this function is applicable for only F280013X, F280015X and F28P65X devices

void ClearFSMStatus(void)
{
    Fapi_FlashStatusType  oFlashStatus;
    Fapi_StatusType  oReturnCheck;

    // Wait until FSM is done with the previous flash operation
    while (Fapi_checkFsmForReady() != Fapi_Status_FsmReady){}

    oFlashStatus = Fapi_getFsmStatus();

    if(oFlashStatus != 0)
    {

        /* Clear the Status register */
        oReturnCheck = Fapi_issueAsyncCommand(Fapi_ClearStatus);

        // Wait until status is cleared
        while (Fapi_getFsmStatus() != 0) {}

        if(oReturnCheck != Fapi_Status_Success)
        {
            // Check Flash API documentation for possible errors
            Sample_Error();
        }
    }
}
//######################### ClearFSMStatus ############################

//######################### EEPROM_GET_VALID_BANK ############################
void EEPROM_GetValidBank(uint16 Read_Flag)
{
    //Each page holds N Data Words
    //Page size = 8 Page_Status Words + N Data Words = (8 + N) Words
    //Bank Size = 8 Bank_Status words + NUM_EEPROM_PAGES * Page size = 8 + NUM_EEPROM_PAGES*(8 + N) Words

    uint16 i;

    RESET_BANK_POINTER;     // Reset Bank Pointer to enable search for current Bank
    RESET_PAGE_POINTER;     // Reset Page Pointer to enable search for current Page

    // Find Current Bank
    for(i=0; i < NUM_EEPROM_BANKS; i++)
    {
        Bank_Status[0] = *(Bank_Pointer);       // Read contents of Bank Pointer
        Bank_Status[4] = *(Bank_Pointer + 4);

        if(Bank_Status[0] == EMPTY_BANK)        // Check for Unused Bank
        {
            Bank_Counter = i;                   // Set Bank Counter to number of current page
            return;                             // If Bank is Unused, return as EEPROM is empty
        }

        if(Bank_Status[0] == CURRENT_BANK && Bank_Status[4] != CURRENT_BANK)      // Check for Current Bank
        {
            Bank_Counter = i;                   // Set Bank Counter to number of current bank
            Page_Pointer = Bank_Pointer + 8;    // Set Page Pointer to first page in current bank
            break;                              // Break from loop as current bank has been found
        }

        if(Bank_Status[0] == CURRENT_BANK && Bank_Status[4] == CURRENT_BANK)         // Check for Used Bank
            Bank_Pointer += Bank_Size;          // If Bank has been used, set pointer to next bank

    }

    // Find Current Page
    for(i=0; i < NUM_EEPROM_PAGES; i++)
    {
        Page_Status[0] = *(Page_Pointer);       // Read contents of Page Pointer
        Page_Status[4] = *(Page_Pointer + 4);

        // Check for Blank Page or Current Page
        if(Page_Status[0] == BLANK_PAGE)
        {
            Page_Counter = i;                   // Set Page Counter to number of current page
            break;                              // Break from loop as current page has been found
        }

        if (Page_Status[0] == CURRENT_PAGE && Page_Status[4] != CURRENT_PAGE)
        {
            Page_Counter = i + 1;
            break;
        }

        // Check for Used Page
        if(Page_Status[0] == CURRENT_PAGE && Page_Status[4] == CURRENT_PAGE)
        {
            Page_Pointer += EEPROM_PAGE_DATA_SIZE + 8;                 // If page has been used, set pointer to next page
        }

    }

    // Check for full EEPROM
    if (!Read_Flag)
    {
        if (Bank_Counter == NUM_EEPROM_BANKS - 1 && Page_Counter == NUM_EEPROM_PAGES)
        {
            Erase_Inactive_Unit = 1;                // Set flag to erase inactive (full) Flash Bank
            EEPROM_UpdatePageStatus();              // Update Page Status of previous page
            EEPROM_UpdateBankStatus();              // Update Bank Status of previous page
            Erase_Blank_Check = 1;                  // Set flag to perform blank check on inactive (full) Flash Bank
            EEPROM_Erase();                         // Erase flash sector being used as EEPROM
            RESET_BANK_POINTER;                     // Reset Bank Pointer as EEPROM is empty
            RESET_PAGE_POINTER;                     // Reset Page Pointer as EEPROM is empty
        }
    }

}
//######################### EEPROM_GET_VALID_BANK ############################


//############################# EEPROM_ERASE #################################
void EEPROM_Erase()
{

    Fapi_StatusType  oReturnCheck;

    // Clears status of previous Flash operation
    ClearFSMStatus();

    // Enable program/erase protection for select sectors
    // CMDWEPROTA is applicable for sectors 0-31
    // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
    // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
    // sectors 40-47, etc
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

    // Erase the EEPROM Bank
    oReturnCheck = Fapi_issueBankEraseCommand((uint32*) FLASH_BANK_SELECT);

    // Wait for completion and check for any programming errors
    EEPROM_CheckStatus(&oReturnCheck);

}
//############################# EEPROM_ERASE #################################

//############################# ERASE_BANK #################################
void Erase_Bank()
{
    Fapi_StatusType  oReturnCheck;

    // Clears status of previous Flash operation
    ClearFSMStatus();

    // Enable program/erase protection for select sectors
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

    // Erase the inactive EEPROM Bank
    oReturnCheck = Fapi_issueBankEraseCommand((uint32*) FLASH_BANK_SELECT);
    // Wait for completion and check for any programming errors
    EEPROM_CheckStatus(&oReturnCheck);
}

//############################# EEPROM_READ ##################################
void EEPROM_Read(uint16* Read_Buffer)
{

#ifdef PAGE_MODE
    uint16 i;

    // Check for empty EEPROM
    if (Empty_EEPROM)
    {
        Sample_Error(); // Attempting to read data that hasn't been written
    } else
    {
        // Find Current Bank and Current Page
        EEPROM_GetValidBank(1);

        // Increment page pointer to point at first data word
        Page_Pointer += 8;

        // Transfer contents of Current Page to Read Buffer
        for(i=0;i<DATA_SIZE;i++)
        {
            Read_Buffer[i] = *(Page_Pointer++);
        }
    }
#else
    uint16 i;

    // Check for empty EEPROM
    if (Empty_EEPROM)
    {
        Sample_Error(); // Attempting to read data that hasn't been written
    } else
    {
        // Move the bank pointer backwards to read data
        Bank_Pointer -= 4;

        // Transfer contents of Current Page to Read Buffer
        for(i=0;i<4;i++)
        {
            Read_Buffer[i] = *(Bank_Pointer++);
        }
    }

#endif

}
//############################# EEPROM_READ ##################################


//############################ EEPROM_WRITE ##################################
void EEPROM_Write(uint16* Write_Buffer)
{
    EEPROM_GetValidBank(0);                 // Find Current Bank and Current Page
    EEPROM_UpdatePageStatus();              // Update Page Status of previous page
    EEPROM_UpdateBankStatus();              // Update Bank Status of current and previous bank
    EEPROM_UpdatePageData(Write_Buffer);    // Update Page Data of current page
}
//############################ EEPROM_WRITE ##################################


//###################### EEPROM_UPDATE_BANK_STATUS ###########################
void EEPROM_UpdateBankStatus()
{
    // Variables needed for Flash API Functions
    Fapi_StatusType  oReturnCheck;

    Bank_Status[0] = *(Bank_Pointer);       // Read Bank Status from Bank Pointer
    Page_Status[0] = *(Page_Pointer);       // Read Page Status from Page Pointer

    // Program Bank Status for Empty EEPROM
    if (Bank_Status[0] == EMPTY_BANK)
    {

        // Set Bank Status to Current Bank
        Bank_Status[0] = CURRENT_BANK;
        Bank_Status[1] = CURRENT_BANK;
        Bank_Status[2] = CURRENT_BANK;
        Bank_Status[3] = CURRENT_BANK;

        // Clears status of previous Flash operation
        ClearFSMStatus();

        // Enable program/erase protection for select sectors
        // CMDWEPROTA is applicable for sectors 0-31
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        // Program Bank Status to current bank
        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Bank_Pointer,
                                                    Bank_Status, 4, 0, 0,
                                                    Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);

        // Set Page Pointer to first page of current bank
        Page_Counter = 0;
        Page_Pointer = Bank_Pointer + 8;
    }

    // Program Bank Status of full bank and following bank
    if (Bank_Status[0] == CURRENT_BANK && Page_Counter == NUM_EEPROM_PAGES)
    {
        // Set Bank Status to Used Bank
        Bank_Status[0] = CURRENT_BANK;
        Bank_Status[1] = CURRENT_BANK;
        Bank_Status[2] = CURRENT_BANK;
        Bank_Status[3] = CURRENT_BANK;

        // Clears status of previous Flash operation
        ClearFSMStatus();

        // Enable program/erase protection for select sectors
        // CMDWEPROTA is applicable for sectors 0-31
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        // Program Bank Status to full bank
        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Bank_Pointer + 2,
                                                            Bank_Status, 4, 0, 0,
                                                            Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);

        // Increment Bank Pointer to next bank
        Bank_Pointer += Bank_Size;

        if (Bank_Counter == NUM_EEPROM_BANKS - 1 && Page_Counter == NUM_EEPROM_PAGES){
            return;
        }
        else{
        // Set Bank Status to Current Bank
        Bank_Status[0] = CURRENT_BANK;
        Bank_Status[1] = CURRENT_BANK;
        Bank_Status[2] = CURRENT_BANK;
        Bank_Status[3] = CURRENT_BANK;

        // Clears status of previous Flash operation
        ClearFSMStatus();

        // Enable program/erase protection for select sectors
        // CMDWEPROTA is applicable for sectors 0-31
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        // Program Bank Status to current bank
        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Bank_Pointer,
                                                            Bank_Status, 4, 0, 0,
                                                            Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);

        // Set Page Pointer to first page of current bank
        Page_Counter = 0;
        Page_Pointer = Bank_Pointer + 8;
        }
    }
}
//###################### EEPROM_UPDATE_BANK_STATUS ###########################


//###################### EEPROM_UPDATE_PAGE_STATUS ###########################
void EEPROM_UpdatePageStatus()
{

    Fapi_StatusType  oReturnCheck;

    Bank_Status[0] = *(Bank_Pointer);       // Read Bank Status from Bank Pointer
    Page_Status[0] = *(Page_Pointer);       // Read Page Status from Page Pointer

    // Check if Page Status is blank. If so return to EEPROM_WRITE.
    if(Page_Status[0] == BLANK_PAGE)
        return;

    // Program previous page's status to Used Page
    else
    {

        // Set Page Status to Used Page
        Page_Status[0] = CURRENT_PAGE;
        Page_Status[1] = CURRENT_PAGE;
        Page_Status[2] = CURRENT_PAGE;
        Page_Status[3] = CURRENT_PAGE;

        // Clears status of previous Flash operation
        ClearFSMStatus();

        // Enable program/erase protection for select sectors
        // CMDWEPROTA is applicable for sectors 0-31
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        // Program Bank Status to current bank
        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Page_Pointer + 2,
                                                            Page_Status, 4, 0, 0,
                                                            Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);

        // Increment Page Pointer to next page
        Page_Pointer += EEPROM_PAGE_DATA_SIZE + 8;
    }
}
//###################### EEPROM_UPDATE_PAGE_STATUS ###########################

//###################### EEPROM_UPDATE_PAGE_DATA ###########################
void EEPROM_UpdatePageData(uint16* Write_Buffer)
{
    // Variable for write incrementing
    uint16 i;

    // Variables needed for Flash API Functions
    Fapi_StatusType  oReturnCheck;

    for (i = 0; i < EEPROM_PAGE_DATA_SIZE / 4; i++)
    {

        // Clears status of previous Flash operation
        ClearFSMStatus();

        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        // Variable for page offset (first write position has offset of 2 (64 bits),
        // second has offset of 4 (128 bits), etc.)
        uint32 Page_Offset = 4 + (2 * i);

        // Program data located in Write_Buffer to current page
        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Page_Pointer + Page_Offset,
                                                    Write_Buffer + (i*4), 4, 0, 0,
                                                    Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);
    }


    if(oReturnCheck == Fapi_Status_Success)
    {
        // Set Page Status to Current Page
        Page_Status[0] = CURRENT_PAGE;
        Page_Status[1] = CURRENT_PAGE;
        Page_Status[2] = CURRENT_PAGE;
        Page_Status[3] = CURRENT_PAGE;

        // Clears status of previous Flash operation
        ClearFSMStatus();

        // Enable program/erase protection for select sectors
        // CMDWEPROTA is applicable for sectors 0-31
        // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
        // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
        // sectors 40-47, etc
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
        Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

        oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Page_Pointer,
                                                    Page_Status, 4, 0, 0,
                                                    Fapi_AutoEccGeneration);

        // Wait for completion and check for any programming errors
        EEPROM_CheckStatus(&oReturnCheck);

        Empty_EEPROM = 0;

    }
    if (Erase_Inactive_Unit)
    {
        // Erase the inactive (full) EEPROM Bank
        Erase_Inactive_Unit = 0;
    }
}
//###################### EEPROM_UPDATE_PAGE_DATA ###########################

//###################### EEPROM_CHECKSTATUS ###########################
void EEPROM_CheckStatus(Fapi_StatusType* oReturnCheck)
{
    Fapi_FlashStatusType  oFlashStatus;
    Fapi_FlashStatusWordType oFlashStatusWord;

    uint32_t sectorAddress = FLASH_BANK_SELECT + FIRST_AND_LAST_SECTOR[0] * FLASH_SECTOR_SIZE;
    uint16_t sectorSize = (FIRST_AND_LAST_SECTOR[1] - FIRST_AND_LAST_SECTOR[0] + 1) * (FLASH_SECTOR_SIZE / 2);

    // Wait until the Flash program operation is over
    while(Fapi_checkFsmForReady() == Fapi_Status_FsmBusy);

    if(*oReturnCheck != Fapi_Status_Success)
    {
        // Check Flash API documentation for possible errors
        Sample_Error();
    }

    //
    // Read FMSTAT register contents to know the status of FSM after
    // program command to see if there are any program operation related
    // errors
    //
    oFlashStatus = Fapi_getFsmStatus();

    if (Erase_Inactive_Unit && Erase_Blank_Check){
        *oReturnCheck = Fapi_doBlankCheck((uint32_t *) sectorAddress,
                                          sectorSize, &oFlashStatusWord);
        Erase_Blank_Check = 0;
    }

    if(*oReturnCheck != Fapi_Status_Success || oFlashStatus != 3)
    {
        //Check FMSTAT and debug accordingly
        Sample_Error();
    }
}
//###################### EEPROM_CHECKSTATUS ###########################


//###################### EEPROM_GET_FOUR_WORD_POINTER ###########################
// NOTE: This function expects that 0xFFFF is invalid data
void EEPROM_Get_64_Bit_Data_Address()
{
    uint16 *End_Address;

    End_Address = (uint16 *)END_OF_SECTOR;  // Set End_Address for sector

    if(Bank_Pointer > End_Address-3)         // Test if EEPROM is full
    {
        Erase_Inactive_Unit = 1;            // Set flag to erase inactive (full) Flash Bank
        Erase_Blank_Check = 1;              // Set flag to perform blank check on inactive (full) Flash Bank
        EEPROM_Erase();                     // Erase flash Bank being used as EEPROM
        Erase_Inactive_Unit = 0;            // Reset flag for erasing inactive (full) Flash Bank)
        RESET_BANK_POINTER;                 // Reset Bank Pointer as EEPROM is empty

    }

}
//###################### EEPROM_GET_SINGLE_POINTER ###########################


//##################### EEPROM_PROGRAM_FOUR_WORDS ###########################
// NOTE: If Num_Words < 4, the missing missing words will be filled with 0xFFFF
// Example: EEPROM_Program_64_Bits(2, 1, 2, 3, 4)
// This results in [0x0001 0x0002 0xFFFF 0xFFFF] being programmed to Flash
void EEPROM_Program_64_Bits(uint16 Num_Words, uint16 * Write_Buffer)
{

    // Variables needed for Flash API Functions
    Fapi_StatusType oReturnCheck;

    // Test for full sector
    EEPROM_Get_64_Bit_Data_Address();

    // Overwrite anything in first 4 Write_Buffer addresses with 0xFFFF if less than 4 words are to be written
    int i;
    for(i = Num_Words; i < 4; i++)
    {
        Write_Buffer[i] = 0xFFFF;
    }

    // Clears status of previous Flash operation
    ClearFSMStatus();

    // Enable program/erase protection for select sectors
    // CMDWEPROTA is applicable for sectors 0-31
    // Bits 0-11 of CMDWEPROTB is applicable for sectors 32-127, each bit represents
    // a group of 8 sectors, e.g bit 0 represents sectors 32-39, bit 1 represents
    // sectors 40-47, etc
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTA, WE_Protection_A_Mask);
    Fapi_setupBankSectorEnable(FLASH_WRAPPER_PROGRAM_BASE+FLASH_O_CMDWEPROTB, WE_Protection_B_Mask);

    oReturnCheck = Fapi_issueProgrammingCommand((uint32*) Bank_Pointer,
                                                Write_Buffer, 4, 0, 0,
                                                Fapi_AutoEccGeneration);

    // Wait for completion and check for any programming errors
    EEPROM_CheckStatus(&oReturnCheck);

    Empty_EEPROM = 0;

    // Increment to next location
    Bank_Pointer += 4;


}
//##################### EEPROM_PROGRAM_FOUR_WORDS ###########################

//##################### SAMPLE_ERROR ###########################
// This is a sample error function, no error handling is implemented in this project,
void Sample_Error() {

     asm(" ESTOP0");

}

//##################### EXAMPLE_DONE ###########################
// This is a sample function that signifies the end of program execution
void Example_Done() {

    asm(" ESTOP0");

}
