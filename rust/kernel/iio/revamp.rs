use core::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use crate::{device::Device, error::{to_result, VTABLE_DEFAULT_ERROR}, iio::channels::{Buffered, Channel, Simple}, prelude::*, str::CStr, types::{ARef, ForeignOwnable, Opaque}, ThisModule};

use crate::iio::channels::{ChannelType, SensorValue, Specification, SensorData};

#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct BetterRegistration<T> {
    #[pin]
    indio_dev: Opaque<bindings::iio_dev>,
    _priv: PhantomData<T>
}

impl<'a, 'b, T: Driver> BetterRegistration<T> {
    pub fn new(
        dev: &'a Device,
        module: &'static ThisModule,
        options: &'b RegistrationOptions,
    ) -> impl PinInit<Self, Error> + use<'a, 'b, T> {

        try_pin_init!(Self {
            indio_dev <- Opaque::try_ffi_init(move |slot: *mut bindings::iio_dev| {
                unsafe {
                    (*slot).name = options.name.as_char_ptr();
                    (*slot).priv_ = T::alloc()?.into_foreign().cast();
                    (*slot).channels = T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec;
                    (*slot).num_channels = T::CHANNELS.len() as i32;
                    (*slot).modes = Mode::Direct as i32;
                    (*slot).info = IioVTableAdapter::<T>::build() as *const bindings::iio_info;
                    (*slot).dev.parent = dev.as_raw();
                }
                to_result(unsafe { bindings::__devm_iio_device_register(dev.as_raw(), slot, module.as_ptr()) })
            }),
            _priv: PhantomData,
        })
    }
}

#[pinned_drop]
impl<T> PinnedDrop for BetterRegistration<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe { bindings::iio_device_unregister(self.indio_dev.get()) };
        let _ = unsafe { KBox::from_foreign((*self.indio_dev.get()).priv_) };
    }
}

unsafe impl<T: Send + Sync> Send for BetterRegistration<T> {}
unsafe impl<T: Send + Sync> Sync for BetterRegistration<T> {}

// TODO Sealed for now, figure out if necessary
impl<T: Send + Sync> crate::private::Sealed for BetterRegistration<T> {}

pub struct RegistrationOptions {
    pub name: &'static CStr,
    pub mode: Mode,
}

#[repr(u32)]
pub enum Mode {
    Direct = bindings::INDIO_DIRECT_MODE,
}

#[vtable]
pub trait Driver: Sized {
    type Ptr: ForeignOwnable + Send + Sync;
    const CHANNELS: &'static [Channel];

    fn alloc() -> Result<Self::Ptr>;

    fn read_raw(
        data: <Self::Ptr as ForeignOwnable>::Borrowed<'_>,
        _channel: &Specification,
    ) -> SensorData<i32> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(
        data: <Self::Ptr as ForeignOwnable>::BorrowedMut<'_>,
        _channel: &Specification,
        _value: i32,
    ) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}

pub struct IioVTableAdapter<T: Driver>(PhantomData<T>);

impl<T: Driver> IioVTableAdapter<T> where {
    unsafe extern "C" fn read_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: *mut ffi::c_int,
        // Ignore for now
        _val2: *mut ffi::c_int,
        mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        let private = unsafe { bindings::iio_priv(indio_dev)}.cast();
        let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        let device = unsafe { <T::Ptr as ForeignOwnable>::borrow(private) };


        // TODO need to check the mask before casting
        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        let ret = T::read_raw(device, channel);

        // // TODO check if val is always not null
        unsafe { *val = ret.inner(); }
        SensorData::SENSOR_VALUE as ffi::c_int
    }
    unsafe extern "C" fn write_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: ffi::c_int,
        val2: ffi::c_int,
        mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        let private = unsafe { bindings::iio_priv(indio_dev)}.cast();
        let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        let device = unsafe { <T::Ptr as ForeignOwnable>::borrow_mut(private) };

        // TODO need to check the mask before casting
        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        match T::write_raw(device, channel, val) {
            Ok(_) => SensorValue::Int as ffi::c_int,
            // TODO can I return kernel errors here?
            Err(_) => EINVAL.to_errno(),
        }

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
