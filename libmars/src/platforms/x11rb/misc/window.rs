use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::PropMode;
use x11rb::reexports::x11rb_protocol::protocol::xproto;

use crate::common::error::Result;
use crate::common::Dimensions;
use crate::platforms::x11::misc::atoms::X11Atom;


pub trait Window<'a, C: Connection> {
    fn attributes(self, conn: &'a C) -> Result<xproto::GetWindowAttributesReply>;
    fn dimensions(self, conn: &'a C) -> Result<Dimensions>;
    fn replace_property_char(&self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()>;
    fn replace_property_long(&self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()>;
    fn replace_property_string(&self, conn: &'a C, property: X11Atom, _type: X11Atom, string: &str) -> Result<()>;
    fn replace_property_strings(&self, conn: &'a C, property: X11Atom, _type: X11Atom, strings: &[&str]) -> Result<()>;
    fn transient_for(&self, conn: &'a C) -> Option<u32>;
    fn map(&self, conn: &'a C) -> Result<()>;
}

impl<'a, C: Connection> Window<'a, C> for u32 {
    fn attributes(self, conn: &'a C) -> Result<xproto::GetWindowAttributesReply> {
        Ok(conn.get_window_attributes(self)?.reply()?)
    }

    fn dimensions(self, conn: &'a C) -> Result<Dimensions> {
        let geometry = conn.get_geometry(self)?.reply()?;
        Ok(Dimensions::new(geometry.x as i32, geometry.y as i32, geometry.width as u32, geometry.height as u32))
    }

    fn replace_property_long(&self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()> {
        let property_request = conn.intern_atom(false, property.as_bytes())?;
        let type_request = conn.intern_atom(false, &_type.as_bytes())?;
        let reply = conn.change_property(PropMode::REPLACE, *self, property_request.reply()?.atom,
            type_request.reply()?.atom, 32, (data.len() / 4).try_into().unwrap(), data)?.check()?;
        Ok(reply)
    }

    fn replace_property_char(&self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()> {
        let property_request = conn.intern_atom(false, property.as_bytes())?;
        let type_request = conn.intern_atom(false, &_type.as_bytes())?;
        let reply = conn.change_property(PropMode::REPLACE, *self, property_request.reply()?.atom,
            type_request.reply()?.atom, 8, data.len().try_into().unwrap(), data)?.check()?;
        Ok(reply)
    }
    fn replace_property_string(&self, conn: &'a C, property: X11Atom, _type: X11Atom, string: &str) -> Result<()> {
        let mut string_buffer = Vec::new();
        string_buffer.extend(string.as_bytes());
        string_buffer.push(0x0);
        self.replace_property_char(conn, property, _type, &string_buffer)
    }


    fn replace_property_strings(&self, conn: &'a C, property: X11Atom, _type: X11Atom, strings: &[&str]) -> Result<()> {
        let strings_buffer: Vec<u8>= strings.iter()
            .map(|n| n.as_bytes())
            .fold(Vec::new(), |mut v, b| { v.extend_from_slice(b); v.push(0x0); v });
        self.replace_property_char(conn, property, _type, &strings_buffer)
    }

    fn transient_for(&self, conn: &'a C) -> Option<u32> {
        let wm_transient_for = conn.intern_atom(false, X11Atom::WMTransientFor.as_bytes()).ok()?.reply().ok()?.atom;
        let cookie = conn.get_property(false, *self, wm_transient_for, xproto::AtomEnum::WINDOW, 0, std::u32::MAX).ok()?;
        let reply = cookie.reply().ok()?;
        // TODO: check if windows are always 32bit
        if reply.type_ != xproto::AtomEnum::WINDOW.into() || reply.format != 32 {
            None
        } else {
            reply.value32()?.next()
        }
    }

    fn map(&self, conn: &'a C) -> Result<()> {
        Ok(conn.map_window(*self)?.check()?)
    }
}
