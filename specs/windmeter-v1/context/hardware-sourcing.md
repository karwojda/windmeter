# Hardware Sourcing (Poland)

Research date: 2026-09-18, wind sensor revised 2026-09-21. Component
selection is recorded formally in `requirements/model/system.sysml`
(each part def's `doc` block); this file is the sourcing/pricing record
behind those choices -- the "why this specific product" that doesn't
belong in the architecture model.

Prices found via web search on Polish retailer listings; e-commerce
prices are volatile and several of these sites render prices via
JavaScript (not visible to a plain fetch), so **confirmed** prices were
read directly off search results, **estimated** ones were not found and
should be checked on the live page before ordering.

## Bill of materials

| Component | Selected | Why | Poland source | Price |
|---|---|---|---|---|
| MCU devkit | NUCLEO-H723ZG | Official ST board, onboard ST-LINK/V3, Arduino+Zio/Morpho headers -- matches the STM32H723ZG already targeted in `Cargo.toml`/`memory.x` | [Kamami](https://kamami.pl/en/stm-nucleo-144/581677-nucleo-h723zg-starter-kit-with-a-microcontroller-from-the-stm32-family-stm32h723zg-5906623436453.html) | **159 PLN confirmed** |
| Wind sensor (speed+direction) | Davis Instruments 6410 (Vantage Pro2 anemometer + vane) | Stainless steel bearings, 200 mph wind-tunnel tested, field-proven 10+ year lifespans incl. DIY marine/boat use -- see § Wind Sensor Ruggedness Comparison below for what this replaced and why | [Allegro](https://allegro.pl/wiatromierz-anemometr-czujnik-wiatru-davis-6410-i7333596839.html), also MeteoPlus, StacjaMeteo (Polish meteo-equipment specialists) | **~824.50 PLN confirmed** (Allegro) |
| GPS module | u-blox NEO-6M breakout (e.g. GY-NEO6MV2) | Cheapest common hobby GPS, 9600 baud NMEA0183 default, matches `gps.rs`'s `nmea0183` parser. NEO-M8N is a pin-compatible upgrade if accuracy is insufficient | Allegro (widely listed), also TME/Kamami stock bare NEO-6M/M8N modules | **~27-50 PLN confirmed** (Allegro, several listings) |
| CAN transceiver | Waveshare SN65HVD230 board | 3.3V logic (direct STM32 connection, no level shifter), pinout-compatible with PCA82C250, standard NMEA2000-adjacent choice | [Botland](https://botland.store/can-bus-modules/6960-sn65hvd230-can-board-waveshare-3945-5904422300715.html), also [Kamami](https://kamami.pl/en/CAN-converters/569521-waveshare-can-interface-module-sn65hvd230--5906623439409.html), elty.pl | **~4 EUR confirmed** (Botland, ~17-18 PLN) |
| SD storage | Generic microSD SPI breakout (3.3V) | Matches `logger.rs`'s `embedded-sdmmc` driver; no special part needed | [Botland](https://botland.store/259-memory-cards-accessories) (several models), msalamon.pl, mikrobot.pl | Not confirmed -- typically ~10-20 PLN for a bare 3.3V module |
| Battery + charger | Single-cell Li-ion/LiPo (e.g. 18650) + TP4056 USB-C charge/protection module | Cheapest standalone rechargeable option for REQ-008 (dinghy/no-power-supply case); buck/boost to the board's rail(s) still needs picking | [Kamami](https://kamami.pl/en/li-po-chargers-modules/577708-tp4056-li-ion-usb-type-c-1a-charger-module-with-protection-5906623457595.html), Botland, Allegro | **~3-4 PLN confirmed** (charger module only, Allegro); battery cell separate |

No external pull resistor needed for the direction sense (unlike the
earlier SEN-15901 choice) -- Davis's vane is a self-contained 3-wire
potentiometer (see § Wiring below).

**Rough total (confirmed items + estimates): ~1000-1100 PLN**, excluding
the battery cell itself. The wind sensor is now by far the largest line
item -- see below for why that trade was made deliberately.

## Wind Sensor Ruggedness Comparison

The original SEN-15901 pick (Argent Data Systems / SparkFun Weather
Meter Kit) is a general-purpose hobbyist/education weather-station
sensor -- "tough molded plastic" with bearings of unspecified material,
no wind-tunnel rating, no long-term field-life data found, and no marine
branding or track record. Reasonable for a bench prototype, questionable
for a masthead exposed to UV, salt spray, and vibration for years.

Four alternatives researched, in increasing order of how much they'd
change this project's architecture:

| Option | Interface | Ruggedness case | Price (approx) | Architecture impact |
|---|---|---|---|---|
| SEN-15901 (previous pick) | Raw pulse + resistor-ladder vane | "Tough molded plastic," sealed bearings (material unspecified); no wind-tunnel/field-life data found | ~$50-70 | none (already built) |
| **Davis 6410 (chosen)** | Raw pulse + continuous pot | Stainless steel ball bearings, 200 mph wind-tunnel tested, 10-12+ year field reports (incl. DIY marine use per sailing forums); NOT marine-certified, but the most-cited durable choice in the DIY marine-electronics community | ~824 PLN | Low: recalibrate `mps_per_hz`, replace lookup-table direction decode with a linear one (done) |
| Garmin GWS 10 | NMEA2000 native | Genuinely marine-branded ("Marine Wind Sensor"), elliptical cups, bus-powered | ~$465-489 (~1900-2000 PLN) | High: outputs a finished wind PGN itself -- `wind_sensor.rs`'s whole driver role disappears, REQ-001/REQ-004 (raw sensing, sensor swappability) would need re-scoping |
| B&G 508 | NMEA2000/SimNet native | "Ocean-race proven" (B&G/Simrad), used across the Triton/IS20/IS40 line | Not found | Same as Garmin GWS 10 -- NMEA2000-native, not a raw sensor |

[Practical Sailor's wind sensor test](https://www.practical-sailor.com/marine-electronics/wind-sensor-testing/)
(indoor wind-tunnel comparison of NKE HR, Nexus nWind, Garmin GWS 10,
Raymarine Tacktick/i60, B&G Triton 508, Sailtimer) rated the NKE HR
"Best Choice" for construction (anodized machined aluminum on a
polished stainless bracket) and the B&G 508 as its budget-priced
recommendation for cruisers -- neither tested for saltwater/UV lifespan,
and both are NMEA2000-native like the Garmin/B&G options above, not
raw-sensor alternatives.

**Decision**: Davis 6410. It's the only option that meaningfully improves
ruggedness over SEN-15901 *without* changing what this project's
firmware actually does (still senses + computes + logs + outputs, not
just relays an existing PGN). The NMEA2000-native options are a
legitimate different path -- noted here in case the Davis's build quality
still proves insufficient after real-world testing -- but weren't chosen
because they'd turn REQ-001/REQ-004 into "listen for an existing wind
PGN," a different project than the one `requirements.md` describes.

## Wiring

Davis 6410's vane runs its potentiometer between the datasheet's nominal
5V and GND (Yellow = 5V, Red = GND, Green = wiper). **Power it from the
STM32's 3.3V rail instead** -- it's a passive resistor, so this just
rescales its 0..360° sweep to a 0..3.3V wiper output, which is what the
STM32's ADC (3.3V-referenced, absolute max input = VDD) needs; wiring it
to the datasheet's 5V would risk feeding the ADC pin above its rated
input range near the top of the sweep. No external pull resistor is
needed -- the vane's own two end-terminals form the divider.

Anemometer: black wire is a plain switch closure (reed/Hall) to ground --
same interface pattern as SEN-15901's, wire to the same EXTI-capable pin
with an internal or external pull-up, as already implemented.

## What this research changed in the code (not just the model)

Selecting real components surfaced actual behavior that the earlier
placeholder assumptions got wrong:

- **Anemometer constant (original SEN-15901 research)**: 2.4 km/h per
  pulse/second (datasheet) -> `mps_per_hz = 2.4/3.6 ≈ 0.667`, not the
  placeholder `0.5`.
- **Anemometer constant (Davis 6410 switch)**: Davis's documented
  constant is 2.25 mph per Hz -> `mps_per_hz = 2.25 * 0.44704 ≈ 1.006`
  m/s per Hz, replacing the SEN-15901 value above.
- **Direction decode changed from a lookup table to linear**:
  SEN-15901's vane was a resistor ladder (8 reed switches, up to 16
  resolvable positions), decoded by nearest-match against a 16-entry ADC
  table. Davis 6410's vane is a genuine continuous ~20kΩ potentiometer
  -- a straight `adc / adc_max * 360` scale, which is *simpler* code, not
  more complex (`wind_sensor.rs::adc_to_direction_deg`, no table). The
  scale is calculated from the datasheet, not measured against a real
  vane yet -- confirm during the hardware pass, including whether the
  pot has a dead zone at the wrap-around point that a pure linear model
  doesn't account for.
- **GPS baud rate**: NEO-6M/NEO-M8N default to 9600 baud; `embassy-stm32`'s
  `usart::Config::default()` is 115200. This would have silently
  desynced the NMEA parser -- now set explicitly (`wind_meter.rs`).
- **Nucleo-H723ZG's actual SPI1 pin routing**: the Arduino/Zio header
  wires SPI1 MOSI to `PB5`, not `PA7` -- `PA7` is a chip-level-valid
  alternate function that happened to compile but isn't what's
  physically exposed on this specific board's header.

## Still open (confirm on the hardware pass, per `tasks.md`)

- Wind vane's *actual* ADC readings on real hardware (linear scale is
  datasheet-derived, not measured) -- including whether the pot has a
  dead zone at the wrap-around point.
- Confirm the vane is actually wired to 3.3V, not the datasheet's
  nominal 5V (see § Wiring) -- getting this wrong risks the ADC pin.
- `direction_offset_deg` (mounting alignment) -- inherently
  installation-specific, can't be researched.
- Buck/boost regulator choice from the battery to the board's supply
  rail(s) -- not selected yet.
- Whether `PA0`/`PA1`/`PA4` (anemometer pulse, direction ADC, SD chip
  select) are actually free/exposed on the Nucleo's Zio header for this
  specific use -- SPI/USART/FDCAN pins were cross-checked against ST's
  documented routing; these three were not.
