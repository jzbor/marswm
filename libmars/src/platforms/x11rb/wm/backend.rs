use std::marker::PhantomData;
use x11rb::connection::Connection;
use x11rb::connection::RequestConnection;
use x11rb::protocol::randr;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::x11_utils::ExtensionInformation;
use x11rb::COPY_DEPTH_FROM_PARENT;

use crate::platforms::x11::misc::atoms::X11Atom;
use crate::platforms::x11::misc::atoms::X11Atom::*;
use crate::common::error::Result;
use crate::common::MonitorConfig;
use crate::interfaces::wm::WindowManager;
use crate::platforms::x11rb::misc::monitors::*;
use crate::platforms::x11rb::misc::window::Window;


type WM<'a, C, A> = dyn WindowManager<X11RBBackend<C, A>, A> + 'a;

const SUPPORTED_ATOMS: &[X11Atom; 22] = & [
    NetActiveWindow,
    NetClientList,
    NetClientListStacking,
    NetCloseWindow,
    NetCurrentDesktop,
    NetDesktopNames,
    NetNumberOfDesktops,
    NetSupported,
    NetSupportingWMCheck,
    NetWMDesktop,
    NetWMName,
    NetWMState,
    NetWMStateFullscreen,
    NetWMWindowType,
    NetWMWindowTypeDesktop,
    NetWMWindowTypeDialog,
    NetWMWindowTypeDock,
    NetWMWindowTypeMenu,
    NetWMWindowTypeNotification,
    NetWorkarea,

    MarsCenter,
    MarsWMStateTiled,
];

pub struct X11RBBackend<C: Connection, A: PartialEq> {
    conn: C,
    attribute_phantom: PhantomData<A>,
    wmcheck_win: u32,
    root: u32,
    monitors: Vec<MonitorConfig>,
    xrandr_ext_information: Option<ExtensionInformation>,
}

impl<'a, A: PartialEq + Default> X11RBBackend<RustConnection, A> {
    pub fn init(name: &str) -> Result<Self> {
        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // export wm name
        let wmcheck_win = conn.generate_id()?;
        conn.create_window(COPY_DEPTH_FROM_PARENT, wmcheck_win, screen.root, 0, 0, 1, 1, 0,
            WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel))?;
        wmcheck_win.replace_property_long(&conn, X11Atom::NetSupportingWMCheck, X11Atom::XAWindow, &wmcheck_win.to_ne_bytes())?;
        wmcheck_win.replace_property_long(&conn, X11Atom::NetWMName, X11Atom::UTF8String, name.as_bytes())?;
        root.replace_property_long(&conn, X11Atom::NetSupportingWMCheck, X11Atom::XAWindow, &wmcheck_win.to_ne_bytes())?;

        // try to become the window manager
        let event_mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY | EventMask::STRUCTURE_NOTIFY | EventMask::KEY_PRESS | EventMask::BUTTON_PRESS;
        let change = ChangeWindowAttributesAux::default().event_mask(event_mask);
        let result = conn.change_window_attributes(screen.root, &change)?.check();
        // panic if another wm is running
        result.unwrap();

        let xrandr_ext_information = conn.extension_information("")?.map(|ei| {
            let _ = randr::select_input(&conn, root, randr::NotifyMask::CRTC_CHANGE);
            ei
        });

        Self::set_supported_atoms(&conn, root, SUPPORTED_ATOMS);
        let monitors = query_monitor_config(&conn, screen, true);

        Ok(X11RBBackend {
            conn,
            attribute_phantom: PhantomData::default(),
            wmcheck_win,
            root,
            monitors,
            xrandr_ext_information,
        })
    }
}

impl<C: Connection, A: PartialEq + Default> X11RBBackend<C, A> {
    fn set_supported_atoms(conn: &C, win: u32, supported_atoms: &[X11Atom]) {
        let atom_vec: Vec<u8> = (*supported_atoms).iter()
            .map(|a| conn.intern_atom(false, a.as_bytes())).flatten()
            .map(|c| c.reply()).flatten()
            .map(|c| c.atom.to_ne_bytes())
            .flatten().collect();
        let _ = win.replace_property_long(conn, X11Atom::NetSupported, X11Atom::XAAtom, &atom_vec);
    }
}
