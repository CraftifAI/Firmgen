//###########################################################################
//
// FILE:    flash_kernel_c28x_dual_ex1_sci_get_function_cpu1.c
//
// TITLE:   Kernel commands and corresponding function calls
//! <h1> Kernel commands and corresponding function calls </h1>
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
#include <flash_kernel_c28x_dual_ex1_erase_cpu1.h>
#include <flash_kernel_c28x_dual_ex1_verify_cpu1.h>
#include "sysctl.h"
#include "device.h"
#include "stdint.h"
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
StatusCode statusCode;

uint16_t checksum;

//
// getWordData is a pointer to the function that interfaces to the peripheral.
// Each loader assigns this pointer to it's particular getWordData function.
//
extern uint16fptr getWordData;

//
// Function Prototypes
//
uint16_t sciaGetWordData(void);
uint16_t sciGetWordDataCPU2(void);
uint16_t sciaGetOnlyWordData(void);
void sendACK(void);
void sendNAK(void);
inline uint16_t sciaGetACK(void);
inline void sciaFlush(void);
void sciaInit(uint32_t BootMode);
uint32_t sciGetFunction(uint32_t BootMode);
uint16_t sciGetPacket(uint16_t* length, uint16_t* data);
uint16_t sciSendPacket(uint16_t command, uint16_t status, uint16_t length,
                      uint16_t* data1, uint16_t flashAPIerror, uint16_t* data2);
void sciSendWord(uint16_t word);
void sciSendChecksum(void);
void setBootModeForCPU2(void);
void assignSciaIOCPU2(uint32_t BootMode);
void assignSharedRAMsToCPU2(void);
void assignSharedBANKsToCPU2(void);
void waitForCPU2Signal(void);
void sciPinmuxOption(uint32_t BootMode);
void sciaAutobaudLock(void);
//
// Function Prototypes (External)
//
extern uint32_t sciLoadApplication(uint32_t BootMode);
extern void sharedErase(uint32_t sectors1, uint32_t sectors2, uint32_t bank);
extern void copyData(void);
extern void verifyData(void);
extern uint32_t getLongData(void);
extern void readReservedFn(void);
extern void sciLoadCPU2Kernel(uint32_t BootMode);

//
// Function Prototypes
//
#pragma CODE_SECTION(goToCodeStartCPU2Kernel, "MSGRAM_CPU1_TO_CPU2_COPY_TO_M1_RAM")
#pragma RETAIN(goToCodeStartCPU2Kernel)
void goToCodeStartCPU2Kernel(void);

