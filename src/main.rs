mod helpers;

use crate::helpers::*;
use bitbar::{ContentItem, Menu, MenuItem};
use chrono::{Date, DateTime, Local, Utc};
use circadia::GlobalPosition;

#[bitbar::main]
fn main() -> Menu {
    let lat: f64 = std::env::var("SOLAR_LATITUDE")
        .expect("You must set a SOLAR_LATITUDE environment variable")
        .parse()
        .expect("Invalid latitude");
    let lng: f64 = std::env::var("SOLAR_LONGITUDE")
        .expect("You must set a SOLAR_LONGITUDE environment variable")
        .parse()
        .expect("Invalid longitude");

    let position = GlobalPosition::at(lat, lng);

    let today = Local::now().date().with_timezone(&Utc);
    let (sunrise, sunset) = sunrise_sunset(today, &position);

    let yesterday = today.pred();
    let (yest_sunrise, yest_sunset) = sunrise_sunset(yesterday, &position);

    Menu(vec![
        item_daytime_short(sunrise, sunset),
        MenuItem::Sep,
        item_daytime_long(sunrise, sunset, yest_sunrise, yest_sunset),
        item_sunrise(sunrise, yest_sunrise),
        item_sunset(sunset, yest_sunset),
        MenuItem::Sep,
        item_year_progress_bar(today, &position),
        item_year_progress_pct(today, &position),
    ])
}

fn item_daytime_short(sunrise: DateTime<Utc>, sunset: DateTime<Utc>) -> MenuItem {
    MenuItem::new(format!("{}", fmt_hr_min(sunset - sunrise)))
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

fn item_year_progress_bar(today: Date<Utc>, position: &GlobalPosition) -> MenuItem {
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
        .font("Source Code Pro")
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

fn item_year_progress_pct(today: Date<Utc>, position: &GlobalPosition) -> MenuItem {
    let (_min, _max, progress) = pct_year_progress(today, position);

    MenuItem::Content(
        ContentItem::new(format!("Progress: {:.1}%", progress * 100.0))
            .color("black")
            .unwrap(),
    )
}
