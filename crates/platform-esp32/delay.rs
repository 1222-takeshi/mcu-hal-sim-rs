//! Delay adapter for ESP32: wraps embedded-hal DelayNs to provide platform delay.

use embedded_hal::delay::DelayNs;

/// Wraps any `embedded_hal::delay::DelayNs` implementation.
pub struct Esp32Delay<D> {
    inner: D,
}

impl<D: DelayNs> Esp32Delay<D> {
    pub fn new(inner: D) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> D {
        self.inner
    }
}

impl<D: DelayNs> DelayNs for Esp32Delay<D> {
    fn delay_ns(&mut self, ns: u32) {
        self.inner.delay_ns(ns);
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingDelay {
        ns_total: u64,
    }

    impl DelayNs for CountingDelay {
        fn delay_ns(&mut self, ns: u32) {
            self.ns_total += u64::from(ns);
        }
    }

    #[test]
    fn esp32_delay_forwards_ns() {
        let inner = CountingDelay { ns_total: 0 };
        let mut delay = Esp32Delay::new(inner);

        delay.delay_ns(1_000);
        delay.delay_ns(2_000);

        assert_eq!(delay.into_inner().ns_total, 3_000);
    }

    #[test]
    fn esp32_delay_exposes_inner() {
        let inner = CountingDelay { ns_total: 42 };
        let delay = Esp32Delay::new(inner);

        assert_eq!(delay.into_inner().ns_total, 42);
    }
}