//
// SCI_GetFunction - This function first initializes SCIA and performs
//                   an autobaud lock. It contains a while loop waiting on
//                   commands from the host.  It processes each
//                   command and sends a response except for Run and
//                   Reset commands.  On Run the kernel exits and branches
//                   to the Entry Point.  On Reset, the kernel exits the
//                   while loop and does a WatchDog Time-out.
//
uint32_t sciGetFunction(uint32_t  BootMode)
{
    uint32_t EntryAddr;
    uint16_t command;
    uint16_t data[10]; // 16*10 = 128 + 32
    uint16_t length;
    uint32_t selected_bank;

    //
    // Assign GetWordData to the SCI-A version of the
    // function. GetWordData is a pointer to a function.
    //
    getWordData = sciaGetWordData;

    //
    // Initialize the SCI-A port for communications
    // with the host.
    //
    sciaInit(BootMode);

    //
    // driverlib autobaud lock:
    //
    sciaAutobaudLock();

    //
    // get user command through console.
    //
    command = sciGetPacket(&length, data);

    while(command != RESET_CPU1)
    {
        //
        // Reset the statusCode.
        //
        statusCode.status = NO_ERROR;
        statusCode.address = 0x12345678;
        statusCode.flashAPIError = NO_ERROR;
        statusCode.flashAPIFsmStatus = 0;
        checksum = 0;

        //
        // CPU1_UNLOCK_Z1
        //
        if(command == CPU1_UNLOCK_Z1)
           {

               //
               // driverlib struct for csmKey.
               //
               DCSM_CSMPasswordKey psCMDKey;

               psCMDKey.csmKey0 = (uint32_t)data[0] | ((uint32_t)data[1] << 16);
               psCMDKey.csmKey1 = (uint32_t)data[2] | ((uint32_t)data[3] << 16);
               psCMDKey.csmKey2 = (uint32_t)data[4] | ((uint32_t)data[5] << 16);
               psCMDKey.csmKey3 = (uint32_t)data[6] | ((uint32_t)data[7] << 16);


               //
               // Unlock the zone 1, driverlib.
               //
               DCSM_unlockZone1CSM(&psCMDKey);

               //
               // check if it is unlocked.
               //
               if(DCSM_getZone1CSMSecurityStatus() == DCSM_STATUS_LOCKED)
               {
                   statusCode.status = UNLOCK_ERROR;
               }
           }
           else if(command == CPU1_UNLOCK_Z2)
           {
               //
               // driver lib implementation of csmKeys
               //
               DCSM_CSMPasswordKey psCMDKey;

               psCMDKey.csmKey0 = (uint32_t) data[0] | ((uint32_t) data[1] << 16);
               psCMDKey.csmKey1 = (uint32_t) data[2] | ((uint32_t) data[3] << 16);
               psCMDKey.csmKey2 = (uint32_t) data[4] | ((uint32_t) data[5] << 16);
               psCMDKey.csmKey3 = (uint32_t) data[6] | ((uint32_t) data[7] << 16);

               //
               // Unlock the zone 2, driverlib.
               //
               DCSM_unlockZone2CSM(&psCMDKey);

               //
               // check if zone 2 is unlocked.
               //
               if (DCSM_getZone2CSMSecurityStatus() == DCSM_STATUS_LOCKED)
               {
                   statusCode.status = UNLOCK_ERROR;
               }
           }
        //
        // DFU_CPU1
        //
        else if(command == DFU_CPU1)
        {
            //
            // loads application into CPU1 FLASH
            //
            EntryAddr = sciLoadApplication(BootMode);
            if(statusCode.status == NO_ERROR)
            {
                statusCode.address = EntryAddr;
            }
        }

        //
        // DFU_CPU2
        //
        else if(command == DFU_CPU2)
        {
            //
            // loads application into CPU2 FLASH
            //
            EntryAddr = sciLoadApplication(BootMode);
            if(statusCode.status == NO_ERROR)
            {
                statusCode.address = EntryAddr;
            }
        }

        //
        // ERASE_CPU1
        //
        else if(command == ERASE_CPU1)
        {
            uint32_t sectors_0to31 = (uint32_t)(((uint32_t)data[1] << 16) |
                    (uint32_t)data[0]);
            uint32_t sectors_32to127 = (uint32_t)data[2];

            sharedErase(sectors_0to31, sectors_32to127, selected_bank);
        }

        //
        // ERASE_CPU2
        //
        else if(command == ERASE_CPU2)
        {
            uint32_t sectors_0to31 = (uint32_t)(((uint32_t)data[1] << 16) |
                    (uint32_t)data[0]);
            uint32_t sectors_32to127 = (uint32_t)data[2];

            sharedErase(sectors_0to31, sectors_32to127, selected_bank);
        }

        //
        // BANK_SELECT
        //
        else if (command == BANK_SELECT)
        {
            //
            // Selects which bank the CPU chooses to erase
            //
            selected_bank = data[0];
        }

        //
        // VERIFY_CPU1
        //
        else if(command == VERIFY_CPU1)
        {
            verifyData();
        }

        //
        // VERIFY_CPU2
        //
        else if(command == VERIFY_CPU2)
        {
            verifyData();
        }

        //
        // RUN_CPU1
        //
        else if(command == RUN_CPU1)
        {
            EntryAddr = (uint32_t)(((uint32_t)data[1] << 16) | ((uint32_t)data[0]));
            return(EntryAddr);
        }

        //
        // COMMAND_ERROR
        //
        else
        {
            statusCode.status = COMMAND_ERROR;
        }
        //
        // send the packet and if NAK send again.
        //
        while(sciSendPacket(command, statusCode.status, 12,
                             (uint16_t*)&statusCode.address, statusCode.flashAPIError,
                             (uint16_t*)&statusCode.flashAPIFsmStatus)){}

        command = sciGetPacket(&length, data); //get next packet

    }
    //
    // RESET_CPU1

    //
    // Reset with WatchDog Timeout
    //
    EALLOW;

    //
    // driverlib, Watchdog reset enable = WDENINT->0 and WDOVERRIDE->0
    //
    SysCtl_setWatchdogMode(SYSCTL_WD_MODE_RESET);

    //
    // enable the Watchdog, driverlib; // same as HWREGH(WD_BASE + SYSCTL_O_WDCR) = SYSCTL_WD_CHKBITS;
    //
    SysCtl_enableWatchdog();
    EDIS;
    while(1){}
}

