use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::PropMode;

use crate::common::error::Result;
use crate::platforms::x11::misc::atoms::X11Atom;


pub trait Window<'a, C: Connection> {
    fn replace_property_long(self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()>;
}

impl<'a, C: Connection> Window<'a, C> for u32 {
    fn replace_property_long(self, conn: &'a C, property: X11Atom, _type: X11Atom, data: &[u8]) -> Result<()>
    {
        let property_request = conn.intern_atom(false, property.as_bytes())?;
        let type_request = conn.intern_atom(false, &_type.as_bytes())?;
        let reply = conn.change_property(PropMode::REPLACE, self, property_request.reply()?.atom,
            type_request.reply()?.atom, 32, data.len().try_into().unwrap(), data)?.check()?;
        Ok(reply)
    }
}
