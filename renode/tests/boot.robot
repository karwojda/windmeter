*** Settings ***
Resource          common.resource

*** Variables ***
${FAULT_MARKER}    /tmp/windmeter_boot_robot_hardfault.marker

*** Test Cases ***
Should Boot Without A Hard Fault
    Boot Windmeter Firmware    ${FAULT_MARKER}
    Execute Command            emulation RunFor "2"
    File Should Not Exist      ${FAULT_MARKER}
