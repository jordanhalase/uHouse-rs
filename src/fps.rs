#![cfg(feature = "fps")]

use arduino_hal;
use ufmt::{uwriteln, uWrite};
use core::sync::atomic::{AtomicBool, Ordering};
use avr_device::atmega328p::TC1;

static mut FPS_READY: AtomicBool = AtomicBool::new(false);

pub struct FpsCounter<W: uWrite> {
    count: u16,
    serial: W,
}

impl<W> FpsCounter<W> where W: uWrite {

    /// Create a new FPS Counter
    /// 
    /// This currently takes full ownership of the serial device
    pub fn new(serial: W, tc1: TC1) -> Self {
        setup_timer1(&tc1);
        Self {
            count: 0,
            serial: serial,
        }
    }

    /// Update the FPS Counter
    /// 
    /// Will reset and print the count to serial when the timer expires
    pub fn update(&mut self) {
        self.count += 1;

        // SAFETY: We own TC1 so we are effectively a singleton
        if unsafe { FPS_READY.load(Ordering::Acquire) } {
            unsafe { FPS_READY.store(false, Ordering::Release); }
            let _ = uwriteln!(self.serial, "{}", self.count);
            self.count = 0;
        }
    }
}

fn setup_timer1(tc1: &TC1) {
    use arduino_hal::pac::tc1::tccr1b::CS1_A;

    const CLOCK_SOURCE: CS1_A = CS1_A::PRESCALE_256;
    tc1.tccr1a.write(|w| w.wgm1().bits(0));
    tc1.tccr1b.write(|w| w.cs1()
        .variant(CLOCK_SOURCE)
        .wgm1()
        .bits(0b01)
    );
    tc1.tcnt1.write(|w| unsafe { w.bits(0) });
    tc1.ocr1a.write(|w| unsafe { w.bits(62500u16) }); // 16e6 >> 8
    tc1.timsk1.write(|w| w.ocie1a().set_bit()); // Enable this interrupt
}

#[avr_device::interrupt(atmega328p)]
fn TIMER1_COMPA() {
    unsafe {
        // SAFETY: This is an atomic bool to signal the timer expired
        FPS_READY.store(true, Ordering::SeqCst);
    }
}