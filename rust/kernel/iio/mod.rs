use core::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use crate::{device::Device, error::{to_result, VTABLE_DEFAULT_ERROR}, iio::channels::{Buffered, Channel, Simple}, prelude::*, str::CStr, types::{ForeignOwnable, Opaque}, ThisModule};

pub mod channels;
pub mod trigger;
mod buffer;
mod revamp;

pub use channels::{ChannelType, SensorValue, Specification, SensorData};

pub struct RegistrationOptions {
    pub name: &'static CStr,
    pub mode: Mode,
}

// TODO: Instead of iio_priv, store private data on the Rust side?
#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Registration<T> {
    #[pin]
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

// TODO Safety
unsafe impl<T: Send + Sync> Send for Registration<T> {}
unsafe impl<T: Send + Sync> Sync for Registration<T> {}

// TODO Sealed for now, figure out if necessary
impl<T: Send + Sync> crate::private::Sealed for Registration<T> {}

impl<T: Driver> Registration<T> {
    pub fn new(
        dev: &Device,
        module: &'static ThisModule,
        options: &RegistrationOptions,
    ) -> Result<Self> {
        let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;
        let indio_dev = unsafe { bindings::iio_device_alloc(dev.as_raw(), sizeof_priv) };
        if indio_dev.is_null() {
            return Err(ENOMEM);
        }

        unsafe {
            (*indio_dev).name = options.name.as_char_ptr();
            // Have &'static [Specification]
            // Need a *const iio_chan_spec
            // Specification is repr(transpent) iio_chan_spec
            // &'static[].as_ptr() should be valid since its static
            (*indio_dev).channels = T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec;
            (*indio_dev).num_channels = T::CHANNELS.len() as i32;
            (*indio_dev).modes = Mode::Direct as i32;
            (*indio_dev).info = IioVTableAdapter::<T>::build() as *const bindings::iio_info;
        }

        // TODO Here we need to do any other configuration with buffers, etc.
        // Possibly move into RegistrationOptions

        // In the ideal case, we automatically configure all the buffer/event channels
        // If they are present in T::CHANNELS.

        // TODO Need to confirm, can a device have multiple buffers and events?

        // TODO We need to do all of the configuration here
        // Some ideas are loop over T::CHANNELS
        // for channel in T::CHANNELS { /* configuration */ }
        // If the channel is Specification<T> where T: BufferChannel
        // Run the corresponding configuration with
        // <T as BufferChannel>::configure (needs to be added)
        // However this means that we need to loop over every channel,
        // every time a device is registered. Maybe the compiler will see
        // that nothing is done for the Specification<Simple>?

        // Other idea is iio::Driver has statically allocated
        // associate constant array for simple arrays, and buffer arrays
        // Then we only loop over the buffer arrays to call their
        // configure. The only issue is that we need to eventually
        // stitch them all back together
        // into a single statically allocated iio_chan_spec[]
        // Probably some macro magic can accomplish this.
        // For the future, we'd probably also need to add a separate
        // one for events, and maybe others depending if the C code expands

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

    fn device(&self) -> &Device {
        todo!()
    }
}

#[pinned_drop]
impl<T> PinnedDrop for Registration<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe {
            bindings::iio_device_unregister(self.indio_dev.as_ptr());
            bindings::iio_device_free(self.indio_dev.as_ptr());
        }
    }
}

#[repr(u32)]
pub enum Mode {
    Direct = bindings::INDIO_DIRECT_MODE,
}

pub trait Wraps<D> { }

impl<D> Wraps<D> for Pin<KBox<D>> { }
impl<D> Wraps<D> for Pin<KBox<crate::sync::Mutex<D>>> { }
impl<D> Wraps<D> for crate::sync::Mutex<D> { }
impl<D> Wraps<D> for &'static D { }

#[vtable]
pub trait Driver: Sized {
    type Data;
    type Ptr: ForeignOwnable + Send + Sync + Wraps<Self::Data>;
    const CHANNELS: &'static [Channel];

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

impl<T: Driver> IioVTableAdapter<T> {
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
