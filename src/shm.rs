use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shm::ConnectionExt as ShmConnectionExt;
use x11rb::protocol::xproto::ConnectionExt as XpConnectionExt;
use libc::{shmat, shmdt, shmget, IPC_CREAT, IPC_PRIVATE};
use std::ptr;

pub struct ShmSegWrapper {
    pub seg: u32,
    pub shm_addr: *mut u8,
    pub size: usize,
}

impl ShmSegWrapper {

    pub fn new(connection: &x11rb::rust_connection::RustConnection, size: usize)
        -> Result<Self, x11rb::errors::ConnectionError> {

        if connection
            .extension_information(x11rb::protocol::shm::X11_EXTENSION_NAME)?
            .is_none() {
                return Err(x11rb::errors::ConnectionError::UnsupportedExtension);
        }

        let seg = connection.generate_id().unwrap();

        let shm_addr = unsafe {
            let shmid = shmget(
                IPC_PRIVATE,
                size,
                IPC_CREAT | 0o777,
            );

            if shmid < 0 {
                panic!("shmget failed");
            }

            let shm_addr = shmat(shmid, ptr::null_mut(), 0);

            if shm_addr == -1isize as *mut _ {
                dbg!(std::io::Error::last_os_error().raw_os_error().unwrap());
                panic!("shmat failed");
            }

            dbg!(
                shmid,
                shm_addr,
                size
            );

            connection.shm_attach(seg, shmid as u32, false)?;

            shm_addr
        };

        Ok(Self {
            seg,
            shm_addr: shm_addr as *mut u8,
            size,
        })

    }

    pub fn as_slice(&self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.shm_addr, self.size)
        }
    }
}

impl Drop for ShmSegWrapper {
    fn drop(&mut self) {
        if !self.shm_addr.is_null() {
            unsafe { shmdt(self.shm_addr as _); }
        }
    }
}

pub struct ShmPixmap {
    pub pixmap: u32,
    pub shmseg: ShmSegWrapper,

    pub width: u16,
    pub height: u16,

    pub drawable: x11rb::protocol::xproto::Drawable,
}

impl ShmPixmap {
    pub fn new(
        connection: &x11rb::rust_connection::RustConnection,
        drawable: x11rb::protocol::xproto::Drawable,
        width: u16,
        height: u16,
        ) -> Result<Self, x11rb::errors::ConnectionError> {

        let shmseg = ShmSegWrapper::new(connection, width as usize * height as usize * 4)?;

        let pixmap = connection.generate_id().unwrap();

        // connection.create_pixmap(
        //     32,
        //     pixmap,
        //     connection.setup().roots[0].root,
        //     width,
        //     height,
        // )?;

        connection.shm_create_pixmap(
            pixmap,
            drawable,
            width,
            height,
            32,
            shmseg.seg,
            0,
        )?;

        Ok(Self {
            pixmap,
            shmseg,

            width,
            height,

            drawable,
        })
    }

    pub fn resize(&mut self, connection: &x11rb::rust_connection::RustConnection, width: u16, height: u16) -> Result<(), x11rb::errors::ConnectionError> {

        connection.free_pixmap(self.pixmap).unwrap();

        self.shmseg = ShmSegWrapper::new(connection, width as usize * height as usize * 4)?;

        self.width = width;
        self.height = height;

        // self.pixmap = connection.generate_id().unwrap();
        connection.shm_create_pixmap(
            self.pixmap,
            self.drawable,
            width,
            height,
            32,
            self.shmseg.seg,
            0,
        )?;
        Ok(())
    }

}