//
// sciSendPacket -  Sends a Packet to the host which contains
//                  status in the data and address.  It sends the
//                  statusCode global variable contents.  It then waits
//                  for an ACK or NAK from the host.
//
uint16_t sciSendPacket(uint16_t command, uint16_t status, uint16_t length,
                       uint16_t* data1, uint16_t flashAPIError, uint16_t* data2)
{
    int i;

    sciaFlush();
    DEVICE_DELAY_US(100000);
    sciSendWord(0x1BE4);
    sciSendWord(length);

    checksum = 0;
    sciSendWord(command);
    sciSendWord(status);

    for(i = 0; i < 2; i++)
    {
        sciSendWord(*(data1 + i));
    }

    sciSendWord(flashAPIError);

    for(i = 0; i < 2; i++){
        sciSendWord(*(data2 + i));
    }
    sciSendChecksum();
    sciSendWord(0xE41B);

    //
    // Receive an ACK or NAK
    //
    return sciaGetACK();
}

//
// sciaGetACK - Gets 1-byte ACK from the host.
//
inline uint16_t sciaGetACK()
{
    uint16_t wordData;

    //
    // wait for and read a char blocking nonFIFO, driverlib.
    //
    wordData = SCI_readCharBlockingNonFIFO(SCIA_BASE);

    if(wordData != ACK)
    {
        return(1);
    }

    return(0);
}

//
// sciSendChecksum - Sends the Global checksum value
//
void sciSendChecksum()
{
    //
    // wait for SCIA_TX to be free and write LSB of checksum to it.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, (checksum & 0xFF));

    sciaFlush();
    sciaGetACK();

    //
    // wait for SCIA_TX to be free and write MSB of checksum to it.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, ((checksum >> 8) & 0xFF));

    sciaFlush();
    sciaGetACK();
}

//
// sciSendWord - Sends a uint16_t word.
//
void sciSendWord(uint16_t word)
{
    //
    // send LSB of word, driverlib.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, (word & 0xFF));

    checksum += word & 0xFF;

    sciaFlush();
    sciaGetACK();

    //
    // send MSB of word, driverlib.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, ((word>>8) & 0xFF));

    checksum += word>>8 & 0xFF;

    sciaFlush();
    sciaGetACK();
}

//
// sciaInit - Initialize the SCI-A port for communications with the host.
//
void sciaInit(uint32_t  BootMode)
{
    //
    // Enable the SCI-A clocks
    //
    EALLOW;

    SysCtl_enablePeripheral(SYSCTL_PERIPH_CLK_SCIA);

    //
    // TRM, 0x0007 -> scaler of 14. OSCLOK low speed scaling for SCI.
    //
    SysCtl_setLowSpeedClock(SYSCTL_LSPCLK_PRESCALE_14);

    // reset SCI channels.
    SCI_resetChannels(SCIA_BASE);
    HWREGH(SCIA_BASE + SCI_O_FFTX) &= ~SCI_FFTX_TXFIFORESET;

    //
    // 1 stop bit, No parity, 8-bit character
    // No loopback
    //
    // CLK speed and Baud rate get overwritten in autobaud_lock function later.
    //
    SCI_disableLoopback(SCIA_BASE);
    SCI_setConfig(
            SCIA_BASE, DEVICE_LSPCLK_FREQ, DEFAULT_BAUD,
            SCI_CONFIG_WLEN_8 | SCI_CONFIG_STOP_ONE | SCI_CONFIG_PAR_NONE);


    //
    // Enable TX, RX, Use internal SCICLK
    //
    HWREGH(SCIA_BASE + SCI_O_CTL1) = (SCI_CTL1_TXENA | SCI_CTL1_RXENA);

    //
    // Disable RxErr, Sleep, TX Wake,
    // Disable Rx Interrupt, Tx Interrupt
    //
    HWREGB(SCIA_BASE + SCI_O_CTL2) = 0x0U;

    SCI_disableFIFO(SCIA_BASE);
    //
    // Relinquish SCI-A from reset
    //
    SCI_enableModule(SCIA_BASE);

    EDIS;

    // pick SCIA TX and RX GPIO pin according to BootMode.
    sciPinmuxOption(BootMode);

    return;
}

