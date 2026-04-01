

//#############################################################################
//
// FILE:   EEPROM_Config.h
//
// The purpose of this file is to configure various aspects of EEPROM Emulation,
// declare functions/global variables, and define unchanging values.
// The following variables can be configured in this file:
// 1. The number of sectors to use for EEPROM Emulation
// 2. EEPROM Bank Size
// 3. EEPROM Page Size
// 4. Flash Sector Size
// 5. Page_Mode or _64_BIT_MODE
//
// NOTE: To select which Flash sectors to use in EEPROM Emulation, adjust SECTOR_NUMBERS
// in EEPROM_FLASH.c
//#############################################################################

// Include Flash API Header Files
#include "driverlib.h"
#include "device.h"

//
// Include Flash API include file
//

#include "FlashTech_F28P65x_C28x.h"

// Include Flash API example header file
//
#include "flash_programming_f28p65x.h"

// Un-comment appropriate definition if one of the following variants is being used

#define F28P65xDKx 1
//#define F28P65xSKx 1
//#define F28P65xSHx 1

// Project specific defines
//#define _64_BIT_MODE 1
#define PAGE_MODE 1

// Define which Flash Bank to use for EEPROM Emulation.
// Bank 0 reserved for loading EEPROM Emulation/Flash API by default, choose Banks 1-4
// Note: Check data-sheet to verify that device has indicated bank
#define FLASH_BANK_SELECT FlashBank1StartAddress

// Define Flash Sector Size in 16-bit words
#define FLASH_SECTOR_SIZE F28P65x_FLASH_SECTOR_SIZE

// Define how many sectors are in a Flash Bank
#define NUM_FLASH_SECTORS F28P65x_NUM_FLASH_SECTORS

// Define how many EEPROM Banks to emulate
#define NUM_EEPROM_BANKS 4

// Define how many EEPROM Pages for each EEPROM Bank
#define NUM_EEPROM_PAGES 3

// Define size of data to be written to EEPROM Pages (measured in 16-bit words)
// If size is not a multiple of 4, the size of the Page data will be rounded up to the closest
// multiple of 4 to be in accordance with ECC requirements
// Example: DATA_SIZE = 3
//          This will result in EEPROM Pages using 4 16-bit words
//          for data, but will only write 3 words of valid data per page
#define DATA_SIZE 64


// Calculate the appropriate size of data in EEPROM Pages.
// To comply with ECC, must be a multiple of 4 greater than or equal to the
// DATA_SIZE specified by user
#define EEPROM_PAGE_DATA_SIZE (DATA_SIZE + 3 - ((DATA_SIZE + 3) % 4))


// Pointer Initialization
#define RESET_BANK_POINTER Bank_Pointer = (uint16 *) (FLASH_BANK_SELECT + ((uint32)FIRST_AND_LAST_SECTOR[0] * FLASH_SECTOR_SIZE))
#define RESET_PAGE_POINTER Page_Pointer = (uint16 *) (FLASH_BANK_SELECT + ((uint32)FIRST_AND_LAST_SECTOR[0] * FLASH_SECTOR_SIZE)) + 8

// Define end address of last sector to be used in EEPROM Emulation
#define END_OF_SECTOR (FLASH_BANK_SELECT + ((uint32)FIRST_AND_LAST_SECTOR[1] * FLASH_SECTOR_SIZE) + (FLASH_SECTOR_SIZE - 1));

// Bank/Page Status Definitions
#define EMPTY_BANK   0xFFFF
#define CURRENT_BANK 0x5A5A
#define BLANK_PAGE   0xFFFF
#define CURRENT_PAGE 0xA5A5

#define F28P65x_FLASH_SECTOR_SIZE 0x400

#define F28P65x_NUM_FLASH_SECTORS 128


// Function Prototypes
extern int EEPROM_Config_Check();
extern void EEPROM_GetValidBank(uint16 Read_Flag);
extern void EEPROM_Erase();
extern void EEPROM_Read(uint16* Read_Buffer);
extern void EEPROM_Write(uint16* Write_Buffer);
extern void EEPROM_UpdateBankStatus();
extern void EEPROM_UpdatePageStatus();
extern void EEPROM_UpdatePageData();
extern void EEPROM_CheckStatus(Fapi_StatusType* oReturnCheck);
extern void EEPROM_Get_64_Bit_Data_Address();
extern void EEPROM_Program_64_Bits(uint16 Num_Words, uint16* Write_Buffer);
extern uint64 Configure_Protection_Masks(uint16* Sector_Numbers, uint16 Num_EEPROM_Sectors);
extern void Sample_Error();
extern void Example_Done();
extern void ClearFSMStatus();

