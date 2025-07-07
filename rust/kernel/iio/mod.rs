use core::{marker::PhantomData, ptr::{null, null_mut, NonNull}};

use bindings::{__iio_device_register, iio_device_alloc};

use crate::{device, driver, error::to_result, str::CStr, types::Opaque, ThisModule};
use crate::prelude::*;

struct Adapter<T>(T);

unsafe impl<T> driver::RegistrationOps for Adapter<T> {
    type RegType = bindings::iio_dev;

    unsafe fn register(
        indio_dev: &Opaque<Self::RegType>,
        name: &'static CStr,
        module: &'static ThisModule,
    ) -> Result {

        // let indio_dev = unsafe { iio_device_alloc(null_mut::<device>(), 0)};
        to_result(unsafe {
            __iio_device_register(indio_dev.get(), module.as_ptr())
        })
    }

    unsafe fn unregister(indio_dev: &Opaque<Self::RegType>) {
        unsafe { bindings::iio_device_unregister(indio_dev.get()) };
    }
}

impl<T> Adapter<T> {
    extern "C" fn probe_callback() {

    }
    extern "C" fn remove_callback() {

    }
}

pub struct SWDevice;

pub trait SWDriver {
    fn probe(name: &'static CStr) -> Pin<KBox<Self>>;
}

pub struct IioDevice<T> {
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

impl<T> IioDevice<T> {
    fn probe(dev: &crate::device::Device, module: &'static ThisModule) {
        let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;
        let indio_dev = unsafe { bindings::iio_device_alloc(dev.as_raw(), sizeof_priv)};
        // unsafe { bindings::iio_priv }
        let ret = unsafe { bindings::__devm_iio_device_register(dev.as_raw(), indio_dev, module.as_ptr())};
    }
}
#[vtable]
pub trait Driver<const N: usize> {
    const CHANNELS: [IioChanSpec; N];
    fn read_raw<T>(indio_dev: &mut IioDevice<T>, _channel: &IioChanSpec, _fst: u32, _snd: u32, _mask: u32);
    fn write_raw<T>(indio_dev: &mut IioDevice<T>, _channel: &IioChanSpec, _fst: u32, _snd: u32, _mask: u32);
}


pub struct IioChanSpec;

/// struct iio_chan_spec[]
pub struct Channels(KVec<IioChanSpec>);

impl IioChanSpec {
    pub fn with_light(mut self) -> Self {
        // let channel = 
        todo!()
    }

    pub fn with_temp(mut self) -> Self {
        todo!()
    }
}


#[macro_export]
macro_rules! module_iio_sw_device_driver {
    ($($f:tt)*) => {
        $crate::module_driver!(<T>, $crate::iio::Adapter<T>, { $($f)* });
    };
}