#![no_main]
#![no_std]

use cortex_m::singleton;
use cortex_m_rt::entry;
use distance_measurement::{DistanceError, Future};
use panic_reset as _;
//use panic_semihosting as _;
use defmt::write;
use stm32f3_discovery::stm32f3xx_hal::hal::digital::v2::{InputPin, OutputPin};
use stm32f3_discovery::stm32f3xx_hal::serial::*;
use stm32f3_discovery::stm32f3xx_hal::time::MonoTimer;
use stm32f3_discovery::stm32f3xx_hal::{delay::Delay, time::Bps};
use stm32f3_discovery::stm32f3xx_hal::{prelude::*, stm32};
use stm32f3_discovery::{stm32f3xx_hal::hal, switch_hal::OutputSwitch};

const SPEED_OF_SOUND: u16 = 340;

mod distance_measurement;

#[entry]
fn main() -> ! {
    let core_peripherals = cortex_m::peripheral::Peripherals::take().unwrap();
    let peripherals = stm32::Peripherals::take().unwrap();

    let mut flash = peripherals.FLASH.constrain();
    let mut rcc = peripherals.RCC.constrain();
    let mut parts = peripherals.GPIOE.split(&mut rcc.ahb);
    let mut input_parts = peripherals.GPIOB.split(&mut rcc.ahb);
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut gpioa = peripherals.GPIOA.split(&mut rcc.ahb);
    let mut gpioc = peripherals.GPIOC.split(&mut rcc.ahb);

    let pins = (
        gpioc.pc4.into_af7(&mut gpioc.moder, &mut gpioc.afrl),
        gpioc.pc5.into_af7(&mut gpioc.moder, &mut gpioc.afrl),
    );
    let (mut serial_tx, _serial_rx) =
        Serial::usart1(peripherals.USART1, pins, Bps(115200), clocks, &mut rcc.apb2).split();

    let dma_channels = peripherals.DMA1.split(&mut rcc.ahb);

    let mut leds = stm32f3_discovery::leds::Leds::new(
        parts.pe8,
        parts.pe9,
        parts.pe10,
        parts.pe11,
        parts.pe12,
        parts.pe13,
        parts.pe14,
        parts.pe15,
        &mut parts.moder,
        &mut parts.otyper,
    )
    .into_array();

    let mono_timer = MonoTimer::new(core_peripherals.DWT, clocks);
    let mut delay = Delay::new(core_peripherals.SYST, clocks);
    let us_drive_pin = input_parts
        .pb6
        .into_push_pull_output(&mut input_parts.moder, &mut input_parts.otyper);
    let us_measure_pin = input_parts
        .pb9
        .into_pull_down_input(&mut input_parts.moder, &mut input_parts.pupdr);

    let user_button = gpioa
        .pa0
        .into_pull_down_input(&mut gpioa.moder, &mut gpioa.pupdr);

    let mut measure =
        distance_measurement::DistanceMeasurement::new(us_drive_pin, us_measure_pin, &mono_timer);

    let mut distance_mm;

    // the data we are going to send over serial
    let mut tx_buf = singleton!(: [u8; 128] = [0; 128]).unwrap();
    let mut tx_channel = dma_channels.ch4;

    loop {
        if user_button.is_high().unwrap() {
            measure.start();
        }

        //delay.delay_ms((distance * 1000f32) as u32);

        match measure.poll() {
            Err(DistanceError::NoEcho) => {
                measure.reset();
            }
            Err(DistanceError::PinError(_err)) => todo!("Handle errors"),
            Ok(Future::Complete(duration)) => {
                distance_mm = duration.as_secs_f32() / 2f32 * SPEED_OF_SOUND as f32 * 1000f32;
                let measured_heigth = 2000.0f32 - distance_mm;
                let factor = measured_heigth / 2000f32;
                /*
                    let mut wrapp: Wrapper<128> = tx_buf.into();
                    write!(wrapp, "{:?} %", factor);
                */

                tx_buf[..11].copy_from_slice(b"Fullstand: ");
                {
                    let remaining_buf = print(&mut tx_buf[11..], (factor * 100f32) as u8);
                    remaining_buf[..4].copy_from_slice(b"% \r\n");
                }

                let res = serial_tx.write_all(tx_buf, tx_channel).wait();

                serial_tx = res.2;
                tx_buf = res.0;
                tx_channel = res.1;

                let count = (factor * 8f32) as usize;
                for led in &mut leds[..count] {
                    let _ = led.on();
                }

                for led in &mut leds[count..] {
                    let _ = led.off();
                }
            }
            Ok(Future::Pending) => delay.delay_us(1u8),
            Ok(Future::NotStarted) => {
                delay.delay_ms(1000u32);
                measure.start();

                for led in &mut leds {
                    let _ = led.off();
                }
            }
        }
    }
}

fn print(mut buffer: &mut [u8], mut value: u8) -> &mut [u8] {
    let digit_1 = b"1"[0];
    let mut started: bool = false;
    if value >= 100 {
        let digit_value = value.div_euclid(100);
        value = value.rem_euclid(100);
        buffer[0] = digit_1 + (digit_value - 1);
        buffer = &mut buffer[1..];
        started = true;
    }

    if value >= 10 && value < 100 {
        let digit_value = value.div_euclid(10);
        value = value.rem_euclid(10);
        buffer[0] = digit_1 + (digit_value - 1);
        buffer = &mut buffer[1..];
    } else if value < 10 && started {
        buffer[0] = b"0"[0];
        buffer = &mut buffer[1..];
    }

    if value >= 1 && value < 10 {
        let digit_value = value.div_euclid(1);
        buffer[0] = digit_1 + (digit_value - 1);
        buffer = &mut buffer[1..];
    } else if value < 1 {
        buffer[0] = b"0"[0];
        buffer = &mut buffer[1..];
    }

    buffer
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
            true => match self.pin.set_low() {
                Ok(_) => self.state = false,
                Err(_) => {}
            },
            false => {
                if let Ok(_) = self.pin.set_high() {
                    self.state = true;
                }
            }
        };
    }
}
