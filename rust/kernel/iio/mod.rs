#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(missing_docs)]
use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};

use crate::{
    device::Device, error::VTABLE_DEFAULT_ERROR, iio::channels::Channel, prelude::*, str::CStr,
    types::ForeignOwnable, ThisModule,
};

// mod buffer;
pub mod channels;
pub mod revamp;
pub mod trigger;

pub use channels::{ChannelType, SensorData, SensorValue, Specification};

pub struct RegistrationOptions {
    pub name: &'static CStr,
    pub mode: Mode,
}

// TODO: Instead of iio_priv, store private data on the Rust side?
#[repr(transparent)]
pub struct Registration<T: Driver> {
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

// TODO Safety
unsafe impl<T: Send + Sync + Driver> Send for Registration<T> {}
unsafe impl<T: Send + Sync + Driver> Sync for Registration<T> {}

// TODO Sealed for now, figure out if necessary

#[repr(transparent)]
struct ChanSpec(bindings::iio_chan_spec);

unsafe impl Send for ChanSpec {}
unsafe impl Sync for ChanSpec {}

static FAKECHANNELS: &'static [ChanSpec] = &[];

impl<T: Driver> Registration<T> {
    pub fn new(
        dev: &Device,
        module: &'static ThisModule,
        options: &RegistrationOptions,
        data: T::Ptr,
        // data: impl PinInit<T::Data, Error>
    ) -> Result<Self> {
        // let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;
        pr_info!("iio: Registering with device as parent");
        let sizeof_priv = 0 as ffi::c_int;
        let indio_dev = unsafe { bindings::devm_iio_device_alloc(dev.as_raw(), sizeof_priv) };
        if indio_dev.is_null() {
            return Err(ENOMEM);
        }

        unsafe {
            // Have &'static [Specification]
            // Need a *const iio_chan_spec
            // Specification is repr(transpent) iio_chan_spec
            // &'static[].as_ptr() should be valid since its static
            // (*indio_dev).name = options.name.as_char_ptr();
            // (*indio_dev).channels = T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec;
            // (*indio_dev).num_channels = T::CHANNELS.len() as i32;
            // (*indio_dev).modes = Mode::Direct as i32;
            // (*indio_dev).info = IioVTableAdapter::<T>::build() as *const bindings::iio_info;
            ptr::addr_of_mut!((*indio_dev).name).write(options.name.as_char_ptr());
            ptr::addr_of_mut!((*indio_dev).channels)
                .write(FAKECHANNELS.as_ptr() as *const bindings::iio_chan_spec);
            ptr::addr_of_mut!((*indio_dev).num_channels).write(FAKECHANNELS.len() as i32);
            ptr::addr_of_mut!((*indio_dev).modes).write(Mode::Direct as i32);
            ptr::addr_of_mut!((*indio_dev).info)
                .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
        }
        pr_info!("Initialized indio_dev");

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

impl<T: Driver> Drop for Registration<T> {
    fn drop(&mut self) {
        unsafe {
            //             bindings::iio_device_unregister(self.indio_dev.as_ptr());
            bindings::iio_device_free(self.indio_dev.as_ptr());
        }
    }
}

// #[pinned_drop]
// impl<T> PinnedDrop for Registration<T> {
//     fn drop(self: Pin<&mut Self>) {
//         unsafe {
//             bindings::iio_device_unregister(self.indio_dev.as_ptr());
//             bindings::iio_device_free(self.indio_dev.as_ptr());
//         }
//     }
// }

#[repr(u32)]
pub enum Mode {
    Direct = bindings::INDIO_DIRECT_MODE,
}

#[vtable]
pub trait Driver: Sized {
    // type Data: Send + Sync;
    type Ptr: ForeignOwnable + Send + Sync;
    // type Ptr = Pin<KBox<Self>>;
    const CHANNELS: &'static [Channel];

    fn read_raw(_data: Pin<&Self>, _channel: &Specification) -> SensorData<i32> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(_data: Pin<&mut Self>, _channel: &Specification, _value: i32) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}

pub struct IioVTableAdapter<T: Driver>(PhantomData<T>);

impl<T: Driver> IioVTableAdapter<T> {
    unsafe extern "C" fn read_raw(
        _indio_dev: *mut bindings::iio_dev,
        _iio_chan_spec: *const bindings::iio_chan_spec,
        _val: *mut ffi::c_int,
        // Ignore for now
        _val2: *mut ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        // // Copied from kernel::miscdevice
        // let private = unsafe { bindings::iio_priv(_indio_dev) }.cast();
        // // let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        // let device = unsafe { <T::Ptr as ForeignOwnable>::borrow(private) };

        // // TODO need to check the mask before casting
        // let channel = unsafe { &*_iio_chan_spec.cast::<Specification<Simple>>() };
        // let ret = T::read_raw(device, channel);

        // // // TODO check if val is always not null
        // unsafe {
        //     *_val = ret.inner();
        // }
        // <ret as Sensor>::SENSOR_VALUE as ffi::c_int
        todo!()
    }
    unsafe extern "C" fn write_raw(
        _indio_dev: *mut bindings::iio_dev,
        _iio_chan_spec: *const bindings::iio_chan_spec,
        _val: ffi::c_int,
        _val2: ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        // let private = unsafe { bindings::iio_priv(_indio_dev) }.cast();
        // // let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        // let device = unsafe { <T::Ptr as ForeignOwnable>::borrow_mut(private) };

        // // TODO need to check the mask before casting
        // let channel = unsafe { &*_iio_chan_spec.cast::<Specification<Simple>>() };
        // match T::write_raw(device, channel, _val) {
        //     Ok(_) => SensorValue::Int as ffi::c_int,
        //     // TODO can I return kernel errors here?
        //     Err(_) => EINVAL.to_errno(),
        // }
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
