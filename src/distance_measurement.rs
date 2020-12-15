use core::time::Duration;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32f3_discovery::stm32f3xx_hal::hal as embedded_hal;
use stm32f3_discovery::stm32f3xx_hal::time::{Instant, MonoTimer};
use stm32f3_discovery::switch_hal::{ActiveLow, IntoSwitch, OutputSwitch, Switch};

pub enum Future<T> {
    NotStarted,
    Pending,
    Complete(T),
}

pub enum DistanceError<TOutError> {
    PinError(TOutError),
    NoEcho,
}

enum States {
    Idle,
    SendPulse(Instant),
    WaitAnswerPulse,
    MeasureAnswerPulse(Instant),
}

pub struct DistanceMeasurement<'timer, TIn, TOut, TOutError>
where
    TIn: InputPin,
    TOut: OutputPin<Error = TOutError>,
{
    state: States,
    driver: Switch<TOut, ActiveLow>,
    receiver: TIn,
    timer: &'timer MonoTimer,
}

impl<T> From<T> for DistanceError<T> {
    fn from(x: T) -> Self {
        DistanceError::PinError(x)
    }
}

impl<'timer, TIn, TOut, TOutError> DistanceMeasurement<'timer, TIn, TOut, TOutError>
where
    TIn: InputPin,
    TOut: OutputPin<Error = TOutError>,
{
    pub fn new(
        trigger: TOut,
        echo: TIn,
        timer: &'timer MonoTimer,
    ) -> DistanceMeasurement<'timer, TIn, TOut, TOutError> {
        let driver = trigger.into_active_low_switch();
        DistanceMeasurement {
            state: States::Idle,
            driver: driver,
            receiver: echo,
            timer,
        }
    }

    pub fn start(&mut self) {
        self.state = States::SendPulse(self.timer.now());
    }

    pub fn poll(&mut self) -> Result<Future<Duration>, DistanceError<TOutError>> {
        const MIN_LOW_PULSE: Duration = Duration::from_micros(10);
        const ECHO_TIME_OUT: Duration = Duration::from_millis(200);
        match self.state {
            States::Idle => {
                self.driver.off()?;
                Ok(Future::NotStarted)
            }
            States::SendPulse(start) => {
                self.driver.on()?;
                if self.get_duration(&start) > MIN_LOW_PULSE {
                    self.state = States::WaitAnswerPulse;
                }
                Ok(Future::Pending)
            }
            States::WaitAnswerPulse => {
                self.driver.on()?;
                match self.receiver.is_high() {
                    Ok(true) => self.state = States::MeasureAnswerPulse(self.timer.now()),
                    _ => (),
                }
                Ok(Future::Pending)
            }
            States::MeasureAnswerPulse(start) => match self.receiver.is_low() {
                Ok(true) => {
                    let result = self.get_duration(&start);
                    self.state = States::Idle;
                    Ok(Future::Complete(result))
                }
                Ok(false) => {
                    if self.get_duration(&start) < ECHO_TIME_OUT {
                        Ok(Future::Pending)
                    } else {
                        Err(DistanceError::NoEcho)
                    }
                }
                _ => Ok(Future::Pending),
            },
        }
    }

    fn get_duration(&self, start: &Instant) -> Duration {
        Duration::from_micros(
            (start.elapsed() * 1000_000) as u64 / (self.timer.frequency().0 as u64),
        )
    }
}