//
// SCIA_AutobaudLock - Perform autobaud lock with the host.
//                     Note that if autobaud never occurs
//                     the program will hang in this routine as there
//                     is no timeout mechanism included.
//

//replace with SCI_lockAutobaud?
void sciaAutobaudLock(void)
{
    uint16_t byteData;

    //
    // Prime the baud register
    //
    HWREGH(SCIA_BASE + SCI_O_HBAUD) = 0x0U;
    HWREGH(SCIA_BASE + SCI_O_LBAUD) = 0x1U;

    //
    // Prepare for autobaud detection.
    // Set the CDC bit to enable autobaud detection and clear the ABD bit.
    //
    HWREGH(SCIA_BASE + SCI_O_FFCT) |= SCI_FFCT_CDC;
    HWREGH(SCIA_BASE + SCI_O_FFCT) |= SCI_FFCT_ABDCLR;

    //
    // Wait until we correctly read an 'A' or 'a' and lock
    //
    while((HWREGH(SCIA_BASE + SCI_O_FFCT) & SCI_FFCT_ABD) != SCI_FFCT_ABD)
    {
    }

    //
    // After autobaud lock, clear the ABD and CDC bits
    //
    HWREGH(SCIA_BASE + SCI_O_FFCT) |= SCI_FFCT_ABDCLR;
    HWREGH(SCIA_BASE + SCI_O_FFCT) &= ~SCI_FFCT_CDC;

   byteData = SCI_readCharBlockingNonFIFO(SCIA_BASE);

   SCI_writeCharBlockingNonFIFO(SCIA_BASE, byteData);

    return;
}
//
// sciaGetWordData -  This routine fetches two bytes from the SCI-A
//                    port and puts them together to form a single
//                    16-bit value.  It is assumed that the host is
//                    sending the data in the order LSB followed by MSB.
//
uint16_t sciaGetWordData(void)
{
   uint16_t wordData;
   uint16_t byteData;

   wordData = 0x0000;
   byteData = 0x0000;

   //
   // Fetch the LSB and verify back to the host
   //
   wordData = SCI_readCharBlockingNonFIFO(SCIA_BASE); // wait until RX is rdy then read.

#if !checksum_enable

   //
   // wait until TX is rdy then write.
   //
   SCI_writeCharBlockingNonFIFO(SCIA_BASE, wordData);

#endif

   //
   // Fetch the MSB and verify back to the host
   //
   byteData = SCI_readCharBlockingNonFIFO(SCIA_BASE); // read

#if !checksum_enable

   //
   // write
   //
   SCI_writeCharBlockingNonFIFO(SCIA_BASE, wordData);

#endif

//
// form checksum.
//
#if checksum_enable
    checksum += wordData + byteData;
#endif

   //
   // form the wordData from the MSB:LSB
   //
   wordData |= (byteData << 8);

   return wordData;
}



//
// sciaGetWordDataCPU2 - This routine fetches two bytes from the SCI-A
//                    port and puts them together to form a single
//                    16-bit value.  It is assumed that the host is
//                    sending the data in the order LSB followed by MSB.
//
uint16_t sciaGetWordDataCPU2(void)
{
   uint16_t wordData;
   uint16_t byteData;

   wordData = 0x0000;
   byteData = 0x0000;

   //
   // Fetch the LSB and verify back to the host
   //
   wordData = SCI_readCharBlockingNonFIFO(SCIA_BASE);
   SCI_writeCharBlockingNonFIFO(SCIA_BASE, wordData);

   //
   // Fetch the MSB and verify back to the host
   //
   byteData = SCI_readCharBlockingNonFIFO(SCIA_BASE);
   SCI_writeCharBlockingNonFIFO(SCIA_BASE, byteData);

   //
   // form the wordData from the MSB:LSB
   //
   wordData |= (byteData << 8);

   return wordData;
}

