*** Settings ***
Library           OperatingSystem

*** Variables ***
${REPL}           @/home/qarol/git/windmeter/windmeter/renode/stm32h723zg.repl
${ELF}            @/home/qarol/git/windmeter/windmeter/target/thumbv7em-none-eabihf/debug/wind_meter
${FAULT_PROBE}     @/home/qarol/git/windmeter/windmeter/renode/fault_probe.py
${FAULT_MARKER}    /tmp/windmeter_boot_robot_hardfault.marker

*** Test Cases ***
Should Boot Without A Hard Fault
    Remove File               ${FAULT_MARKER}

    Execute Command           mach create "windmeter"
    Execute Command           machine LoadPlatformDescription ${REPL}
    Execute Command           cpu VectorTableOffset 0x8000000

    # Same Renode-side RCC register-persistence workaround as
    # renode/boot.resc -- see PROJECT_PRINCIPLES.md.
    Execute Command           rcc AddBeforeReadDoubleWordHook 0x58 "value = 0x20000"
    Execute Command           rcc AddBeforeReadDoubleWordHook 0x50 "value = 0x10004000"

    Execute Command           sysbus LoadELF ${ELF}

    Execute Command           i ${FAULT_PROBE}
    Execute Command           probe_hard_fault \`sysbus GetSymbolAddress "HardFault_"\` "${FAULT_MARKER}"

    Execute Command           emulation RunFor "2"

    File Should Not Exist     ${FAULT_MARKER}
