use chrono::{Date, DateTime, Duration, Utc};
use circadia::{time_of_event, GlobalPosition, SunEvent};

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

pub fn sunrise_sunset(day: Date<Utc>, position: &GlobalPosition) -> (DateTime<Utc>, DateTime<Utc>) {
    let sunrise = time_of_event(day, &position, SunEvent::SUNRISE).unwrap();
    let sunset = time_of_event(day, &position, SunEvent::SUNSET).unwrap();
    (sunrise, sunset)
}

pub fn pct_year_progress(today: Date<Utc>, position: &GlobalPosition) -> (Duration, Duration, f64) {
    // Find min/max day durations within the next year
    let mut date = today;
    let mut min = Duration::max_value();
    let mut max = Duration::min_value();
    for _ in 1..=366 {
        let (sunrise, sunset) = sunrise_sunset(date, &position);
        let duration = sunset - sunrise;
        if duration < min {
            min = duration;
        }
        if duration > max {
            max = duration;
        }
        date = date.succ();
    }

    // Find the "progress" between the day min/max. For example, assume the min day duration is 10
    // hours, the max is 14 hours, and today is is currently 11 hours long. The max variance over
    // the year is 4 hours (14-10), and we're 1 hour away from the minimum, or at "25% progress".
    //   10h {██      } 14h
    //         ^ 11h
    let current_day_length = {
        let (today_sunrise, today_sunset) = sunrise_sunset(today, &position);
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
        p if (0.0..6.25).contains(&p) => None,            // ~0
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
