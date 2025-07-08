use core::{
    marker::PhantomData,
    ptr::{null, null_mut, NonNull},
};

use crate::{device::Device, error::VTABLE_DEFAULT_ERROR};
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

pub struct Registration<T> {
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

// TODO Safety
unsafe impl<T: Send + Sync> Send for Registration<T> {}
unsafe impl<T: Send + Sync> Sync for Registration<T> {}

// TODO Sealed for now, figure out if necessary
impl<T: Send + Sync> crate::private::Sealed for Registration<T> {}

impl<T: Driver> Registration<T> {
    pub fn new(name: &'static CStr, dev: &Device, module: &'static ThisModule) -> Result<Self> {
        let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;

        // On failure, iio_device_alloc returns NULL
        let indio_dev = unsafe { bindings::iio_device_alloc(dev.as_raw(), sizeof_priv) };
        if indio_dev.is_null() {
            return Err(ENOMEM);
        }
        unsafe {
            // TODO: Safety - can fail if CHANNELS is bigger than i32
            (*indio_dev).name = name.as_bytes_with_nul().as_ptr();
            (*indio_dev).num_channels = T::CHANNELS.len() as i32;
        }

        // TODO use iio_device_register instead?
        // TODO Safety
        // If ret is negative register has failed
        let ret = unsafe {
            bindings::__devm_iio_device_register(dev.as_raw(), indio_dev, module.as_ptr())
        };

        if ret < 0 {
            unsafe { bindings::iio_device_free(indio_dev) };
            // TODO Check error to return
            return Err(EINVAL);
        }


        Ok(Self {
            indio_dev: NonNull::new(indio_dev).ok_or(EINVAL)?,
            _priv: PhantomData,
        })
    }
}

fn to_raw_channels(channels: &'static [IioChanSpec]) -> *const bindings::iio_chan_spec {
    todo!()
}

#[vtable]
pub trait Driver {
    const CHANNELS: &'static [IioChanSpec];
    fn read_raw<T>(
        indio_dev: &mut Registration<T>,
        _channel: &IioChanSpec,
        _fst: i32,
        _snd: i32,
        _mask: isize,
    ) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw<T>(
        indio_dev: &mut Registration<T>,
        _channel: &IioChanSpec,
        _fst: i32,
        _snd: i32,
        _mask: isize,
    ) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
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
        read_raw: if T::HAS_READ_RAW {
            Some(Self::read_raw)
        } else {
            None
        },
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