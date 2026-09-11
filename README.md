# solar

Plugin for [xbar](https://xbarapp.com) that shows information about the solar cycle.

By default, it looks up your approximate location from your public IP address. To skip this and 
set it to a specific location, set both environment variables:

`SOLAR_LATITUDE=40.1 SOLAR_LONGITUDE=-105.2 solar`

<img height="250" src="screenshot.png" alt="Screenshot of how Xbar displays the plugin" />

"Progress" shows how close/far today is from the shortest and longest upcoming days. Nice when I'm really feeling how
short the days are getting, but the minimum is in sight and not _that_ much shorter.
