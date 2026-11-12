//! Dragging a meme out of the pane on Linux (GTK 3).
//!
//! The `drag` crate's GTK backend never delivers the file: it starts the drag with the
//! button mask where GTK wants the button number (crabnebula-dev/drag-rs#103), and it
//! disconnects its data handler as soon as the drop is sent, before the target has asked
//! for the data (drag-rs#84). This is the same drag, cleaned up only once GTK says it's over.

use gtk::glib::prelude::ObjectExt as _;
use gtk::{gdk, gdk_pixbuf, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

/// Starts dragging `file`, with `icon` (PNG bytes) under the cursor. `on_end` runs once
/// the drag is over, with whether it was dropped on something that took it.
pub fn start(
    window: &gtk::ApplicationWindow,
    file: &Path,
    icon: &[u8],
    on_end: impl Fn(bool) + 'static,
) -> Result<(), String> {
    let uri = glib::filename_to_uri(file, None).map_err(|e| e.to_string())?;
    let targets = gtk::TargetList::new(&[]);
    targets.add_uri_targets(0);

    let handlers: Rc<RefCell<Vec<glib::SignalHandlerId>>> = Rc::default();
    let dropped = Rc::new(Cell::new(false));

    // The target asks for the data after the drop, so this stays connected until drag-end.
    let data_get = window.connect_drag_data_get(move |_, _, data, _, _| {
        data.set_uris(&[uri.as_str()]);
    });
    let end = {
        let handlers = handlers.clone();
        let dropped = dropped.clone();
        window.connect_drag_end(move |window, _| {
            for id in handlers.take() {
                window.disconnect(id);
            }
            on_end(dropped.get());
        })
    };
    handlers.borrow_mut().extend([data_get, end]);

    // Button 1: the pane only starts drags from a primary-button press.
    let Some(context) = window.drag_begin_with_coordinates(&targets, gdk::DragAction::COPY, 1, None, -1, -1) else {
        for id in handlers.take() {
            window.disconnect(id);
        }
        return Err("GTK didn't start the drag.".into());
    };
    context.connect_drop_performed(move |_, _| dropped.set(true));
    if let Some(pixbuf) = pixbuf(icon) {
        context.drag_set_icon_pixbuf(&pixbuf, pixbuf.width() / 2, pixbuf.height() / 2);
    }
    Ok(())
}

fn pixbuf(png: &[u8]) -> Option<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(png).ok()?;
    loader.close().ok()?;
    loader.pixbuf()
}
