def mc_probe_hard_fault(address):
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

    for cpu in bus.GetCPUs():
        cpu.AddHook(address, on_hit)


def mc_dump_rtt(buffer_addr, length, out_path):
    bus = monitor.Machine.SystemBus
    data = bus.ReadBytes(buffer_addr, length)
    with open(out_path, "wb") as f:
        f.write(bytes(data))