//
// sciaGetOnlyWordData -  This routine fetches two bytes from the SCI-A
//                        port and puts them together to form a single
//                        16-bit value.  It is assumed that the host is
//                        sending the data in the order LSB followed by MSB.
//
uint16_t sciaGetOnlyWordData(void)
{
   uint16_t wordData;
   uint16_t byteData;

   wordData = 0x0000;
   byteData = 0x0000;

   //
   // Fetch the LSB and verify back to the host
   //
   wordData = SCI_readCharBlockingNonFIFO(SCIA_BASE);

   //
   // Fetch the MSB and verify back to the host
   //
   byteData = SCI_readCharBlockingNonFIFO(SCIA_BASE);

   // compute checksum.
   checksum += wordData + byteData;

   //
   // form the wordData from the MSB:LSB
   //
   wordData |= (byteData << 8);

   return wordData;
}

//
// sciGetPacket -  This routine receives the packet, returns the
//                 command and puts the data length in Uin16* length
//                 and data in uint16_t* data
//
uint16_t sciGetPacket(uint16_t* length, uint16_t* data)
{
    if(sciaGetOnlyWordData() != 0x1BE4)
    {
        sendNAK();

        //
        // start packet error
        //
        return(100);
    }

    *length = sciaGetOnlyWordData();

    //
    // checksum of command and data
    //
    checksum = 0;
    uint16_t command = sciaGetOnlyWordData();

    int i = 0;
    for(i = 0; i < (*length)/2; i++)
    {
        *(data+i) = sciaGetOnlyWordData();
    }

    uint16_t dataChecksum = checksum;
    if(dataChecksum != sciaGetOnlyWordData())
    {
        sendNAK();

        //
        // checksum error
        //
        return(101);
    }
    if(sciaGetOnlyWordData() != 0xE41B)
    {
        sendNAK();

        //
        // end packet error
        //
        return(102);
    }

    sendACK();
    return(command);
}


//
// sendACK - This routine transmits ACK.
//
void sendACK(void)
{
    //
    // write ACKowledged.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, ACK);

    sciaFlush();
}

//
// sendNAK - This routine transmits NAK.
//
void sendNAK(void)
{
    //
    // write NotAcKowledged.
    //
    SCI_writeCharBlockingNonFIFO(SCIA_BASE, NAK);

    sciaFlush();
}

//
// sciaFlush - This routine flushes SCIA.
//
void sciaFlush(void)
{
    //
    // wait while TX is busy.
    //
    while(SCI_isTransmitterBusy(SCIA_BASE))
    {
    }
}


