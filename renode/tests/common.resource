*** Settings ***
Library           OperatingSystem

*** Variables ***
# Default matches a local checkout; CI overrides it (repo mounted at
# /workspace, see ci/renode-test.sh) with `--variable
# WINDMETER_ROOT:/workspace`.
${WINDMETER_ROOT}    /home/qarol/git/windmeter/windmeter
${REPL}               @${WINDMETER_ROOT}/renode/stm32h723zg.repl
${ELF}                @${WINDMETER_ROOT}/target/thumbv7em-none-eabihf/debug/wind_meter
${FAULT_PROBE}         @${WINDMETER_ROOT}/renode/fault_probe.py

*** Keywords ***
# Common machine bring-up shared by every windmeter Renode test: create
# the machine, apply the RCC register-persistence workaround (see
# PROJECT_PRINCIPLES.md), load the real compiled ELF, and arm the
# HardFault marker-file detector (a hook, not a post-hoc PC comparison
# -- see PROJECT_PRINCIPLES.md for why that doesn't work). Every test
# should check `File Should Not Exist ${fault_marker}` after its own
# `emulation RunFor`, since a fault partway through would otherwise
# silently invalidate whatever else the test observed.
Boot Windmeter Firmware
    [Arguments]    ${fault_marker}
    Remove File               ${fault_marker}

    Execute Command           mach create "windmeter"
    Execute Command           machine LoadPlatformDescription ${REPL}
    Execute Command           cpu VectorTableOffset 0x8000000
    Execute Command           rcc AddBeforeReadDoubleWordHook 0x58 "value = 0x20000"
    Execute Command           rcc AddBeforeReadDoubleWordHook 0x50 "value = 0x10004000"
    Execute Command           sysbus LoadELF ${ELF}

    Execute Command           i ${FAULT_PROBE}
    Execute Command           probe_hard_fault \`sysbus GetSymbolAddress "HardFault_"\` "${fault_marker}"
