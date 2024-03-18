use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use x11rb::connection::Connection;
use x11rb::connection::RequestConnection;
use x11rb::protocol::randr;
use x11rb::protocol::xproto;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::x11_utils::ExtensionInformation;
use x11rb::COPY_DEPTH_FROM_PARENT;

use crate::platforms::x11::misc::atoms::X11Atom;
use crate::platforms::x11::misc::atoms::X11Atom::*;
use crate::common::error::Result;
use crate::common::*;
use crate::interfaces::wm::*;
use crate::platforms::x11rb::wm::client::X11RBClient;
use crate::platforms::x11rb::wm::unmanaged::*;
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
    conn: Rc<C>,
    attribute_phantom: PhantomData<A>,
    wmcheck_win: u32,
    screen_num: usize,
    monitors: Vec<MonitorConfig>,
    xrandr_ext_information: Option<ExtensionInformation>,
    unmanaged_clients: Vec<UnmanagedClient<C>>,
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
        wmcheck_win.replace_property_char(&conn, X11Atom::NetWMName, X11Atom::UTF8String, name.as_bytes())?;
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
        let monitors = query_monitor_config(&conn, &screen, true);

        Ok(X11RBBackend {
            conn: conn.into(),
            attribute_phantom: PhantomData::default(),
            wmcheck_win,
            screen_num,
            monitors,
            xrandr_ext_information,
            unmanaged_clients: Vec::new(),
        })
    }
}

impl<C: Connection, A: PartialEq + Default> X11RBBackend<C, A> {
    fn apply_dock_insets(&mut self) {
        self.monitors.iter_mut().for_each(|m| m.remove_insets());

        for dock in self.unmanaged_clients.iter().filter(|u| u.get_type() == UnmanagedType::Dock) {
            let dimensions = match dock.window().dimensions(&self.conn) {
                Ok(dimensions) => dimensions,
                Err(_) => continue,
            };

            if let Some(mon) = self.monitors.iter_mut().find(|m| m.contains_point(dimensions.center())) {
                // apply top indent
                if dimensions.center().1 < mon.dimensions().center().1 {
                    let inset = dimensions.bottom() - mon.dimensions().y();
                    mon.add_inset_top(inset as u32);
                } else {
                    let inset = mon.dimensions().bottom() - dimensions.y();
                    mon.add_inset_bottom(inset as u32);
                }
            }
        }
    }

    fn set_supported_atoms(conn: &C, win: u32, supported_atoms: &[X11Atom]) {
        let atom_vec: Vec<u8> = (*supported_atoms).iter()
            .map(|a| conn.intern_atom(false, a.as_bytes())).flatten()
            .map(|c| c.reply()).flatten()
            .map(|c| c.atom.to_ne_bytes())
            .flatten().collect();
        let _ = win.replace_property_long(conn, X11Atom::NetSupported, X11Atom::XAAtom, &atom_vec);
    }

    fn handle_event(&mut self, wm: &mut dyn WindowManager<Self, A>, event: Event) {
        if let Event::RandrNotify(_) = event {
            self.monitors = query_monitor_config(self.conn.deref(), self.get_screen(), true);
            self.apply_dock_insets();
            wm.update_monitor_config(self, self.monitors.clone())
        }

        use x11rb::protocol::Event::*;
        match event {
            ButtonPress(e) => self.on_button_press(wm, e),
            ClientMessage(_) => todo!(),
            ConfigureNotify(_) => todo!(),
            ConfigureRequest(_) => todo!(),
            DestroyNotify(_) => todo!(),
            EnterNotify(_) => todo!(),
            Expose(_) => todo!(),
            KeyPress(_) => todo!(),
            LeaveNotify(_) => todo!(),
            MapRequest(_) => todo!(),
            MapNotify(_) => todo!(),
            UnmapNotify(_) => todo!(),
            PropertyNotify(_) => todo!(),
            _ => (),
        }

        todo!();
    }

    fn get_screen(&self) -> &Screen {
        &self.conn.setup().roots[self.screen_num]
    }

    fn get_root(&self) -> u32 {
        self.get_screen().root
    }

    /// Create a new client for the window and give it to the window manager
    fn manage(&mut self, wm: &mut (impl WindowManager<Self, A> + ?Sized), window: u32) {
        todo!();
    }

    fn on_button_press(&mut self, wm: &mut WM<C, A>, event: xproto::ButtonPressEvent) {
        let modifiers = sanitize_modifiers(event.state);

        if let Some(client_rc) = Self::client_by_frame(wm, event.child) {
            wm.handle_button(self, modifiers.bits() as u32, event.detail as u32, ButtonTarget::Frame, Some(client_rc));
        } else if let Some(client_rc) = Self::client_by_window(wm, event.child) {
            wm.handle_button(self, modifiers.bits() as u32, event.detail as u32, ButtonTarget::Window, Some(client_rc));
        } else if event.child == self.get_root() {
            wm.handle_button(self, modifiers.bits() as u32, event.detail as u32, ButtonTarget::Root, None);
        }
    }

    fn client_by_frame(wm: &mut WM<C, A>, frame: u32) -> Option<Rc<RefCell<X11RBClient<A>>>> {
        wm.clients()
            .find(|c| c.borrow().frame() == frame || c.borrow().title_window() == Some(frame.into()))
            .cloned()
    }

