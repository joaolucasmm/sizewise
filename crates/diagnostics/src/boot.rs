//! How long recent startups took, from Windows' boot performance log
//! (`Microsoft-Windows-Diagnostics-Performance/Operational`, event 100).
//! Windows only lets administrators read this log.

use windows::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_TIMEOUT};
use windows::Win32::System::EventLog::{
    EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection, EvtRender,
    EvtRenderEventXml, EVT_HANDLE,
};

use crate::wide::{from_wide, Wide};
use crate::Unavailable;

const CHANNEL: &str = "Microsoft-Windows-Diagnostics-Performance/Operational";

/// One startup as Windows measured it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupDuration {
    /// When Windows recorded it, in milliseconds since the Unix epoch.
    pub recorded_at_ms: i64,
    /// From power-on of Windows until the desktop was usable.
    pub boot_ms: u64,
    /// The part until the desktop appeared.
    pub main_path_ms: u64,
    /// The part after the desktop appeared, while startup programs loaded.
    pub post_boot_ms: u64,
}

/// The most recent startups, newest first, at most `limit`.
pub fn startup_durations(limit: usize) -> Result<Vec<StartupDuration>, Unavailable> {
    let channel = Wide::new(CHANNEL);
    let query = Wide::new("*[System[(EventID=100)]]");
    // SAFETY: strings are null-terminated; the handle is closed below.
    let results = unsafe {
        EvtQuery(
            None,
            channel.as_pcwstr(),
            query.as_pcwstr(),
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        )
    }?;
    let results = Handle(results);

    let mut durations = Vec::new();
    while durations.len() < limit {
        let mut events = [0isize; 16];
        let mut returned = 0u32;
        // SAFETY: `events` has room for the count passed.
        if let Err(error) = unsafe { EvtNext(results.0, &mut events, 1_000, 0, &mut returned) } {
            let code = error.code();
            if code == ERROR_NO_MORE_ITEMS.to_hresult() || code == ERROR_TIMEOUT.to_hresult() {
                break;
            }
            return Err(error.into());
        }
        for &event in &events[..returned as usize] {
            let event = Handle(EVT_HANDLE(event));
            if let Some(duration) = render_xml(&event).as_deref().and_then(parse) {
                durations.push(duration);
            }
        }
    }
    durations.truncate(limit);
    Ok(durations)
}

struct Handle(EVT_HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: each handle is closed once.
        let _ = unsafe { EvtClose(self.0) };
    }
}

fn render_xml(event: &Handle) -> Option<String> {
    let (mut used, mut properties) = (0u32, 0u32);
    // SAFETY: a first call with no buffer reports the size needed.
    let _ = unsafe {
        EvtRender(
            None,
            event.0,
            EvtRenderEventXml.0,
            0,
            None,
            &mut used,
            &mut properties,
        )
    };
    if used == 0 {
        return None;
    }
    let mut buffer = vec![0u16; (used as usize).div_ceil(2)];
    // SAFETY: the buffer has the size reported above, in bytes.
    unsafe {
        EvtRender(
            None,
            event.0,
            EvtRenderEventXml.0,
            (buffer.len() * 2) as u32,
            Some(buffer.as_mut_ptr().cast()),
            &mut used,
            &mut properties,
        )
    }
    .ok()?;
    Some(from_wide(&buffer))
}

fn parse(xml: &str) -> Option<StartupDuration> {
    Some(StartupDuration {
        recorded_at_ms: parse_utc(attribute(xml, "TimeCreated", "SystemTime")?)?,
        boot_ms: data(xml, "BootTime")?,
        main_path_ms: data(xml, "MainPathBootTime").unwrap_or(0),
        post_boot_ms: data(xml, "BootPostBootTime").unwrap_or(0),
    })
}

/// The value of `<Data Name='name'>value</Data>`.
fn data(xml: &str, name: &str) -> Option<u64> {
    let start = [
        format!("<Data Name='{name}'>"),
        format!("<Data Name=\"{name}\">"),
    ]
    .iter()
    .find_map(|tag| xml.find(tag.as_str()).map(|i| i + tag.len()))?;
    let end = start + xml[start..].find('<')?;
    xml[start..end].trim().parse().ok()
}

/// The value of `attribute` on the first `<element ...>`.
fn attribute<'a>(xml: &'a str, element: &str, attribute: &str) -> Option<&'a str> {
    let tag_start = xml.find(&format!("<{element} "))?;
    let tag = &xml[tag_start..tag_start + xml[tag_start..].find('>')?];
    let value_start = tag.find(&format!("{attribute}="))? + attribute.len() + 2;
    let quote = tag.as_bytes()[value_start - 1] as char;
    let value_end = value_start + tag[value_start..].find(quote)?;
    Some(&tag[value_start..value_end])
}

/// Milliseconds since the Unix epoch of a UTC time such as
/// `2026-10-22T08:15:42.1234567Z`.
fn parse_utc(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let millis = text
        .get(20..23)
        .and_then(|m| m.parse::<i64>().ok())
        .unwrap_or(0);

    // Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_event_100() {
        let xml =
            "<Event><System><TimeCreated SystemTime='2026-10-22T08:15:42.1234567Z'/></System>\
                   <EventData><Data Name='BootTime'>41234</Data>\
                   <Data Name='MainPathBootTime'>20000</Data>\
                   <Data Name='BootPostBootTime'>21234</Data></EventData></Event>";
        let parsed = parse(xml).unwrap();
        assert_eq!(parsed.boot_ms, 41_234);
        assert_eq!(parsed.main_path_ms, 20_000);
        assert_eq!(parsed.post_boot_ms, 21_234);
        assert_eq!(parsed.recorded_at_ms, 1_792_656_942_123);
    }

    #[test]
    fn unix_epoch_is_zero() {
        assert_eq!(parse_utc("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_utc("2000-03-01T00:00:00Z"), Some(951_868_800_000));
    }
}
