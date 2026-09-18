# Hardware Sourcing (Poland)

Research date: 2026-09-18. Component selection is recorded formally in
`requirements/model/system.sysml` (each part def's `doc` block); this
file is the sourcing/pricing record behind those choices -- the "why
this specific product" that doesn't belong in the architecture model.

Prices found via web search on Polish retailer listings; e-commerce
prices are volatile and several of these sites render prices via
JavaScript (not visible to a plain fetch), so **confirmed** prices were
read directly off search results, **estimated** ones were not found and
should be checked on the live page before ordering.

## Bill of materials

| Component | Selected | Why | Poland source | Price |
|---|---|---|---|---|
| MCU devkit | NUCLEO-H723ZG | Official ST board, onboard ST-LINK/V3, Arduino+Zio/Morpho headers -- matches the STM32H723ZG already targeted in `Cargo.toml`/`memory.x` | [Kamami](https://kamami.pl/en/stm-nucleo-144/581677-nucleo-h723zg-starter-kit-with-a-microcontroller-from-the-stm32-family-stm32h723zg-5906623436453.html) | **159 PLN confirmed** |
| Wind sensor (speed+direction) | Weather Meter Kit, SEN-15901 (Argent Data Systems, sold as SparkFun) | Pulse anemometer + resistor-ladder wind vane, RJ11 cables, no soldering, well-documented calibration -- directly matches `wind_sensor.rs`'s driver | [Kamami](https://kamami.pl/en/motion-sensors/584241-weather-meter-kit-stacja-pogodowa-z-czujnikiem-opadow-oraz-kierunku-i-predkosci-wiatru-sen-15901.html) (also Distrelec PL) | Not found (import item; typically ~$50-70 -- **check live page**) |
| GPS module | u-blox NEO-6M breakout (e.g. GY-NEO6MV2) | Cheapest common hobby GPS, 9600 baud NMEA0183 default, matches `gps.rs`'s `nmea0183` parser. NEO-M8N is a pin-compatible upgrade if accuracy is insufficient | Allegro (widely listed), also TME/Kamami stock bare NEO-6M/M8N modules | **~27-50 PLN confirmed** (Allegro, several listings) |
| CAN transceiver | Waveshare SN65HVD230 board | 3.3V logic (direct STM32 connection, no level shifter), pinout-compatible with PCA82C250, standard NMEA2000-adjacent choice | [Botland](https://botland.store/can-bus-modules/6960-sn65hvd230-can-board-waveshare-3945-5904422300715.html), also [Kamami](https://kamami.pl/en/CAN-converters/569521-waveshare-can-interface-module-sn65hvd230--5906623439409.html), elty.pl | **~4 EUR confirmed** (Botland, ~17-18 PLN) |
| SD storage | Generic microSD SPI breakout (3.3V) | Matches `logger.rs`'s `embedded-sdmmc` driver; no special part needed | [Botland](https://botland.store/259-memory-cards-accessories) (several models), msalamon.pl, mikrobot.pl | Not confirmed -- typically ~10-20 PLN for a bare 3.3V module |
| Battery + charger | Single-cell Li-ion/LiPo (e.g. 18650) + TP4056 USB-C charge/protection module | Cheapest standalone rechargeable option for REQ-008 (dinghy/no-power-supply case); buck/boost to the board's rail(s) still needs picking | [Kamami](https://kamami.pl/en/li-po-chargers-modules/577708-tp4056-li-ion-usb-type-c-1a-charger-module-with-protection-5906623457595.html), Botland, Allegro | **~3-4 PLN confirmed** (charger module only, Allegro); battery cell separate |
| Direction-sense pull resistor | 10 kΩ resistor | External pull resistor the wind vane's ADC divider assumes (see below) | Any component shop / kit | negligible |

**Rough total (confirmed items + estimates): ~230-350 PLN**, excluding the
wind sensor kit (likely the single biggest line item as an imported
product) and the battery cell itself.

## What this research changed in the code (not just the model)

Selecting real components surfaced actual behavior that the earlier
placeholder assumptions got wrong:

- **Anemometer constant**: 2.4 km/h per pulse/second (datasheet) ->
  `mps_per_hz = 2.4/3.6 ≈ 0.667`, not the placeholder `0.5`
  (`wind_sensor.rs`).
- **Wind vane is a resistor ladder, not a linear pot**: direction is
  decoded by nearest-match against a 16-entry ADC lookup table derived
  from the vane's documented per-direction resistance (assuming a 10kΩ
  external pull resistor and the STM32's 12-bit ADC), not a linear
  `adc/adc_max * 360` scale (`wind_sensor.rs::VANE_ADC_TABLE`). The table
  is calculated from the datasheet's resistance values, not measured
  against a real vane yet -- confirm during the hardware pass.
- **GPS baud rate**: NEO-6M/NEO-M8N default to 9600 baud; `embassy-stm32`'s
  `usart::Config::default()` is 115200. This would have silently
  desynced the NMEA parser -- now set explicitly (`wind_meter.rs`).
- **Nucleo-H723ZG's actual SPI1 pin routing**: the Arduino/Zio header
  wires SPI1 MOSI to `PB5`, not `PA7` -- `PA7` is a chip-level-valid
  alternate function that happened to compile but isn't what's
  physically exposed on this specific board's header.

## Still open (confirm on the hardware pass, per `tasks.md`)

- Wind vane's *actual* ADC readings on real hardware (table is
  datasheet-derived, not measured) -- and confirm the 10kΩ pull
  resistor is actually populated as assumed.
- `direction_offset_deg` (mounting alignment) -- inherently
  installation-specific, can't be researched.
- Buck/boost regulator choice from the battery to the board's supply
  rail(s) -- not selected yet.
- Whether `PA0`/`PA1`/`PA4` (anemometer pulse, direction ADC, SD chip
  select) are actually free/exposed on the Nucleo's Zio header for this
  specific use -- SPI/USART/FDCAN pins were cross-checked against ST's
  documented routing; these three were not.
