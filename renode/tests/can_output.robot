*** Settings ***
Resource          common.resource

*** Variables ***
${FAULT_MARKER}    /tmp/windmeter_can_robot_hardfault.marker
${CAN_LOG}         /tmp/windmeter_can_robot.log

*** Test Cases ***
Should Transmit An NMEA2000 Wind Data Frame
    Remove File                ${CAN_LOG}

    Boot Windmeter Firmware    ${FAULT_MARKER}

    # fdcan1 (REQ-006's NMEA2000 output, PGN 130306) -- connect it to a
    # hub and capture every frame it actually transmits, proving the
    # real FDCAN1 driver code, not just nmea2000.rs's own unit-tested
    # encoder.
    Execute Command             emulation CreateCANHub "canHub"
    Execute Command             connector Connect fdcan1 canHub
    Execute Command             probe_can fdcan1 "${CAN_LOG}"

    Execute Command             emulation RunFor "2"

    File Should Not Exist       ${FAULT_MARKER}
    File Should Exist           ${CAN_LOG}
    ${frames}=                  Get File    ${CAN_LOG}
    # 0x09fd0242: priority 3 << 26 | PGN 130306 (0x1FD02) << 8 | source
    # address 0x42 (NMEA2000_SOURCE_ADDRESS in wind_meter.rs), the exact
    # 29-bit extended CAN ID wind_data_frame() builds -- see
    # nmea2000.rs. No sensor stimulus in this test, so the data bytes
    # are the "not available" pattern (0xFFFF fields, 0xF0 reference
    # byte) -- still a real, correctly-encoded frame, just an invalid
    # reading's encoding rather than a valid one's.
    Should Contain               ${frames}    id=0x09fd0242
