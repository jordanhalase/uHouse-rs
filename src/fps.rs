#![cfg(feature = "fps")]

use super::CLOCK_FREQ;
use arduino_hal;
use avr_device::atmega328p::TC1;
use core::sync::atomic::{AtomicBool, Ordering};
use ufmt::{uWrite, uwriteln};

static FPS_READY: AtomicBool = AtomicBool::new(false);

pub struct FpsCounter<W: uWrite> {
    count: u16,
    serial: W,
}

impl<W> FpsCounter<W>
where
    W: uWrite,
{
    /// Create a new FPS Counter
    ///
    /// This currently takes full ownership of the serial device
    ///
    /// Interrupts must not yet be enabled before calling
    pub unsafe fn new(serial: W, tc1: TC1) -> Self {
        use arduino_hal::pac::tc1::tccr1b::CS1_A;

        const CLOCK_SOURCE: CS1_A = CS1_A::PRESCALE_256;
        tc1.tccr1a.write(|w| w.wgm1().bits(0));
        tc1.tccr1b
            .write(|w| w.cs1().variant(CLOCK_SOURCE).wgm1().bits(0b01));
        tc1.tcnt1.write(|w| w.bits(0));
        tc1.ocr1a.write(|w| w.bits((CLOCK_FREQ >> 8) as u16));
        tc1.timsk1.write(|w| w.ocie1a().set_bit()); // Enable this interrupt

        Self { count: 0, serial }
    }

    /// Update the FPS Counter
    ///
    /// Will reset and print the count to serial when the timer expires
    pub fn update(&mut self) {
        self.count += 1;

        if FPS_READY.load(Ordering::Acquire) {
            FPS_READY.store(false, Ordering::Release);
            let _ = uwriteln!(self.serial, "{}", self.count);
            self.count = 0;
        }
    }
}

#[avr_device::interrupt(atmega328p)]
fn TIMER1_COMPA() {
    // SAFETY: This is only otherwise modified by a singleton
    FPS_READY.store(true, Ordering::SeqCst);
}
