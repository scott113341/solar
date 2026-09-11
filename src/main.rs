mod helpers;

use crate::helpers::*;
use bitbar::{ContentItem, Menu, MenuItem};
use chrono::{DateTime, Local, NaiveDate, Utc};
use circadia::GlobalPosition;

#[bitbar::main]
fn main() -> Menu {
    let location = resolve_location();
    let position = GlobalPosition::at(location.lat, location.lng);

    let today = Local::now().date_naive();
    let (sunrise, sunset) = sunrise_sunset(today, &position);

    let yesterday = today.pred_opt().expect("date out of range");
    let (yest_sunrise, yest_sunset) = sunrise_sunset(yesterday, &position);

    Menu(vec![
        item_daytime_short(sunrise, sunset),
        MenuItem::Sep,
        item_daytime_long(sunrise, sunset, yest_sunrise, yest_sunset),
        item_sunrise(sunrise, yest_sunrise),
        item_sunset(sunset, yest_sunset),
        item_location(&location),
        MenuItem::Sep,
        item_year_progress_bar(today, &position),
        item_year_progress_pct(today, &position),
    ])
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
            let loc = geolocate().expect(
                "Could not look up location from IP",
            );
            Location {
                lat: loc.latitude,
                lng: loc.longitude,
                place: Some(format!("{}, {}", loc.city, loc.region)),
            }
        }
    }
}

fn item_daytime_short(sunrise: DateTime<Utc>, sunset: DateTime<Utc>) -> MenuItem {
    MenuItem::new(format!("{}", fmt_hr_min(sunset - sunrise)))
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

fn item_sunrise(sunrise: DateTime<Utc>, yest_sunrise: DateTime<Utc>) -> MenuItem {
    MenuItem::Content(
        ContentItem::new(format!(
            "Sunrise: {} ({})",
            sunrise.with_timezone(&Local).format("%H:%M"),
            fmt_min_sec(sunrise.time() - yest_sunrise.time()),
        ))
        .color("black")
        .unwrap(),
    )
}

fn item_sunset(sunset: DateTime<Utc>, yest_sunset: DateTime<Utc>) -> MenuItem {
    MenuItem::Content(
        ContentItem::new(format!(
            "Sunset: {} ({})",
            sunset.with_timezone(&Local).format("%H:%M"),
            fmt_min_sec(sunset.time() - yest_sunset.time()),
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
