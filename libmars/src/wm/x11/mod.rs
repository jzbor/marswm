use x11::xlib;
use std::ffi::*;

pub mod backend;
mod client;

extern "C" fn on_wm_detected(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> c_int {
    panic!("Another Window Manager seems to be running already");
}

extern "C" fn on_error(display: *mut xlib::Display, error: *mut xlib::XErrorEvent) -> c_int {
    let msg = unsafe {
        let bufsize = 1024;
        let mut buf = vec![0u8; bufsize];
        xlib::XGetErrorText(display, (*error).error_code.into(), buf.as_mut_ptr() as *mut i8,
                            (bufsize - 1) as c_int);
        let msg_cstring = CStr::from_ptr(buf.as_mut_ptr() as *mut i8);
        msg_cstring.to_str().unwrap().to_owned()
        // println!("{}", msg);
    };

    unsafe {
        match (*error).error_code {
            xlib::Success => println!("X11 non-error: {} (request code: {})", msg, (*error).request_code),
            xlib::BadMatch => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            xlib::BadWindow => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            _ => panic!("Fatal X11 error: {} (request code: {})", msg, (*error).request_code),
        }
    }

    // unsafe {
    //     match (*error).error_code {
    //         // @TODO add error types
    //         xlib::Success => println!("everything's okay"),
    //         xlib::BadRequest => panic!("bad request code"),
    //         xlib::BadValue => panic!("int parameter out of range"),
    //         xlib::BadWindow => println!("parameter not a Window"),
    //         xlib::BadPixmap => panic!("parameter not a Pixmap"),
    //         xlib::BadAtom => panic!("parameter not an Atom"),
    //         xlib::BadCursor => panic!("parameter not a Cursor"),
    //         xlib::BadFont => panic!("parameter not a Font"),
    //         xlib::BadMatch => panic!("parameter mismatch"),
    //         xlib::BadDrawable => panic!("parameter not a Pixmap or Window"),
    //         xlib::BadAccess => panic!("depending on context (see X.h)"),
    //         xlib::BadAlloc => panic!("insufficient resources"),
    //         xlib::BadColor => panic!("no such colormap"),
    //         xlib::BadGC => panic!("parameter not a GC"),
    //         xlib::BadIDChoice => panic!("choice not in range or already used"),
    //         xlib::BadName => panic!("font or color name doesn't exist"),
    //         xlib::BadLength => panic!("Request length incorrect"),
    //         xlib::BadImplementation => panic!("server is defective"),
    //         _ => panic!("unknown error occured"),
    //     }
    // }

    return 0;
}
