#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use distance_measurement::{DistanceError, DistanceMeasurement, Future};
use panic_reset as _;
use stm32f3_discovery::stm32f3xx_hal::delay::Delay;
use stm32f3_discovery::stm32f3xx_hal::hal;
use stm32f3_discovery::stm32f3xx_hal::time::MonoTimer;
use stm32f3_discovery::stm32f3xx_hal::{prelude::*, stm32};

mod distance_measurement;
mod wave_generator;

#[entry]
fn main() -> ! {
    let core_peripherals = cortex_m::peripheral::Peripherals::take().unwrap();
    let peripherals = stm32::Peripherals::take().unwrap();

    let mut flash = peripherals.FLASH.constrain();
    let mut rcc = peripherals.RCC.constrain();
    let mut parts = peripherals.GPIOE.split(&mut rcc.ahb);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut north = parts
        .pe9
        .into_push_pull_output(&mut parts.moder, &mut parts.otyper);

    let mut south = parts
        .pe10
        .into_push_pull_output(&mut parts.moder, &mut parts.otyper);

    let mono_timer = MonoTimer::new(core_peripherals.DWT, clocks);
    let mut delay = Delay::new(core_peripherals.SYST, clocks);
    let mut us_drive_pin = parts
        .pe0
        .into_push_pull_output(&mut parts.moder, &mut parts.otyper);
    let mut us_measure_pin = parts
        .pe1
        .into_pull_down_input(&mut parts.moder, &mut parts.pupdr);

    let mut gpio_a_parts = peripherals.GPIOA.split(&mut rcc.ahb);
    let user_button = gpio_a_parts
        .pa0
        .into_pull_down_input(&mut gpio_a_parts.moder, &mut gpio_a_parts.pupdr);

    let mut measure =
        distance_measurement::DistanceMeasurement::new(us_drive_pin, us_measure_pin, &mono_timer);
    let mut blink = Blink::new(north);

    loop {
        blink.toggle();
        match measure.poll() {
            Err(DistanceError::NoEcho) => {
                south.set_low();
            }
            Err(DistanceError::PinError(_err)) => todo!("Handle errors"),
            Ok(Future::Complete(_duration)) => {
                south.set_low();
                delay.delay_ms(500u16);
            }
            Ok(Future::Pending) => delay.delay_us(10u8),
            Ok(Future::NotStarted) => {
                if let Ok(true) = user_button.is_high() {
                    measure.start();
                    south.set_high();
                }
            }
        }
    }
}

struct Blink<TPin>
where
    TPin: hal::digital::v2::OutputPin,
{
    state: bool,
    pin: TPin,
}

impl<TPin> Blink<TPin>
where
    TPin: hal::digital::v2::OutputPin,
{
    fn new(pin: TPin) -> Blink<TPin> {
        Blink { state: false, pin }
    }

    fn toggle(&mut self) -> () {
        match self.state {
            true => {
                self.pin.set_low();
                self.state = false;
            }
            false => {
                self.pin.set_high();
                self.state = true;
            }
        };
    }
}
