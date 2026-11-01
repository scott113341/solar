use chrono::{DateTime, Duration, NaiveDate, Utc};
use circadia::{time_of_event, GlobalPosition, SunEvent};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct IpLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub city: String,
    pub region: String,
}

/// Look up the machine's approximate latitude/longitude via IP geolocation
pub fn geolocate() -> Result<IpLocation, Box<dyn std::error::Error>> {
    let loc: IpLocation = ureq::get("https://ipapi.co/json/")
        .call()?
        .into_body()
        .read_json()?;
    Ok(loc)
}

pub fn fmt_hr_min(duration: Duration) -> String {
    let duration_min = duration.num_minutes();

    if duration_min >= 60 {
        format!("{}h {}m", duration_min / 60, duration_min % 60)
    } else {
        format!("{}m", duration_min)
    }
}

pub fn fmt_min_sec(duration: Duration) -> String {
    let min = duration.num_minutes();
    let sec = duration.num_seconds();

    if min.abs() >= 1 {
        format!("{:+}m {}s", min, sec.abs() % 60)
    } else {
        format!("{:+}s", sec)
    }
}

pub fn sunrise_sunset(day: NaiveDate, position: &GlobalPosition) -> (DateTime<Utc>, DateTime<Utc>) {
    let sunrise = event_time(day, position, SunEvent::SUNRISE);
    let sunset = event_time(day, position, SunEvent::SUNSET);
    (sunrise, sunset)
}

/// How much earlier or later an event happened than it did the previous day.
pub fn day_over_day_change(today: DateTime<Utc>, yesterday: DateTime<Utc>) -> Duration {
    today - yesterday - Duration::days(1)
}

/// circadia 0.1's `time_of_event` still takes the deprecated `chrono::Date<Utc>`,
/// so this wrapper contains that one unavoidable deprecated call behind a
/// `NaiveDate` API.
#[allow(deprecated)]
fn event_time(day: NaiveDate, position: &GlobalPosition, event: SunEvent) -> DateTime<Utc> {
    let date = chrono::Date::<Utc>::from_utc(day, Utc);
    time_of_event(date, position, event).expect("sun event has no time at this location/date")
}

pub fn pct_year_progress(today: NaiveDate, position: &GlobalPosition) -> (Duration, Duration, f64) {
    // Find min/max day durations within the next year
    let mut date = today;
    let mut min = Duration::MAX;
    let mut max = Duration::MIN;
    for _ in 1..=366 {
        let (sunrise, sunset) = sunrise_sunset(date, position);
        let duration = sunset - sunrise;
        if duration < min {
            min = duration;
        }
        if duration > max {
            max = duration;
        }
        date = date.succ_opt().expect("date out of range");
    }

    // Find the "progress" between the day min/max. For example, assume the min day duration is 10
    // hours, the max is 14 hours, and today is is currently 11 hours long. The max variance over
    // the year is 4 hours (14-10), and we're 1 hour away from the minimum, or at "25% progress".
    //   10h {██      } 14h
    //         ^ 11h
    let current_day_length = {
        let (today_sunrise, today_sunset) = sunrise_sunset(today, position);
        today_sunset - today_sunrise
    };
    let maximum_delta = max - min;
    let current_delta = current_day_length - min;
    let progress = current_delta.num_seconds() as f64 / maximum_delta.num_seconds() as f64;

    (min, max, progress)
}

