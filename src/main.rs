mod helpers;

use crate::helpers::*;
use bitbar::{ContentItem, Menu, MenuItem};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use circadia::GlobalPosition;
use std::env::VarError;
use std::fmt::Display;

#[bitbar::main]
fn main() -> Menu {
    build_menu(current_time(), &resolve_location())
}

fn build_menu<Tz: TimeZone>(now: DateTime<Tz>, location: &Location) -> Menu
where
    Tz::Offset: Display,
{
    let position = GlobalPosition::at(location.lat, location.lng);
    let tz = now.timezone();

    let today = now.date_naive();
    let (sunrise, sunset) = sunrise_sunset(today, &position);

    let yesterday = today.pred_opt().expect("date out of range");
    let (yest_sunrise, yest_sunset) = sunrise_sunset(yesterday, &position);

    Menu(vec![
        item_daytime_short(sunrise, sunset),
        MenuItem::Sep,
        item_daytime_long(sunrise, sunset, yest_sunrise, yest_sunset),
        item_sunrise(sunrise, yest_sunrise, &tz),
        item_sunset(sunset, yest_sunset, &tz),
        item_location(location),
        MenuItem::Sep,
        item_year_progress_bar(today, &position),
        item_year_progress_pct(today, &position),
    ])
}

/// The current local time, which `SOLAR_NOW` (an RFC 3339 timestamp) can override for testing.
fn current_time() -> DateTime<Local> {
    time_from_override(std::env::var("SOLAR_NOW"))
}

fn time_from_override(value: Result<String, VarError>) -> DateTime<Local> {
    match value {
        Ok(value) => DateTime::parse_from_rfc3339(&value)
            .unwrap_or_else(|err| panic!("Invalid SOLAR_NOW {value:?}: {err}"))
            .with_timezone(&Local),
        Err(VarError::NotPresent) => Local::now(),
        Err(err) => panic!("Invalid SOLAR_NOW: {err}"),
    }
}

/// A resolved location, along with a human-readable place name when known.
struct Location {
    lat: f64,
    lng: f64,
    place: Option<String>,
}

/// Resolve the location to use for solar calculations.
///
/// If both `SOLAR_LATITUDE` and `SOLAR_LONGITUDE` are set they take precedence;
/// otherwise the location is looked up from the machine's public IP address.
fn resolve_location() -> Location {
    match (
        std::env::var("SOLAR_LATITUDE"),
        std::env::var("SOLAR_LONGITUDE"),
    ) {
        (Ok(lat), Ok(lng)) => Location {
            lat: lat.parse().expect("Invalid SOLAR_LATITUDE"),
            lng: lng.parse().expect("Invalid SOLAR_LONGITUDE"),
            place: None,
        },
        _ => {
            let loc = geolocate().expect("Could not look up location from IP");
            Location {
                lat: loc.latitude,
                lng: loc.longitude,
                place: Some(format!("{}, {}", loc.city, loc.region)),
            }
        }
    }
}

fn item_daytime_short(sunrise: DateTime<Utc>, sunset: DateTime<Utc>) -> MenuItem {
    MenuItem::new(fmt_hr_min(sunset - sunrise).to_string())
}

fn item_location(location: &Location) -> MenuItem {
    // Fall back to coordinates when there's no place name (env-var override).
    let label = location
        .place
        .clone()
        .unwrap_or_else(|| format!("{:.4}, {:.4}", location.lat, location.lng));

    MenuItem::Content(
        ContentItem::new(label)
            .color("black")
            .unwrap()
            .href(format!(
                "https://www.timeanddate.com/sun/@{},{}",
                location.lat, location.lng,
            ))
            .unwrap(),
    )
}

fn item_sunrise<Tz: TimeZone>(
    sunrise: DateTime<Utc>,
    yest_sunrise: DateTime<Utc>,
    tz: &Tz,
) -> MenuItem
where
    Tz::Offset: Display,
{
    MenuItem::Content(
        ContentItem::new(format!(
            "Sunrise: {} ({})",
            sunrise.with_timezone(tz).format("%H:%M"),
            fmt_min_sec(day_over_day_change(sunrise, yest_sunrise)),
        ))
        .color("black")
        .unwrap(),
    )
}

fn item_sunset<Tz: TimeZone>(sunset: DateTime<Utc>, yest_sunset: DateTime<Utc>, tz: &Tz) -> MenuItem
where
    Tz::Offset: Display,
{
    MenuItem::Content(
        ContentItem::new(format!(
            "Sunset: {} ({})",
            sunset.with_timezone(tz).format("%H:%M"),
            fmt_min_sec(day_over_day_change(sunset, yest_sunset)),
        ))
        .color("black")
        .unwrap(),
    )
}

