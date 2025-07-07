use core::{
    marker::PhantomData,
    ptr::{null, null_mut, NonNull},
};

use crate::error::VTABLE_DEFAULT_ERROR;
use crate::prelude::*;
use crate::{device, driver, error::to_result, str::CStr, types::Opaque, ThisModule};
use core::mem::MaybeUninit;

// struct Adapter<T>(T);

// unsafe impl<T> driver::RegistrationOps for Adapter<T> {
//     type RegType = bindings::iio_dev;

//     unsafe fn register(
//         indio_dev: &Opaque<Self::RegType>,
//         name: &'static CStr,
//         module: &'static ThisModule,
//     ) -> Result {

//         // let indio_dev = unsafe { iio_device_alloc(null_mut::<device>(), 0)};
//         to_result(unsafe {
//             __iio_device_register(indio_dev.get(), module.as_ptr())
//         })
//     }

//     unsafe fn unregister(indio_dev: &Opaque<Self::RegType>) {
//         unsafe { bindings::iio_device_unregister(indio_dev.get()) };
//     }
// }

pub struct IioDevice<T> {
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

// TODO Safety
unsafe impl<T: Send + Sync> Send for IioDevice<T> {}
unsafe impl<T: Send + Sync> Sync for IioDevice<T> {}

// TODO Sealed for now, figure out if necessary
impl<T: Send + Sync> crate::private::Sealed for IioDevice<T> {}

impl<T> IioDevice<T> {
    pub fn register(dev: &crate::device::Device, module: &'static ThisModule) -> Result<Self> {
        let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;
        // TODO Safety
        let indio_dev = unsafe { bindings::iio_device_alloc(dev.as_raw(), sizeof_priv) };

        // unsafe { bindings::iio_priv }

        // TODO Safety
        let ret = unsafe {
            bindings::__devm_iio_device_register(dev.as_raw(), indio_dev, module.as_ptr())
        };

        Ok(Self {
            indio_dev: NonNull::new(indio_dev).ok_or(EINVAL)?,
            _priv: PhantomData,
        })
    }
}
#[vtable]
pub trait Driver {
    const CHANNELS: &'static [IioChanSpec];
    fn read_raw<T>(
        indio_dev: &mut IioDevice<T>,
        _channel: &IioChanSpec,
        _fst: i32,
        _snd: i32,
        _mask: isize,
    ) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw<T>(
        indio_dev: &mut IioDevice<T>,
        _channel: &IioChanSpec,
        _fst: i32,
        _snd: i32,
        _mask: isize,
    );
}

pub struct IioVTableAdapter<T: Driver>(PhantomData<T>);

impl<T: Driver> IioVTableAdapter<T> {
    unsafe extern "C" fn read_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: *mut ffi::c_int,
        val2: *mut ffi::c_int,
        mask: isize,
    ) -> ffi::c_int {
        todo!()
    }
    unsafe extern "C" fn write_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: ffi::c_int,
        val2: ffi::c_int,
        mask: isize,
    ) -> ffi::c_int {
        todo!()
    }

    const VTABLE: bindings::iio_info = bindings::iio_info {
        read_raw: Some(Self::read_raw),
        write_raw: if T::HAS_WRITE_RAW {
            Some(Self::write_raw)
        } else {
            None
        },
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    const fn build() -> &'static bindings::iio_info {
        &Self::VTABLE
    }
}

pub struct IioChanSpec;