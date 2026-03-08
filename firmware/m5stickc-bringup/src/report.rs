#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImuVariant {
    None,
    Mpu6886,
    Sh200q,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeSummary {
    pub axp192_found: bool,
    pub bm8563_found: bool,
    pub mpu6886_found: bool,
    pub sh200q_found: bool,
    pub bme280_primary_found: bool,
    pub bme280_secondary_found: bool,
}

impl ProbeSummary {
    pub const fn imu_variant(&self) -> ImuVariant {
        match (self.mpu6886_found, self.sh200q_found) {
            (false, false) => ImuVariant::None,
            (true, false) => ImuVariant::Mpu6886,
            (false, true) => ImuVariant::Sh200q,
            (true, true) => ImuVariant::Conflict,
        }
    }

    pub const fn onboard_i2c_alive(&self) -> bool {
        self.axp192_found
            || self.bm8563_found
            || self.mpu6886_found
            || self.sh200q_found
    }

    pub const fn pmu_rtc_status(&self) -> &'static str {
        match (self.axp192_found, self.bm8563_found) {
            (true, true) => "alive",
            (true, false) | (false, true) => "partial",
            (false, false) => "missing",
        }
    }

    pub const fn imu_status(&self) -> &'static str {
        match self.imu_variant() {
            ImuVariant::None => "missing",
            ImuVariant::Mpu6886 => "mpu6886",
            ImuVariant::Sh200q => "sh200q",
            ImuVariant::Conflict => "conflict",
        }
    }

    pub const fn external_bme280_status(&self) -> &'static str {
        if self.bme280_primary_found && self.bme280_secondary_found {
            "both"
        } else if self.bme280_primary_found {
            "0x76"
        } else if self.bme280_secondary_found {
            "0x77"
        } else {
            "missing"
        }
    }

    pub const fn health_hint(&self) -> &'static str {
        if !self.onboard_i2c_alive() {
            "no common onboard devices responded; check board power, USB, and I2C lines"
        } else if matches!(self.imu_variant(), ImuVariant::Conflict) {
            "multiple IMU variants responded; verify board revision and I2C signal integrity"
        } else if matches!(self.imu_variant(), ImuVariant::None) {
            "PMU/RTC path responded, but no expected IMU answered"
        } else if self.axp192_found && self.bm8563_found {
            "onboard PMU/RTC path looks alive"
        } else {
            "onboard I2C path is only partially visible"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary() -> ProbeSummary {
        ProbeSummary {
            axp192_found: true,
            bm8563_found: true,
            mpu6886_found: false,
            sh200q_found: false,
            bme280_primary_found: false,
            bme280_secondary_found: false,
        }
    }

    #[test]
    fn detects_missing_onboard_bus() {
        let summary = ProbeSummary {
            axp192_found: false,
            bm8563_found: false,
            mpu6886_found: false,
            sh200q_found: false,
            bme280_primary_found: false,
            bme280_secondary_found: false,
        };

        assert!(!summary.onboard_i2c_alive());
        assert_eq!(summary.pmu_rtc_status(), "missing");
        assert_eq!(summary.imu_status(), "missing");
    }

    #[test]
    fn classifies_expected_mpu6886_board() {
        let mut summary = summary();
        summary.mpu6886_found = true;

        assert!(summary.onboard_i2c_alive());
        assert_eq!(summary.imu_variant(), ImuVariant::Mpu6886);
        assert_eq!(summary.health_hint(), "onboard PMU/RTC path looks alive");
    }

    #[test]
    fn classifies_imu_conflict() {
        let mut summary = summary();
        summary.mpu6886_found = true;
        summary.sh200q_found = true;

        assert_eq!(summary.imu_variant(), ImuVariant::Conflict);
        assert_eq!(
            summary.health_hint(),
            "multiple IMU variants responded; verify board revision and I2C signal integrity"
        );
    }

    #[test]
    fn reports_external_bme280_address() {
        let mut summary = summary();
        summary.bme280_secondary_found = true;

        assert_eq!(summary.external_bme280_status(), "0x77");
    }
}
