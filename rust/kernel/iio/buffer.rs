use core::marker::PhantomData;

use kernel::prelude::*;

use bindings::iio_buffer_setup_ops;

use crate::types::Opaque;

pub struct Buffer(Opaque<bindings::iio_buffer>);

pub struct ScanElement {
    pub scan_index: i32,
    pub sign: char,
    pub realbits: u8,
    pub storagebits: u8,
    pub shift: u8,
    pub repeat: u8,
    pub endianness: Endian,
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum Endian {
    Big = bindings::iio_endian_IIO_BE,
    Little = bindings::iio_endian_IIO_LE,
    Cpu = bindings::iio_endian_IIO_CPU,
}

#[vtable]
pub trait BufferChannel {
    type Data;
    const SCAN_TYPE: &'static ScanElement;

    // trigger handler thread in iio_triggered_buffer_setup
    fn irq_handler();

    // iio_buffer_setup_ops
    fn preenable();
    fn postenable();
    fn predisable();
    fn postdisable();
    fn validate_scan_mask();
}

pub struct BufferOpsVtable<T: BufferChannel>(PhantomData<T>);

impl<T: BufferChannel> BufferOpsVtable<T> {
    unsafe extern "C" fn preenable(iio_dev: *mut bindings::iio_dev) -> ffi::c_int {
        todo!()
    }

    unsafe extern "C" fn postenable(iio_dev: *mut bindings::iio_dev) -> ffi::c_int {
        todo!()
    }

    unsafe extern "C" fn predisable(iio_dev: *mut bindings::iio_dev) -> ffi::c_int {
        todo!()
    }

    unsafe extern "C" fn postdisable(iio_dev: *mut bindings::iio_dev) -> ffi::c_int {
        todo!()
    }

    unsafe extern "C" fn validate_scan_mask(
        indio_dev: *mut bindings::iio_dev,
        scan_mask: *const usize,
    ) -> bool {
        todo!()
    }
    unsafe extern "C" fn thread(irq: ffi::c_int, p: *mut ffi::c_void) -> bindings::irqreturn_t {
        todo!()
    }

    const VTABLE: bindings::iio_buffer_setup_ops = iio_buffer_setup_ops {
        preenable: if T::HAS_PREENABLE {
            Some(Self::preenable)
        } else {
            None
        },
        postenable: if T::HAS_POSTENABLE {
            Some(Self::postenable)
        } else {
            None
        },
        predisable: if T::HAS_PREDISABLE {
            Some(Self::predisable)
        } else {
            None
        },
        postdisable: if T::HAS_POSTDISABLE {
            Some(Self::postdisable)
        } else {
            None
        },
        validate_scan_mask: if T::HAS_VALIDATE_SCAN_MASK {
            Some(Self::validate_scan_mask)
        } else {
            None
        },
    };

    const fn build() -> &'static iio_buffer_setup_ops {
        &Self::VTABLE
    }

    // fn configure(indio_dev: *mut bindings::iio_dev ) -> ffi::c_int {
    //     unsafe { bindings::__iio_trigg
    // }
}
