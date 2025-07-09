use core::{
    marker::PhantomData,
    ptr::NonNull,
    mem::MaybeUninit,
};

use crate::{device::Device, error::VTABLE_DEFAULT_ERROR, prelude::*, str::CStr, ThisModule};

pub mod trigger;
pub mod channels;

pub use channels::{Specification, IIOValue, ChannelType};

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
    pub fn new(dev: &Device, module: &'static ThisModule, options: &RegistrationOptions) -> Result<Self> {
        // Size is probably wrong, needs to be the T::Ptr, private data
        // TODO: Instead of iio_priv, store private data on the Rust side?
        let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;

        // On failure, iio_device_alloc returns NULL
        // let indio_dev = unsafe { bindings::iio_device_alloc(dev.as_raw(), sizeof_priv) };
        // let indio_dev = Opaque::ffi_init(|indio_dev: *mut bindings::iio_dev| {
        //     unsafe { indio_dev.write( bindings::iio_device_alloc(dev.as_raw(), sizeof_priv)  ) };
        // });
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

#[vtable]
pub trait Driver: Sized {
    type Ptr;
    const CHANNELS: &'static [Specification];
    fn read_raw(
        indio_dev: &Registration<Self>,
        _channel: &Specification,
        _val: &i32,
        _snd: &i32,
        _mask: isize,
    ) -> IIOValue {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(
        indio_dev: &Registration<Self>,
        _channel: &Specification,
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
        let st = unsafe { &*bindings::iio_priv(indio_dev).cast::<T::Ptr>() };
        let indio_dev = unsafe { &*indio_dev.cast::<Registration<T>>() };
        let val = unsafe { &*val.cast::<i32>() };
        let val2 = unsafe { &*val2.cast::<i32>() };
        let channel = unsafe { &* iio_chan_spec.cast::<Specification>() };
        let ret = T::read_raw(indio_dev, channel, val, val2, mask);
        ret as ffi::c_int
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