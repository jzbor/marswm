use x11rb::connection::Connection;
use x11rb::protocol::randr;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::Screen;

use crate::common::Dimensions;
use crate::common::Dimensioned;
use crate::common::MonitorConfig;


pub fn query_monitor_config(conn: &impl Connection, screen: &Screen, ignore_overlapping: bool) -> Vec<MonitorConfig> {
    let mut monitors = Vec::new();

    // try xrandr
    if let Ok(mon_cookie) = randr::get_monitors(conn, screen.root, true) {
        if let Ok(reply) = mon_cookie.reply() {
            monitors = reply.monitors.into_iter().map(|m| {
                let name = match conn.get_atom_name(m.name) {
                    Ok(cookie) => cookie.reply().map(|x| x.name).unwrap_or("unknown".into()),
                    Err(_) => "unknown".into(),
                };
                let dimensions = Dimensions::new(m.x.into(), m.y.into(), m.width.into(), m.height.into());
                MonitorConfig::new(String::from_utf8_lossy(&name).to_string(), dimensions, dimensions)
            }).collect()
        }
    }

    // fall back to whole screen
    if monitors.is_empty() {
        let name = "output".to_owned();
        let dimensions = Dimensions::new(0, 0, screen.width_in_pixels.into(), screen.height_in_pixels.into());
        monitors = vec!(MonitorConfig::new(name, dimensions, dimensions))
    }

    if ignore_overlapping {
        let no_overlap = |m1: &MonitorConfig, m2: &MonitorConfig| {
            m1.dimensions().right() <= m2.dimensions().x()  // m1 is left of m2
            || m1.dimensions().x() >= m2.dimensions().right()  // m1 is right of m2
            || m1.dimensions().bottom() <= m2.dimensions().y()  // m1 is on top of m2
            || m1.dimensions().y() >= m2.dimensions().bottom()  // m1 is below m2
        };
        let mut non_overlapping = Vec::new();
        for mon in monitors.drain(..) {
            if non_overlapping.iter().all(|m| no_overlap(m, &mon)) {
                non_overlapping.push(mon);
            }
        }
        non_overlapping
    } else {
        monitors
    }
}