// Global Variables
extern uint16 *Bank_Pointer;
extern uint16 *Page_Pointer;
extern uint16 *Sector_End;
extern uint32 WE_Protection_A_Mask;
extern uint32 WE_Protection_B_Mask;
extern uint32 Bank_Size;
extern uint16 Bank_Counter;
extern uint16 Page_Counter;
extern uint16 Bank_Status[8];
extern uint16 Page_Status[8];
extern uint16 NUM_EEPROM_SECTORS;
extern uint16 FIRST_AND_LAST_SECTOR[2];

#ifndef FLASH_PROGRAMMING_F28P65X_H_
#define FLASH_PROGRAMMING_F28P65X_H_

// Bank0 Sector start addresses

#define     Bzero_Sector0_start         0x00080000U
#define     Bzero_Sector1_start         0x00080400U
#define     Bzero_Sector2_start         0x00080800U
#define     Bzero_Sector3_start         0x00080C00U
#define     Bzero_Sector4_start         0x00081000U
#define     Bzero_Sector5_start         0x00081400U
#define     Bzero_Sector6_start         0x00081800U
#define     Bzero_Sector7_start         0x00081C00U
#define     Bzero_Sector8_start         0x00082000U
#define     Bzero_Sector9_start         0x00082400U
#define     Bzero_Sector10_start        0x00082800U
#define     Bzero_Sector11_start        0x00082C00U
#define     Bzero_Sector12_start        0x00083000U
#define     Bzero_Sector13_start        0x00083400U
#define     Bzero_Sector14_start        0x00083800U
#define     Bzero_Sector15_start        0x00083C00U
#define     Bzero_Sector16_start        0x00084000U
#define     Bzero_Sector17_start        0x00084400U
#define     Bzero_Sector18_start        0x00084800U
#define     Bzero_Sector19_start        0x00084C00U
#define     Bzero_Sector20_start        0x00085000U
#define     Bzero_Sector21_start        0x00085400U
#define     Bzero_Sector22_start        0x00085800U
#define     Bzero_Sector23_start        0x00085C00U
#define     Bzero_Sector24_start        0x00086000U
#define     Bzero_Sector25_start        0x00086400U
#define     Bzero_Sector26_start        0x00086800U
#define     Bzero_Sector27_start        0x00086C00U
#define     Bzero_Sector28_start        0x00087000U
#define     Bzero_Sector29_start        0x00087400U
#define     Bzero_Sector30_start        0x00087800U
#define     Bzero_Sector31_start        0x00087C00U
#define     Bzero_Sector32_start        0x00088000U
#define     Bzero_Sector33_start        0x00088400U
#define     Bzero_Sector34_start        0x00088800U
#define     Bzero_Sector35_start        0x00088C00U
#define     Bzero_Sector36_start        0x00089000U
#define     Bzero_Sector37_start        0x00089400U
#define     Bzero_Sector38_start        0x00089800U
#define     Bzero_Sector39_start        0x00089C00U
#define     Bzero_Sector40_start        0x0008A000U
#define     Bzero_Sector41_start        0x0008A400U
#define     Bzero_Sector42_start        0x0008A800U
#define     Bzero_Sector43_start        0x0008AC00U
#define     Bzero_Sector44_start        0x0008B000U
#define     Bzero_Sector45_start        0x0008B400U
#define     Bzero_Sector46_start        0x0008B800U
#define     Bzero_Sector47_start        0x0008BC00U
#define     Bzero_Sector48_start        0x0008C000U
#define     Bzero_Sector49_start        0x0008C400U
#define     Bzero_Sector50_start        0x0008C800U
#define     Bzero_Sector51_start        0x0008CC00U
#define     Bzero_Sector52_start        0x0008D000U
#define     Bzero_Sector53_start        0x0008D400U
#define     Bzero_Sector54_start        0x0008D800U
#define     Bzero_Sector55_start        0x0008DC00U
#define     Bzero_Sector56_start        0x0008E000U
#define     Bzero_Sector57_start        0x0008E400U
#define     Bzero_Sector58_start        0x0008E800U
#define     Bzero_Sector59_start        0x0008EC00U
#define     Bzero_Sector60_start        0x0008F000U
#define     Bzero_Sector61_start        0x0008F400U
#define     Bzero_Sector62_start        0x0008F800U
#define     Bzero_Sector63_start        0x0008FC00U
#define     Bzero_Sector64_start        0x00090000U
#define     Bzero_Sector65_start        0x00090400U
#define     Bzero_Sector66_start        0x00090800U
#define     Bzero_Sector67_start        0x00090C00U
#define     Bzero_Sector68_start        0x00091000U
#define     Bzero_Sector69_start        0x00091400U
#define     Bzero_Sector70_start        0x00091800U
#define     Bzero_Sector71_start        0x00091C00U
#define     Bzero_Sector72_start        0x00092000U
#define     Bzero_Sector73_start        0x00092400U
#define     Bzero_Sector74_start        0x00092800U
#define     Bzero_Sector75_start        0x00092C00U
#define     Bzero_Sector76_start        0x00093000U
#define     Bzero_Sector77_start        0x00093400U
#define     Bzero_Sector78_start        0x00093800U
#define     Bzero_Sector79_start        0x00093C00U
#define     Bzero_Sector80_start        0x00094000U
#define     Bzero_Sector81_start        0x00094400U
#define     Bzero_Sector82_start        0x00094800U
#define     Bzero_Sector83_start        0x00094C00U
#define     Bzero_Sector84_start        0x00095000U
#define     Bzero_Sector85_start        0x00095400U
#define     Bzero_Sector86_start        0x00095800U
#define     Bzero_Sector87_start        0x00095C00U
#define     Bzero_Sector88_start        0x00096000U
#define     Bzero_Sector89_start        0x00096400U
#define     Bzero_Sector90_start        0x00096800U
#define     Bzero_Sector91_start        0x00096C00U
#define     Bzero_Sector92_start        0x00097000U
#define     Bzero_Sector93_start        0x00097400U
#define     Bzero_Sector94_start        0x00097800U
#define     Bzero_Sector95_start        0x00097C00U
#define     Bzero_Sector96_start        0x00098000U
#define     Bzero_Sector97_start        0x00098400U
#define     Bzero_Sector98_start        0x00098800U
#define     Bzero_Sector99_start        0x00098C00U
#define     Bzero_Sector100_start       0x00099000U
#define     Bzero_Sector101_start       0x00099400U
#define     Bzero_Sector102_start       0x00099800U
#define     Bzero_Sector103_start       0x00099C00U
#define     Bzero_Sector104_start       0x0009A000U
#define     Bzero_Sector105_start       0x0009A400U
#define     Bzero_Sector106_start       0x0009A800U
#define     Bzero_Sector107_start       0x0009AC00U
#define     Bzero_Sector108_start       0x0009B000U
#define     Bzero_Sector109_start       0x0009B400U
#define     Bzero_Sector110_start       0x0009B800U
#define     Bzero_Sector111_start       0x0009BC00U
#define     Bzero_Sector112_start       0x0009C000U
#define     Bzero_Sector113_start       0x0009C400U
#define     Bzero_Sector114_start       0x0009C800U
#define     Bzero_Sector115_start       0x0009CC00U
#define     Bzero_Sector116_start       0x0009D000U
#define     Bzero_Sector117_start       0x0009D400U
#define     Bzero_Sector118_start       0x0009D800U
#define     Bzero_Sector119_start       0x0009DC00U
#define     Bzero_Sector120_start       0x0009E000U
#define     Bzero_Sector121_start       0x0009E400U
#define     Bzero_Sector122_start       0x0009E800U
#define     Bzero_Sector123_start       0x0009EC00U
#define     Bzero_Sector124_start       0x0009F000U
#define     Bzero_Sector125_start       0x0009F400U
#define     Bzero_Sector126_start       0x0009F800U
#define     Bzero_Sector127_start       0x0009FC00U
#define     BankThree_Sector48_start    0x000EC000U

