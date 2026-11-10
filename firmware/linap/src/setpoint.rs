use core::fmt;

/// Setpoint in half degrees, as sent in the 7 data bits of the command board state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Setpoint(u8);

impl Setpoint {
    /// Range of the command board keys.
    pub const MIN_HALVES: u8 = 10;
    pub const MAX_HALVES: u8 = 60;

    pub const fn from_halves(halves: u8) -> Option<Self> {
        if halves >= Self::MIN_HALVES && halves <= Self::MAX_HALVES { Some(Self(halves)) } else { None }
    }

    pub const fn halves(self) -> u8 {
        self.0
    }

    /// "14.5", "15", "15.0" (Home Assistant number), rounded to the nearest half degree.
    pub fn parse(text: &str) -> Option<Self> {
        Self::from_halves(parse_halves(text)?)
    }
}

/// A decimal number of halves ("14.5" -> 29), rounded to the nearest half.
pub fn parse_halves(text: &str) -> Option<u8> {
    let (whole, fraction) = text.trim().split_once('.').unwrap_or((text.trim(), "0"));
    let whole: u8 = whole.parse().ok()?;
    let tenths = fraction.bytes().next().filter(u8::is_ascii_digit).map_or(0, |digit| digit - b'0');
    let rounding = match tenths {
        0..=2 => 0,
        3..=7 => 1,
        _ => 2,
    };
    whole.checked_mul(2)?.checked_add(rounding)
}

impl fmt::Display for Setpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0 / 2, if self.0 % 2 == 1 { 5 } else { 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_home_assistant_numbers() {
        assert_eq!(Setpoint::parse("14.5").map(Setpoint::halves), Some(29));
        assert_eq!(Setpoint::parse("15").map(Setpoint::halves), Some(30));
        assert_eq!(Setpoint::parse(" 20.0 ").map(Setpoint::halves), Some(40));
        assert_eq!(Setpoint::parse("19.8").map(Setpoint::halves), Some(40));
    }

    #[test]
    fn rejects_out_of_range_or_garbage() {
        assert_eq!(Setpoint::parse("4.5"), None);
        assert_eq!(Setpoint::parse("30.5"), None);
        assert_eq!(Setpoint::parse("hot"), None);
        assert_eq!(Setpoint::parse("200"), None);
    }

    #[test]
    fn parses_any_number_of_halves() {
        assert_eq!(parse_halves("0"), Some(0));
        assert_eq!(parse_halves("2.5"), Some(5));
        assert_eq!(parse_halves("-1"), None);
    }

    #[test]
    fn prints_with_one_decimal() {
        assert_eq!(Setpoint::from_halves(29).unwrap().to_string(), "14.5");
        assert_eq!(Setpoint::from_halves(20).unwrap().to_string(), "10.0");
    }
}