//
// sciPinmuxOption -     This routine configures correct set of GPIO pins
//                       as SCI pins according to BootMode :-
//                       1) Configure GPIO Tx as SCITXDA pin
//                       2) Configure GPIO Rx as SCIRXDA pin
//                       3) Configure GPIO Rx as asynchronous pin
//
void sciPinmuxOption(uint32_t BootMode)
{
     if(BootMode == SCI_BOOT)
        {
        //
        // Configure GPIO12 as SCITXDA (Output pin)
        // Configure GPIO13 as SCIRXDA (Input pin)
        //

        //
        // GPIO12 is the SCI Tx pin.
        //
        GPIO_setControllerCore(12, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_12_SCIA_TX);
        GPIO_setDirectionMode(12, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(12, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(12, GPIO_QUAL_ASYNC);

        //
        // GPIO13 is the SCI Rx pin.
        //
        GPIO_setControllerCore(13, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_13_SCIA_RX);
        GPIO_setDirectionMode(13, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(13, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(13, GPIO_QUAL_ASYNC);
    }

    else if(BootMode == SCI_BOOT_ALT1)
    {
        //
        // Configure GPIO84 as SCITXDA (Output pin)
        // Configure GPIO85 as SCIRXDA (Input pin)
        //
        //
        // GPIO84 is the SCI Tx pin.
        //;
        GPIO_setControllerCore(84, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_84_SCIA_TX);
        GPIO_setDirectionMode(84, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(84, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(84, GPIO_QUAL_ASYNC);
        //
        // GPIO85 is the SCI Rx pin.
        //
        GPIO_setControllerCore(85, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_85_SCIA_RX);
        GPIO_setDirectionMode(85, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(85, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(85, GPIO_QUAL_ASYNC);
    }
    else if(BootMode == SCI_BOOT_ALT2)
    {
        //
        // Configure GPIO36 as SCITXDA (Output pin)
        // Configure GPIO35 as SCIRXDA (Input pin)
        //
        //
        // GPIO36 is the SCI Tx pin.
        //
        GPIO_setControllerCore(36, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_36_SCIA_TX);
        GPIO_setDirectionMode(36, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(36, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(36, GPIO_QUAL_ASYNC);
        //
        // GPIO35 is the SCI Rx pin.
        //
        GPIO_setControllerCore(35, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_35_SCIA_RX);
        GPIO_setDirectionMode(35, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(35, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(35, GPIO_QUAL_ASYNC);
    }
    else if(BootMode == SCI_BOOT_ALT3)
    {
        //
        // Configure GPIO42 as SCITXDA (Output pin)
        // Configure GPIO43 as SCIRXDA (Input pin)
        //
        //
        // GPIO42 is the SCI Tx pin.
        //
        GPIO_setControllerCore(42, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_42_SCIA_TX);
        GPIO_setDirectionMode(42, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(42, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(42, GPIO_QUAL_ASYNC);
        //
        // GPIO43 is the SCI Rx pin.
        //
        GPIO_setControllerCore(43, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_43_SCIA_RX);
        GPIO_setDirectionMode(43, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(43, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(43, GPIO_QUAL_ASYNC);
    }
    else if(BootMode == SCI_BOOT_ALT4)
    {
        //
        // Configure GPIO65 as SCITXDA (Output pin)
        // Configure GPIO64 as SCIRXDA (Input pin)
        //
        //
        // GPIO65 is the SCI Tx pin.
        //
        GPIO_setControllerCore(65, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_65_SCIA_TX);
        GPIO_setDirectionMode(65, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(65, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(65, GPIO_QUAL_ASYNC);
        //
        // GPIO64 is the SCI Rx pin.
        //
        GPIO_setControllerCore(64, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_64_SCIA_RX);
        GPIO_setDirectionMode(64, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(64, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(64, GPIO_QUAL_ASYNC);
    }
    else if(BootMode == SCI_BOOT_ALT5)
    {
        // Configure GPIO29 as SCITXDA (Output pin)
        // Configure GPIO28 as SCIRXDA (Input pin)
        //
        //
        // GPIO29 is the SCI Tx pin.
        //
        GPIO_setControllerCore(DEVICE_GPIO_PIN_SCITXDA, GPIO_CORE_CPU1);
        GPIO_setPinConfig(DEVICE_GPIO_CFG_SCITXDA);
        GPIO_setDirectionMode(DEVICE_GPIO_PIN_SCITXDA, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(DEVICE_GPIO_PIN_SCITXDA, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(DEVICE_GPIO_PIN_SCITXDA, GPIO_QUAL_ASYNC);
        //
        // GPIO28 is the SCI Rx pin.
        //
        GPIO_setControllerCore(DEVICE_GPIO_PIN_SCIRXDA, GPIO_CORE_CPU1);
        GPIO_setPinConfig(DEVICE_GPIO_CFG_SCIRXDA);
        GPIO_setDirectionMode(DEVICE_GPIO_PIN_SCIRXDA, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(DEVICE_GPIO_PIN_SCIRXDA, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(DEVICE_GPIO_PIN_SCIRXDA, GPIO_QUAL_ASYNC);
    }
    else if(BootMode == SCI_BOOT_ALT6)
    {
        //
        // Configure GPIO8 as SCITXDA (Output pin)
        // Configure GPIO9 as SCIRXDA (Input pin)
        //
        //
        // GPIO8 is the SCI Tx pin.
        //
        GPIO_setControllerCore(8, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_8_SCIA_TX);
        GPIO_setDirectionMode(8, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(8, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(8, GPIO_QUAL_ASYNC);
        //
        // GPIO9 is the SCI Rx pin.
        //
        GPIO_setControllerCore(9, GPIO_CORE_CPU1);
        GPIO_setPinConfig(GPIO_9_SCIA_RX);
        GPIO_setDirectionMode(9, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(9, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(9, GPIO_QUAL_ASYNC);
    }
}


//
// assignSciaIOCPU2 - Assign SCIA module to CPU2 control
//
void assignSciaIOCPU2(uint32_t BootMode)
{
    EALLOW;
    //
    //SCIA connected to CPU2
    //
    SysCtl_selectCPUForPeripheralInstance(SYSCTL_CPUSEL_SCIA, SYSCTL_CPUSEL_CPU2);
    //
    //Relinquishes control
    //of clock configuration registers
    //
    SysCtl_setSemOwner(SYSCTL_CPUSEL_CPU2);

    SysCtl_setLowSpeedClock(SYSCTL_LSPCLK_PRESCALE_14);
    if(BootMode == SCI_BOOT)
        {
        //
        // Configure GPIO12 as SCITXDA (Output pin)
        // Configure GPIO13 as SCIRXDA (Input pin)
        //

        //
        // GPIO12 is the SCI Tx pin.
        //
        GPIO_setControllerCore(12, GPIO_CORE_CPU2);
        GPIO_setPinConfig(GPIO_12_SCIA_TX);
        GPIO_setDirectionMode(12, GPIO_DIR_MODE_OUT);
        GPIO_setPadConfig(12, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(12, GPIO_QUAL_ASYNC);

        //
        // GPIO13 is the SCI Rx pin.
        //
        GPIO_setControllerCore(13, GPIO_CORE_CPU2);
        GPIO_setPinConfig(GPIO_13_SCIA_RX);
        GPIO_setDirectionMode(13, GPIO_DIR_MODE_IN);
        GPIO_setPadConfig(13, GPIO_PIN_TYPE_STD);
        GPIO_setQualificationMode(13, GPIO_QUAL_ASYNC);
    }

        else if(BootMode == SCI_BOOT_ALT1)
        {
            //
            // Configure GPIO84 as SCITXDA (Output pin)
            // Configure GPIO85 as SCIRXDA (Input pin)
            //

            //
            // GPIO84 is the SCI Tx pin.
            //;
            GPIO_setControllerCore(84, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_84_SCIA_TX);
            GPIO_setDirectionMode(84, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(84, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(84, GPIO_QUAL_ASYNC);

            //
            // GPIO85 is the SCI Rx pin.
            //
            GPIO_setControllerCore(85, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_85_SCIA_RX);
            GPIO_setDirectionMode(85, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(85, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(85, GPIO_QUAL_ASYNC);
        }
        else if(BootMode == SCI_BOOT_ALT2)
        {
            //
            // Configure GPIO36 as SCITXDA (Output pin)
            // Configure GPIO35 as SCIRXDA (Input pin)
            //

            //
            // GPIO36 is the SCI Tx pin.
            //
            GPIO_setControllerCore(36, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_36_SCIA_TX);
            GPIO_setDirectionMode(36, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(36, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(36, GPIO_QUAL_ASYNC);

            //
            // GPIO35 is the SCI Rx pin.
            //
            GPIO_setControllerCore(35, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_35_SCIA_RX);
            GPIO_setDirectionMode(35, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(35, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(35, GPIO_QUAL_ASYNC);
        }
        else if(BootMode == SCI_BOOT_ALT3)
        {
            //
            // Configure GPIO42 as SCITXDA (Output pin)
            // Configure GPIO43 as SCIRXDA (Input pin)
            //

            //
            // GPIO42 is the SCI Tx pin.
            //
            GPIO_setControllerCore(42, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_42_SCIA_TX);
            GPIO_setDirectionMode(42, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(42, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(42, GPIO_QUAL_ASYNC);

            //
            // GPIO43 is the SCI Rx pin.
            //
            GPIO_setControllerCore(43, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_43_SCIA_RX);
            GPIO_setDirectionMode(43, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(43, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(43, GPIO_QUAL_ASYNC);
        }
        else if(BootMode == SCI_BOOT_ALT4)
        {
            //
            // Configure GPIO65 as SCITXDA (Output pin)
            // Configure GPIO64 as SCIRXDA (Input pin)
            //

            //
            // GPIO65 is the SCI Tx pin.
            //
            GPIO_setControllerCore(65, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_65_SCIA_TX);
            GPIO_setDirectionMode(65, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(65, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(65, GPIO_QUAL_ASYNC);

            //
            // GPIO64 is the SCI Rx pin.
            //
            GPIO_setControllerCore(64, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_64_SCIA_RX);
            GPIO_setDirectionMode(64, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(64, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(64, GPIO_QUAL_ASYNC);
        }
        else if(BootMode == SCI_BOOT_ALT5)
        {
            // Configure GPIO29 as SCITXDA (Output pin)
            // Configure GPIO28 as SCIRXDA (Input pin)
            //

            //
            // GPIO29 is the SCI Tx pin.
            //
            GPIO_setControllerCore(DEVICE_GPIO_PIN_SCITXDA, GPIO_CORE_CPU2);
            GPIO_setPinConfig(DEVICE_GPIO_CFG_SCITXDA);
            GPIO_setDirectionMode(DEVICE_GPIO_PIN_SCITXDA, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(DEVICE_GPIO_PIN_SCITXDA, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(DEVICE_GPIO_PIN_SCITXDA, GPIO_QUAL_ASYNC);

            //
            // GPIO28 is the SCI Rx pin.
            //
            GPIO_setControllerCore(DEVICE_GPIO_PIN_SCIRXDA, GPIO_CORE_CPU2);
            GPIO_setPinConfig(DEVICE_GPIO_CFG_SCIRXDA);
            GPIO_setDirectionMode(DEVICE_GPIO_PIN_SCIRXDA, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(DEVICE_GPIO_PIN_SCIRXDA, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(DEVICE_GPIO_PIN_SCIRXDA, GPIO_QUAL_ASYNC);
        }
        else if(BootMode == SCI_BOOT_ALT6)
        {
            //
            // Configure GPIO8 as SCITXDA (Output pin)
            // Configure GPIO9 as SCIRXDA (Input pin)
            //

            //
            // GPIO8 is the SCI Tx pin.
            //
            GPIO_setControllerCore(8, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_8_SCIA_TX);
            GPIO_setDirectionMode(8, GPIO_DIR_MODE_OUT);
            GPIO_setPadConfig(8, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(8, GPIO_QUAL_ASYNC);

            //
            // GPIO9 is the SCI Rx pin.
            //
            GPIO_setControllerCore(9, GPIO_CORE_CPU2);
            GPIO_setPinConfig(GPIO_9_SCIA_RX);
            GPIO_setDirectionMode(9, GPIO_DIR_MODE_IN);
            GPIO_setPadConfig(9, GPIO_PIN_TYPE_STD);
            GPIO_setQualificationMode(9, GPIO_QUAL_ASYNC);
        }
}

//
// assignSharedRAMsToCPU2 - Assign shared RAMs specified to CPU2
//
void assignSharedRAMsToCPU2(void)
{
    EALLOW;
    MemCfg_setGSRAMControllerSel(MEMCFG_SECT_GS3, MEMCFG_GSRAMCONTROLLER_CPU2);
    MemCfg_setGSRAMControllerSel(MEMCFG_SECT_GS4, MEMCFG_GSRAMCONTROLLER_CPU2);
    EDIS;
}

//
// assignSharedBANKsToCPU2 - Assign shared BANKs specified to CPU2
//
void assignSharedBANKsToCPU2(void)
{
    EALLOW;
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK3, SYSCTL_CPUSEL_CPU2);
    SysCtl_allocateFlashBank(SYSCTL_FLASH_BANK4, SYSCTL_CPUSEL_CPU2);
    EDIS;
}

//
// Placed in MSGRAM to copy into CPU2 M1 RAM
//
void goToCodeStartCPU2Kernel(void){
    asm(" LB 0x16000");
}

//
// set boot mode
//
void setBootModeForCPU2(void){

   IPC_releaseFlashSemaphore();

   //Clear all flags
   IPC_clearFlagLtoR(IPC_CPU1_L_CPU2_R, IPC_FLAG_ALL);
   //set boot mode
   Device_bootCPU2(BOOTMODE_IPC_MSGRAM_COPY_LENGTH_100W | BOOTMODE_IPC_MSGRAM_COPY_BOOT_TO_M1RAM);
}

//
// CPU1 is waiting for CPU2 to finish kernel operations
//
void waitForCPU2Signal(void)
{

   EALLOW;
   IPC_waitForFlag(IPC_CPU1_L_CPU2_R, IPC_FLAG5); //continues until CPU2 application is finished
   IPC_ackFlagRtoL(IPC_CPU1_L_CPU2_R, IPC_FLAG5); //clearing the acknowledgment flag
   EDIS;

}

//
// End of file
//