    fn client_by_window(wm: &mut WM<C, A>, frame: u32) -> Option<Rc<RefCell<X11RBClient<A>>>> {
        wm.clients()
            .find(|c| c.borrow().window() == frame || c.borrow().title_window() == Some(frame.into()))
            .cloned()
    }
}

impl<A: PartialEq + Default, C: Connection> Backend<A> for X11RBBackend<C, A> {
    type Client = X11RBClient<A>;
    fn export_active_window(&self, client_option: &Option<Rc<RefCell<Self::Client>>>) {
        todo!();
    }

    fn export_client_list(&self, clients: Vec<&Rc<RefCell<Self::Client>>>, clients_stacked: Vec<&Rc<RefCell<Self::Client>>>) {
        todo!();
    }

    fn export_current_workspace(&self, workspace_idx: u32) {
        let ws = &workspace_idx.to_ne_bytes();
        let _ = self.get_root().replace_property_long(&self.conn, NetCurrentDesktop, XACardinal, ws);
    }

    fn export_workspaces(&self, workspaces: Vec<(String, Dimensions, Dimensions)>) {
        let root = self.get_root();
        let nworkspaces: u64 = workspaces.len().try_into().unwrap();
        let mut names = Vec::new();
        let mut workareas = Vec::new();

        for (name, _dimensions, workarea) in workspaces.iter() {
            names.push(name as &str);

            workareas.push(workarea.x() as i64 as u64);
            workareas.push(workarea.y() as i64 as u64);
            workareas.push(workarea.w() as u64);
            workareas.push(workarea.h() as u64);
        }

        // export number of workspaces
        let _ = root.replace_property_long(&self.conn, NetNumberOfDesktops, XACardinal, &nworkspaces.to_ne_bytes());

        // export workspace names
        let _ = root.replace_property_strings(&self.conn, NetDesktopNames, UTF8String, &names);

        // export workareas
        // TODO: remove property just as i3 does
        let data: Vec<_> = workareas.iter().flat_map(|i| i.to_ne_bytes()).collect();
        let _ = root.replace_property_long(&self.conn, NetWorkarea, XACardinal, &data);
    }

    fn get_monitor_config(&self) -> Vec<MonitorConfig> {
        self.monitors.clone()
    }

    fn handle_existing_windows(&mut self, wm: &mut dyn WindowManager<Self, A>) {
        xproto::grab_server(&self.conn).unwrap().check().unwrap();
        let tree_cookie = self.conn.query_tree(self.get_root()).expect("Unable to query x window tree");
        let reply = tree_cookie.reply().unwrap();
        let windows = reply.children;

        let conn = self.conn.clone();
        let check_manage_window = |window: &&u32| {
            let attributes = match window.attributes(&conn) {
                Ok(attr) => attr,
                Err(_) => return false, // unable to get attributes for client (ignoring client)
            };

            // FIXME also manage windows where state == IconicState
            attributes.map_state == xproto::MapState::VIEWABLE
        };

        // manage non-transient windows first
        let conn = self.conn.clone();
        windows.iter().filter(|w| { let conn = conn.clone(); w.transient_for(&conn).is_none() })
            .filter(check_manage_window).for_each(|w| self.manage(wm, *w));
        windows.iter().filter(|w| { let conn = conn.clone(); w.transient_for(&conn).is_some() })
            .filter(check_manage_window).for_each(|w| self.manage(wm, *w));

        xproto::ungrab_server(&self.conn).unwrap().check().unwrap();
    }

    fn mouse_action<WM: WindowManager<Self, A> + ?Sized>(&mut self, wm: &mut WM,
                    client_rc: Rc<RefCell<Self::Client>>, cursor_type: u32,
                    action: MouseActionFn<Self, WM, Self::Client>)
        where Self: Sized {
        todo!();
    }

    fn mouse_move(&mut self, wm: &mut dyn WindowManager<Self, A>, client_rc: Rc<RefCell<Self::Client>>) {
        todo!();
    }

    fn mouse_resize(&mut self, wm: &mut dyn WindowManager<Self, A>, client_rc: Rc<RefCell<Self::Client>>) {
        todo!();
    }

    fn point_to_monitor(&self, point: (i32, i32)) -> Option<u32> {
        todo!();
    }

    fn pointer_pos(&self) -> (i32, i32) {
        todo!();
    }

    fn run(mut self, wm: &mut dyn WindowManager<Self, A>) {
        loop {
            let event = self.conn.wait_for_event().unwrap();
            self.handle_event(wm, event);
        }
    }

    fn set_input_focus(&self, client_rc: Rc<RefCell<Self::Client>>) {
        todo!();
    }

    fn warp_pointer(&self, x: i32, y: i32) {
        todo!();
    }

    fn shutdown(&mut self) {
        todo!();
    }
}


fn sanitize_modifiers(modifiers: KeyButMask) -> KeyButMask {
    modifiers & (KeyButMask::SHIFT | KeyButMask::CONTROL | KeyButMask::MOD1 | KeyButMask::MOD3 | KeyButMask::MOD4)
}