fn item_daytime_long(
    sunrise: DateTime<Utc>,
    sunset: DateTime<Utc>,
    yest_sunrise: DateTime<Utc>,
    yest_sunset: DateTime<Utc>,
) -> MenuItem {
    let today_duration = sunset - sunrise;
    let yest_duration = yest_sunset - yest_sunrise;

    MenuItem::Content(
        ContentItem::new(format!(
            "Daytime: {} ({})",
            fmt_hr_min(sunset - sunrise),
            fmt_min_sec(today_duration - yest_duration),
        ))
        .color("black")
        .unwrap(),
    )
}

fn item_year_progress_bar(today: NaiveDate, position: &GlobalPosition) -> MenuItem {
    // Get progress bar
    let (min, max, progress) = pct_year_progress(today, position);
    let progress_bar = pct_progress_bar(progress, 20);

    MenuItem::Content(
        ContentItem::new(format!(
            "{} [{}] {}",
            fmt_hr_min(min),
            progress_bar,
            fmt_hr_min(max),
        ))
        // Menlo ships with macOS and includes the block-element glyphs; a font
        // that isn't installed falls back to a proportional one and mangles the bar.
        .font("Menlo")
        .size(14)
        .color("black")
        .unwrap()
        .href(format!(
            "https://www.timeanddate.com/sun/@{},{}",
            position.lat(),
            position.lng(),
        ))
        .unwrap(),
    )
}

fn item_year_progress_pct(today: NaiveDate, position: &GlobalPosition) -> MenuItem {
    let (_min, _max, progress) = pct_year_progress(today, position);

    MenuItem::Content(
        ContentItem::new(format!("Progress: {:.1}%", progress * 100.0))
            .color("black")
            .unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::Denver;

    const LONGMONT: Location = Location {
        lat: 40.16722,
        lng: -105.101859,
        place: None,
    };

    fn menu_at<Tz: TimeZone>(now: &str, tz: Tz) -> String
    where
        Tz::Offset: Display,
    {
        let now = DateTime::parse_from_rfc3339(now).unwrap();
        build_menu(now.with_timezone(&tz), &LONGMONT).to_string()
    }

    #[track_caller]
    fn assert_menu_contains(menu: &str, expected: &[&str]) {
        for line in expected {
            assert!(menu.contains(line), "expected {line:?} in:\n{menu}");
        }
    }

    #[test]
    fn test_clock_times_move_back_when_dst_ends() {
        // Oct 31 is also when sunset crosses 00:00 UTC, so this checks that the menu handles that
        let before = menu_at("2026-10-31T12:00:00-06:00", Denver);
        assert_menu_contains(
            &before,
            &["Sunrise: 07:28 (+1m 9s)", "Sunset: 17:59 (-1m 13s)"],
        );

        let after = menu_at("2026-11-01T12:00:00-07:00", Denver);
        assert_menu_contains(
            &after,
            &["Sunrise: 06:29 (+1m 8s)", "Sunset: 16:57 (-1m 12s)"],
        );
    }

    #[test]
    fn test_clock_times_move_forward_when_dst_starts() {
        let before = menu_at("2027-03-13T12:00:00-07:00", Denver);
        assert_menu_contains(
            &before,
            &["Sunrise: 06:15 (-1m 36s)", "Sunset: 18:05 (+1m 3s)"],
        );

        let after = menu_at("2027-03-14T12:00:00-06:00", Denver);
        assert_menu_contains(
            &after,
            &["Sunrise: 07:13 (-1m 37s)", "Sunset: 19:06 (+1m 4s)"],
        );
    }

    #[test]
    fn test_time_zone_determines_the_date_and_displayed_times() {
        // This moment is still Oct 31 in Denver, but already Nov 1 in UTC
        let now = "2026-10-31T20:00:00-06:00";
        assert_menu_contains(&menu_at(now, Denver), &["Sunrise: 07:28 "]);
        assert_menu_contains(&menu_at(now, Utc), &["Sunrise: 13:29 "]);
    }

    #[test]
    fn test_override_sets_the_current_time() {
        let now = time_from_override(Ok("2026-11-01T12:00:00-07:00".to_string()));
        assert_eq!(
            now.with_timezone(&Utc).to_rfc3339(),
            "2026-11-01T19:00:00+00:00"
        );
    }

    #[test]
    fn test_no_override_uses_the_real_time() {
        let now = time_from_override(Err(VarError::NotPresent));
        assert!((Local::now() - now).num_seconds().abs() < 60);
    }

    #[test]
    #[should_panic(expected = "Invalid SOLAR_NOW")]
    fn test_invalid_override_panics_instead_of_using_the_real_time() {
        time_from_override(Ok("2026-11-01".to_string()));
    }
}