#define     FlashBank0StartAddress      0x80000U
#define     FlashBank0EndAddress        0x9FFFFU
#define     FlashBank1StartAddress      0xA0000U
#define     FlashBank1EndAddress        0xBFFFFU
#define     FlashBank2StartAddress      0xC0000U
#define     FlashBank2EndAddress        0xDFFFFU
#define     FlashBank3StartAddress      0xE0000U
#define     FlashBank3EndAddress        0xFFFFFU
#define     FlashBank4StartAddress      0x10FFFFU
#define     FlashBank4EndAddress        0x11FFFFU



//Sector length in number of 16bits
#define Sector2KB_u16length   0x400U
#define Bank256KB_u16length   0x20000U

//Sector length in number of 32bits
#define Sector2KB_u32length   0x200U
#define Bank256KB_u32length   0x10000U

//Flash wrapper base address
#define  FLASH_WRAPPER_PROGRAM_BASE         0x57000U /*(FLASH0CMD_BASE - 0x1000UL)*/

/*!
    \brief Define to map the direct access to the FMC registers.
*/
#define FlashTech_CPU0_BASE_ADDRESS ((Fapi_FmcRegistersType *)FLASH_WRAPPER_PROGRAM_BASE)

#endif /* FLASH_PROGRAMMING_F28P65X_H_ */
