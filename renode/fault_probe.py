# `marker_path`, if non-empty, gets a file written to it on the first
# hit -- lets a caller that can't read Renode's own log (e.g. a robot
# test, via `File Should Not Exist`/`File Should Exist`) detect a fault
# happened. Comparing PC to `address` *after* the fact doesn't work: by
# the time a script samples PC (e.g. after `emulation RunFor`),
# HardFault's own handler body has already executed a few instructions
# past its entry point, so PC no longer equals the entry address even
# though execution is still inside it.
def mc_probe_hard_fault(address, marker_path=""):
    bus = monitor.Machine.SystemBus
    state = {"hits": 0}

    def on_hit(cpu, addr):
        state["hits"] += 1
        if state["hits"] > 1:
            return
        sp = cpu.GetRegisterUlong(13)
        names = ["r0", "r1", "r2", "r3", "r12", "lr", "pc", "xpsr"]
        for i, name in enumerate(names):
            value = bus.ReadDoubleWord(sp + 4 * i)
            cpu.WarningLog("exception frame " + name + " = " + str(value))
        if marker_path:
            with open(marker_path, "w") as f:
                f.write("hard fault hit\n")

    for cpu in bus.GetCPUs():
        cpu.AddHook(address, on_hit)


def mc_dump_rtt(buffer_addr, length, out_path):
    bus = monitor.Machine.SystemBus
    data = bus.ReadBytes(buffer_addr, length)
    with open(out_path, "wb") as f:
        f.write(bytes(data))


# Drains whatever's new in up-channel 0 since the last drain (tracking
# RdOff properly, including ring-buffer wraparound) and appends it to
# out_path -- call this repeatedly across several short `emulation
# RunFor` hops to build a lossless transcript, instead of one big dump
# at the end that a noisy/chatty peripheral (e.g. a retrying SD-card
# driver) can overwrite before it's read. `cb_addr` is `_SEGGER_RTT`'s
# address (aUp[0]'s fields sit at fixed offsets from it -- see
# boot.resc's header comment for the layout).
def mc_drain_rtt(cb_addr, out_path):
    bus = monitor.Machine.SystemBus
    buffer_addr = bus.ReadDoubleWord(cb_addr + 0x1C)
    size = bus.ReadDoubleWord(cb_addr + 0x20)
    wroff = bus.ReadDoubleWord(cb_addr + 0x24)
    rdoff = bus.ReadDoubleWord(cb_addr + 0x28)

    if wroff == rdoff:
        return

    chunks = []
    if wroff > rdoff:
        chunks.append(bus.ReadBytes(buffer_addr + rdoff, wroff - rdoff))
    else:
        chunks.append(bus.ReadBytes(buffer_addr + rdoff, size - rdoff))
        chunks.append(bus.ReadBytes(buffer_addr, wroff))

    with open(out_path, "ab") as f:
        for chunk in chunks:
            f.write(bytes(chunk))

    bus.WriteDoubleWord(cb_addr + 0x28, wroff)


# Logs every CAN frame the given controller (e.g. `fdcan1`, passed bare
# -- Renode's monitor resolves it to the actual peripheral object) sends,
# one line per frame, to out_path. Used to confirm actual transmitted
# bytes (PGN 130306 wind data) rather than just trusting the encoder's
# own unit tests.
def mc_probe_can(obj, out_path):
    def on_frame(frame):
        try:
            line = "id=0x%08x ext=%s data=[%s]\n" % (frame.Id, frame.ExtendedId, frame.DataAsHex)
        except Exception as e:
            line = "CAN frame decode error: " + str(e) + "\n"
        with open(out_path, "a") as f:
            f.write(line)

    obj.FrameSent += on_frame
