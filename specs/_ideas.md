# Ideas

Flat parking lot for "might be neat" observations -- passive capture, not
actively pondered yet. See `ws._meta/lifecycle.md` for graduation rules
(idea -> seed -> spec).

- A wireless output in a standard sensor protocol (BLE is the likely
  candidate), so the separate head_unit display project (`../head_unit`)
  can show windmeter's data. The display itself graduated out of this
  list into that project on 2026-10-05; windmeter-v1 still ships
  NMEA2000-only, and the STM32H723 has no radio, so this would mean added
  hardware. Wait for head_unit to pick its wireless standard.