pub fn pct_progress_bar(progress: f64, progress_bar_length: usize) -> String {
    let mut progress_bar = String::new();

    // Add any full blocks to the beginning
    let full_blocks = progress * progress_bar_length as f64;
    for _ in 0..full_blocks.floor() as usize {
        progress_bar.push('█');
    }

    // Add a partial block if necessary
    let partial_block = full_blocks - full_blocks.floor();
    if let Some(partial_block) = match partial_block * 100.0 {
        p if (0.0..6.25).contains(&p) => None,          // ~0
        p if (6.25..18.75).contains(&p) => Some('▏'),   // 1/8
        p if (18.75..31.25).contains(&p) => Some('▎'),  // 1/4
        p if (31.25..43.75).contains(&p) => Some('▍'),  // 3/8
        p if (43.75..56.25).contains(&p) => Some('▌'),  // 1/2
        p if (56.25..68.75).contains(&p) => Some('▋'),  // 5/8
        p if (68.75..81.25).contains(&p) => Some('▊'),  // 3/8
        p if (81.25..93.75).contains(&p) => Some('▉'),  // 7/8
        p if (93.75..=100.0).contains(&p) => Some('█'), // ~1
        _ => None,
    } {
        progress_bar.push(partial_block);
    }

    // Pad right side with spaces
    while progress_bar.chars().count() < progress_bar_length {
        progress_bar.push(' ');
    }

    progress_bar
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    struct Place {
        name: &'static str,
        lat: f64,
        lng: f64,
        min_daylight_hours: f64,
        max_daylight_hours: f64,
    }

    impl Place {
        fn position(&self) -> GlobalPosition {
            GlobalPosition::at(self.lat, self.lng)
        }
    }

    // Chosen for how their sun events fall relative to 00:00 UTC, and for a spread of hemispheres
    // and latitudes. Sunset crosses 00:00 UTC in Longmont and Reykjavik, and sunrise does in Dhaka.
    // Daylight limits come from the NOAA solar equations over 2022-2026, widened by about 6 minutes.
    #[rustfmt::skip]
    const PLACES: [Place; 8] = [
        Place { name: "Longmont",  lat: 40.16722, lng: -105.101859, min_daylight_hours: 9.2,  max_daylight_hours: 15.15 },
        Place { name: "Dhaka",     lat: 23.8103,  lng: 90.4125,     min_daylight_hours: 10.55, max_daylight_hours: 13.7 },
        Place { name: "Sydney",    lat: -33.8688, lng: 151.2093,    min_daylight_hours: 9.8,  max_daylight_hours: 14.5 },
        Place { name: "Reykjavik", lat: 64.1466,  lng: -21.9426,    min_daylight_hours: 4.0,  max_daylight_hours: 21.25 },
        Place { name: "Quito",     lat: -0.1807,  lng: -78.4678,    min_daylight_hours: 12.0, max_daylight_hours: 12.25 },
        Place { name: "London",    lat: 51.5074,  lng: -0.1278,     min_daylight_hours: 7.7,  max_daylight_hours: 16.75 },
        Place { name: "Honolulu",  lat: 21.3069,  lng: -157.8583,   min_daylight_hours: 10.7, max_daylight_hours: 13.55 },
        Place { name: "Suva",      lat: -18.1416, lng: 178.4419,    min_daylight_hours: 10.9, max_daylight_hours: 13.35 },
    ];

    fn date(value: &str) -> NaiveDate {
        value.parse().unwrap()
    }

    /// Every day from 2022 through 2026, which includes the 2024 leap year.
    fn test_days() -> impl Iterator<Item = NaiveDate> {
        date("2022-01-01")
            .iter_days()
            .take_while(|day| day.year() <= 2026)
    }

    /// Fail with a count and a sample, since a broken calculation can affect many days.
    fn assert_no_violations(violations: Vec<String>) {
        assert!(
            violations.is_empty(),
            "{} violations, starting with:\n{}",
            violations.len(),
            violations[..violations.len().min(10)].join("\n"),
        );
    }

    #[test]
    fn test_sun_events_change_gradually_from_day_to_day() {
        let limit = Duration::minutes(5);
        let mut violations = vec![];

        for place in &PLACES {
            let position = place.position();
            for day in test_days() {
                let (sunrise, sunset) = sunrise_sunset(day, &position);
                let (yest_sunrise, yest_sunset) =
                    sunrise_sunset(day.pred_opt().unwrap(), &position);

                for (event, change) in [
                    ("sunrise", day_over_day_change(sunrise, yest_sunrise)),
                    ("sunset", day_over_day_change(sunset, yest_sunset)),
                ] {
                    if change.abs() > limit {
                        violations.push(format!("{} {day} {event}: {change}", place.name));
                    }
                }
            }
        }

        assert_no_violations(violations);
    }

    #[test]
    fn test_day_over_day_change_across_midnight_utc() {
        let position = PLACES[0].position();

        // Sunset moves from just after 00:00 UTC to just before it...
        let (_, sunset) = sunrise_sunset(date("2026-10-31"), &position);
        let (_, yest_sunset) = sunrise_sunset(date("2026-10-30"), &position);
        assert_eq!(
            day_over_day_change(sunset, yest_sunset),
            Duration::seconds(-73)
        );

        // ...and back again
        let (_, sunset) = sunrise_sunset(date("2027-01-16"), &position);
        let (_, yest_sunset) = sunrise_sunset(date("2027-01-15"), &position);
        assert_eq!(
            day_over_day_change(sunset, yest_sunset),
            Duration::seconds(66)
        );
    }

    #[test]
    fn test_daylight_stays_within_expected_range() {
        let mut violations = vec![];

        for place in &PLACES {
            let position = place.position();
            for day in test_days() {
                let (sunrise, sunset) = sunrise_sunset(day, &position);
                let hours = (sunset - sunrise).num_seconds() as f64 / 3600.0;

                if hours < place.min_daylight_hours || hours > place.max_daylight_hours {
                    violations.push(format!("{} {day}: {hours:.2}h", place.name));
                }
            }
        }

        assert_no_violations(violations);
    }

    #[test]
    fn test_sun_events_fall_on_the_requested_solar_day() {
        let mut violations = vec![];

        for place in &PLACES {
            let position = place.position();
            let solar_offset = Duration::seconds((place.lng * 240.0) as i64);

            for day in test_days() {
                let (sunrise, sunset) = sunrise_sunset(day, &position);
                let (solar_sunrise, solar_sunset) = (sunrise + solar_offset, sunset + solar_offset);

                if solar_sunrise.date_naive() != day || solar_sunrise.hour() >= 12 {
                    violations.push(format!("{} {day} sunrise: {solar_sunrise}", place.name));
                }
                if solar_sunset.date_naive() != day || solar_sunset.hour() < 12 {
                    violations.push(format!("{} {day} sunset: {solar_sunset}", place.name));
                }
            }
        }

        assert_no_violations(violations);
    }

    #[test]
    fn test_sun_events_match_noaa_reference_times() {
        // Smooth, plausible output can still be wrong, so compare with independent values. These
        // UTC times were calculated with the equations from NOAA's solar calculator.
        #[rustfmt::skip]
        let references = [
            (0, "2024-03-20", "2024-03-20T13:02:46Z", "2024-03-21T01:13:07Z"),
            (0, "2024-06-20", "2024-06-20T11:31:07Z", "2024-06-21T02:33:14Z"),
            (0, "2024-12-21", "2024-12-21T14:19:33Z", "2024-12-21T23:38:11Z"),
            (1, "2024-03-20", "2024-03-20T00:02:14Z", "2024-03-20T12:09:32Z"),
            (1, "2024-06-20", "2024-06-19T23:11:46Z", "2024-06-20T12:48:13Z"),
            (2, "2024-12-21", "2024-12-20T18:40:52Z", "2024-12-21T09:05:40Z"),
            (3, "2024-06-20", "2024-06-20T02:55:03Z", "2024-06-21T00:03:58Z"),
            (3, "2024-12-21", "2024-12-21T11:22:36Z", "2024-12-21T15:29:38Z"),
            (4, "2024-03-20", "2024-03-20T11:17:50Z", "2024-03-20T23:24:20Z"),
            (5, "2024-06-20", "2024-06-20T03:42:58Z", "2024-06-20T20:21:28Z"),
            (6, "2024-12-21", "2024-12-21T17:04:50Z", "2024-12-22T03:55:05Z"),
            (7, "2024-06-20", "2024-06-19T18:36:35Z", "2024-06-20T05:39:04Z"),
        ];
        let tolerance = Duration::minutes(2);
        let mut violations = vec![];

        for (place_index, day, expected_sunrise, expected_sunset) in references {
            let place = &PLACES[place_index];
            let (sunrise, sunset) = sunrise_sunset(date(day), &place.position());

            for (event, actual, expected) in [
                ("sunrise", sunrise, expected_sunrise),
                ("sunset", sunset, expected_sunset),
            ] {
                let expected = expected.parse::<DateTime<Utc>>().unwrap();
                if (actual - expected).abs() > tolerance {
                    violations.push(format!(
                        "{} {day} {event}: {actual}, not {expected}",
                        place.name
                    ));
                }
            }
        }

        assert_no_violations(violations);
    }

    #[test]
    fn test_pct_progress_bar() {
        // 0%
        assert_eq!(pct_progress_bar(0.0, 1), " ");
        assert_eq!(pct_progress_bar(0.0, 4), "    ");
        assert_eq!(pct_progress_bar(0.0, 8), "        ");
        assert_eq!(pct_progress_bar(0.0, 10), "          ");

        // 100%
        assert_eq!(pct_progress_bar(1.0, 1), "█");
        assert_eq!(pct_progress_bar(1.0, 4), "████");
        assert_eq!(pct_progress_bar(1.0, 8), "████████");
        assert_eq!(pct_progress_bar(1.0, 10), "██████████");

        // 50%
        assert_eq!(pct_progress_bar(0.5, 1), "▌");
        assert_eq!(pct_progress_bar(0.5, 4), "██  ");
        assert_eq!(pct_progress_bar(0.5, 8), "████    ");
        assert_eq!(pct_progress_bar(0.5, 10), "█████     ");

        // 25%
        assert_eq!(pct_progress_bar(0.25, 1), "▎");
        assert_eq!(pct_progress_bar(0.25, 2), "▌ ");
        assert_eq!(pct_progress_bar(0.25, 3), "▊  ");
        assert_eq!(pct_progress_bar(0.25, 4), "█   ");
        assert_eq!(pct_progress_bar(0.25, 8), "██      ");
        assert_eq!(pct_progress_bar(0.25, 10), "██▌       ");

        // Close to 0%
        assert_eq!(pct_progress_bar(0.0001, 10), "          ");
        assert_eq!(pct_progress_bar(0.001, 10), "          ");
        assert_eq!(pct_progress_bar(0.00624, 10), "          ");
        assert_eq!(pct_progress_bar(0.00625, 10), "▏         ");
        assert_eq!(pct_progress_bar(0.01, 10), "▏         ");
        assert_eq!(pct_progress_bar(0.1, 10), "█         ");

        // Close to 100%
        assert_eq!(pct_progress_bar(0.9, 10), "█████████ ");
        assert_eq!(pct_progress_bar(0.99, 10), "█████████▉");
        assert_eq!(pct_progress_bar(0.999, 10), "██████████");
        assert_eq!(pct_progress_bar(0.9999, 10), "██████████");
    }
}
