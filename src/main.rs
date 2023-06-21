use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::protocol::xproto::{
    ConnectionExt,
    CreateGCAux,
    ChangeWindowAttributesAux,
    AtomEnum,
    CloseDown,
    PropMode,
};
use x11rb::protocol::shm::ConnectionExt as ShmConnectionExt;

#[tokio::main]
async
fn main() {

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let depth = screen.root_depth;

    // check shm
    if !conn.query_extension(x11rb::protocol::shm::X11_EXTENSION_NAME.as_bytes()).unwrap().reply().unwrap().present {
        panic!("shm extension is not supported");
    }

    if !conn.query_extension(x11rb::protocol::randr::X11_EXTENSION_NAME.as_bytes()).unwrap().reply().unwrap().present {
        panic!("randr extension is not supported");
    }

    conn.set_close_down_mode(CloseDown::RETAIN_PERMANENT).unwrap();
    conn.sync().unwrap();

    // let pixmap = conn.generate_id().unwrap();
    // conn.create_pixmap(depth, pixmap, root, screen.width_in_pixels, screen.height_in_pixels).unwrap();

    let pm = xbg::shm::ShmPixmap::new(&conn, root, screen.width_in_pixels, screen.height_in_pixels).unwrap();

    println!("pixmap: 0x{:08x}", pm.pixmap);

    let gc = conn.generate_id().unwrap();
    println!("gc: 0x{:08x}", gc);
    let gc_aux = CreateGCAux::new()
        .background(screen.white_pixel)
        .foreground(0xffff0000);
    conn.create_gc(gc, root, &gc_aux).unwrap();

    // let mut buf = [0u8; 640 * 480 * 4];

    let prop_root = conn.intern_atom(false, b"_XROOTPMAP_ID").unwrap().reply().unwrap().atom;
    let prop_esetroot = conn.intern_atom(false, b"ESETROOT_PMAP_ID").unwrap().reply().unwrap().atom;

    conn.change_property32(
        PropMode::REPLACE,
        root,
        prop_root,
        AtomEnum::PIXMAP,
        &[pm.pixmap],
    ).unwrap();

    conn.change_property32(
        PropMode::REPLACE,
        root,
        prop_esetroot,
        AtomEnum::PIXMAP,
        &[pm.pixmap]
    ).unwrap();

    conn.change_window_attributes(root, &ChangeWindowAttributesAux::new().background_pixmap(pm.pixmap)).unwrap();

    let monitors = x11rb::protocol::randr::get_monitors(&conn, root, false).unwrap().reply().unwrap().monitors.iter().map(|m| {
        [m.x.try_into().unwrap(), m.y.try_into().unwrap(), m.width.into(), m.height.into()]
    }).collect::<Vec<_>>();
    println!("monitors: {:?}", monitors);

    let mut i = 1;

    let mut rnd = xbg::render::Renderer::new(
        [screen.width_in_pixels.into(), screen.height_in_pixels.into()],
        &monitors
        ).await;

    println!("start");


    conn.flush().unwrap();

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(16));

    let start = std::time::Instant::now();

    loop {
        let mut t = std::time::Instant::now();
        rnd.render(
            start.elapsed(),
            |buf| {
                println!("render {}us", t.elapsed().as_micros()); t = std::time::Instant::now();
                println!("buf: {}", buf.len());
                pm.shmseg.as_slice().copy_from_slice(&buf);
            }
        ).await.unwrap();
        conn.flush().unwrap();
        println!("copy {}us", t.elapsed().as_micros()); let t = std::time::Instant::now();

        // render; for non-compositor
        // conn.copy_area(pm.pixmap, root, gc, 0, 0, 0, 0, 500, 400).unwrap();
        conn.shm_put_image(
            //pm.pixmap,
            root,
            gc,
            screen.width_in_pixels,
            screen.height_in_pixels,
            0,
            0,
            screen.width_in_pixels,
            screen.height_in_pixels,
            0,
            0,
            24,
            x11rb::protocol::xproto::ImageFormat::Z_PIXMAP.into(),
            true,
            pm.shmseg.seg,
            0
        ).unwrap();

        println!("draw {}us", t.elapsed().as_micros()); let t = std::time::Instant::now();

        // notify compositor
        conn.change_property32(
            PropMode::REPLACE,
            root,
            prop_root,
            AtomEnum::PIXMAP,
            &[pm.pixmap],
        ).unwrap();

        conn.flush().unwrap();

        println!("notify {}us", t.elapsed().as_micros());

        // std::thread::sleep(std::time::Duration::from_millis(500));
        interval.tick().await;

        // if i > 5000 {
        //     break;
        // }
        i += 1;
    }

    // conn.get_property(true, root, prop_root, AtomEnum::ANY, 0, 1).unwrap().reply().unwrap();
    // conn.get_property(true, root, prop_esetroot, AtomEnum::ANY, 0, 1).unwrap().reply().unwrap();

    // std::thread::sleep(std::time::Duration::from_millis(100000));

    // unreachable but whatever
    // conn.free_pixmap(pixmap).unwrap();
    // conn.free_gc(gc).unwrap();

}
