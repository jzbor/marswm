
#[macro_export]
macro_rules! is_floating {
    ($wm:expr, $client_rc:expr) => {
        $client_rc.borrow().attributes().is_floating
            || $wm.get_workspace($client_rc)
            .map(|ws| ws.current_layout() == LayoutType::Floating)
            .unwrap_or(false)
    }
}
