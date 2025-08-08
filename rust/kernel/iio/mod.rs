#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(missing_docs)]
use core::{
    marker::PhantomData, mem::{self, MaybeUninit}, ops::Deref, ptr::{addr_of_mut, NonNull}
};

use crate::{
    device,
    error::{to_result, VTABLE_DEFAULT_ERROR},
    iio::{channels::{Channel, SensorValue, Simple}},
    prelude::*,
    str::CStr,
    types::ARef,
    ThisModule,
};

pub mod channels;
pub mod trigger;
// pub mod buffer;

use crate::iio::channels::{SensorData, Specification};

#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Device<T: Driver> {
    #[pin]
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

unsafe impl<T: Send + Sync + Driver> Send for Device<T> {}
unsafe impl<T: Send + Sync + Driver> Sync for Device<T> {}

pub struct DeviceRef(NonNull<bindings::iio_dev>);

impl DeviceRef {
    pub(crate) fn inner(&self) -> NonNull<bindings::iio_dev> {
        self.0
    }
}

impl<T: Driver> Device<T> {
    pub(crate) fn dev_ref(self: Pin<&mut Self>) -> DeviceRef {
        DeviceRef(self.indio_dev)
    }

    // Useful when state needs to be initialized based on the iio device
    // Such as with triggers
    fn register_with<F, P>(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        with_device: F,
    ) -> impl PinInit<Self, Error> + use<T, F, P>
    where
        F: FnOnce(DeviceRef) -> P,
        P: PinInit<T::Data, Error>,
    {
        let sizeof_priv = mem::size_of::<T::Data>();
        try_pin_init!(Self {
            indio_dev: NonNull::new(unsafe {
                bindings::iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
            })
            .ok_or(ENOMEM)?,
            _priv: PhantomData,
        })
        .pin_chain(|mut this| {
            let dev_ref = this.as_mut().dev_ref();
            let data = with_device(dev_ref);
            let private: *mut T::Data =
                unsafe { bindings::iio_priv(this.indio_dev.as_ptr()) } as *mut T::Data;

            // SAFETY:
            // - *iio_device_alloc succeeded, so private is guaranteed to be a pointer to unitialized memory
            // - The uninitialized memory is is guaranteed to fit T::Data
            // - TODO: private is aligned for DMA, is this still correct?
            unsafe {
                data.__pinned_init(private).inspect_err(|_| {
                    bindings::iio_device_free(this.indio_dev.as_mut());
                })?;
            }
            unsafe {
                addr_of_mut!((*this.indio_dev.as_ptr()).name).write(options.name.as_char_ptr());
                addr_of_mut!((*this.indio_dev.as_ptr()).modes).write(options.modes as i32);
                addr_of_mut!((*this.indio_dev.as_ptr()).channels)
                    .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
                addr_of_mut!((*this.indio_dev.as_ptr()).num_channels)
                    .write(T::CHANNELS.len() as ffi::c_int);
                addr_of_mut!((*this.indio_dev.as_ptr()).info)
                    .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
            }
            unsafe {
                to_result(bindings::__iio_device_register(
                    this.indio_dev.as_ptr(),
                    module.as_ptr(),
                ))
            }
        })
    }

    pub fn register(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        data: impl PinInit<T::Data, Error>,
    ) -> impl PinInit<Self, Error> {
        for chan in T::CHANNELS.iter() {
            pr_emerg!("Channel: {chan:?}");
        }
        Self::register_with(parent, module, options, |_| data)
        //     let sizeof_priv = mem::size_of::<T::Data>();
        //     try_pin_init!(Self {
        //         indio_dev: NonNull::new(unsafe {
        //             bindings::iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
        //         })
        //         .ok_or(ENOMEM)?,
        //         _priv: PhantomData,
        //     })
        //     .pin_chain(|mut this| {
        //         // Both of these might be valid, but iio_priv is used in the subsystem so maybe that is better
        //         // let private: *mut T::Data = unsafe { ptr::addr_of_mut!((*this.indio_dev.as_ptr()).priv_) } as *mut T::Data;
        //         // let ptr_uninit: *mut MaybeUninit<T::Data> = private.cast();
        //         // unsafe { (*ptr_uninit).write(data) };

        //         // Does this still violate Rust rules for UB and need to work in addr_of_mut somewhere
        //         let private: *mut T::Data =
        //             unsafe { bindings::iio_priv(this.indio_dev.as_ptr()) } as *mut T::Data;
        //         // SAFETY:
        //         // - *iio_device_alloc succeeded, so private is guaranteed to be a pointer to unitialized memory
        //         // - The uninitialized memory is is guaranteed to fit T::Data
        //         // - TODO: private is aligned for DMA, is this still correct?
        //         unsafe {
        //             data.__pinned_init(private).inspect_err(|_| {
        //                 bindings::iio_device_free(this.indio_dev.as_mut());
        //             })?;
        //         }

        //         unsafe {
        //             ptr::addr_of_mut!((*this.indio_dev.as_ptr()).name)
        //                 .write(options.name.as_char_ptr());
        //             ptr::addr_of_mut!((*this.indio_dev.as_ptr()).modes).write(options.modes as i32);
        //             ptr::addr_of_mut!((*this.indio_dev.as_ptr()).channels)
        //                 .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
        //             ptr::addr_of_mut!((*this.indio_dev.as_ptr()).num_channels)
        //                 .write(T::CHANNELS.len() as ffi::c_int);
        //             ptr::addr_of_mut!((*this.indio_dev.as_ptr()).info)
        //                 .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
        //         }
        //         unsafe {
        //             to_result(bindings::__iio_device_register(
        //                 this.indio_dev.as_ptr(),
        //                 module.as_ptr(),
        //             ))
        //         }
        //     })
        // }
    }

    pub fn data(self: Pin<&Self>) -> Pin<&T::Data> {
        let data: *const T::Data = unsafe { bindings::iio_priv(self.indio_dev.as_ptr()) }.cast();
        let data = unsafe { &*data };
        let data = unsafe { Pin::new_unchecked(data) };
        data
    }

    pub fn data_mut(self: Pin<&mut Self>) -> Pin<&mut T::Data> {
        let data: *mut T::Data = unsafe { bindings::iio_priv(self.indio_dev.as_ptr()) }.cast();
        let data = unsafe { &mut *data };
        let data = unsafe { Pin::new_unchecked(data) };
        data
    }

}

struct Claim<'a, T: Driver> {
    inner: Pin<&'a mut Device<T>>
    // indio_dev: *mut bindings::iio_dev,
    // phantom: PhantomData<&'a mut ()>
}

impl<'a, T: Driver> Claim<'a, T> {
    pub fn try_claim_direct(indio_dev: Pin<&'a mut Device<T>>) -> Result<Claim<'a, T>> {
        let res = unsafe { bindings::__iio_device_claim_direct(indio_dev.indio_dev.as_ptr())};
        if res {
            Ok(Self { inner: indio_dev })
        } else {
            Err(EBUSY)
        }
    }

    pub fn data(&self) -> Pin<&T::Data> {
        self.inner.as_ref().data()
    }

    pub fn data_mut(&mut self) -> Pin<&mut T::Data> {
        self.inner.as_mut().data_mut()
    }
}

// impl<'a, T: Driver> Deref for Claim<'a, T> {
//     type Target = Pin<&'a mut Device<T>>;

//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

impl<'a, T: Driver> Drop for Claim<'a, T> {
    fn drop(&mut self) {
        unsafe { bindings::__iio_device_release_direct(self.inner.indio_dev.as_ptr()) }
    }
}

#[pinned_drop]
impl<T: Driver> PinnedDrop for Device<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe { bindings::iio_device_unregister(self.indio_dev.as_ptr()) };
        unsafe { bindings::iio_device_free(self.indio_dev.as_ptr()) };
    }
}

pub struct RegistrationOptions {
    pub name: &'static CStr,
    pub modes: Mode,
}

#[repr(u32)]
pub enum Mode {
    Direct = bindings::INDIO_DIRECT_MODE,
}

#[vtable]
pub trait Driver: Sized {
    const CHANNELS: &'static [Channel];
    type Data: Send + Sync;

    fn read_raw(data: Pin<&Self::Data>, spec: &Specification) -> Result<SensorData> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(data: Pin<&mut Self::Data>, spec: &Specification, sdata: SensorData) -> Result {
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
        _mask: isize,
    ) -> ffi::c_int {
        pr_emerg!("Starting read_raw callback");
        let private: *mut T::Data = unsafe { bindings::iio_priv(indio_dev) } as *mut T::Data;
        let private: &T::Data = unsafe { &*private };
        let data = unsafe { Pin::new_unchecked(private) };

        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };

        match T::read_raw(data, channel) {
            Ok(sdata) => {
                pr_emerg!("read_raw successfully, sending data");
                match &sdata {
                    SensorData::Int(inner) => unsafe {
                        let val = val as *mut MaybeUninit<i32>;
                        (*val).write(*inner);
                    },
                }
                // unsafe {
                //     let val = val as *mut MaybeUninit<i32>;
                //     (*val).write(sdata.inner());
                // }
                // SensorData::<i32>::SENSOR_VALUE as ffi::c_int
                sdata.sensor_value() as ffi::c_int
            }
            Err(e) => e.to_errno(),
        }
    }
    unsafe extern "C" fn write_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: ffi::c_int,
        _val2: ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        pr_emerg!("Starting write_raw callback");
        let private: *mut T::Data = unsafe { bindings::iio_priv(indio_dev) } as *mut T::Data;
        let private: &mut T::Data = unsafe { &mut *private };
        let data = unsafe { Pin::new_unchecked(private) };
        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        let val = SensorData::int(val);

        pr_emerg!("val: {val:?}");

        // // TODO need to check the mask before casting
        match T::write_raw(data, channel, val) {
            Ok(_) => {
                pr_emerg!("Successfully write_raw");
                SensorValue::Int as ffi::c_int
            }
            // TODO can I return kernel errors here?
            Err(_) => {
                pr_emerg!("Failed write_raw");
                EINVAL.to_errno()
            }
        }
    }

    const VTABLE: bindings::iio_info = bindings::iio_info {
        read_raw: Some(Self::read_raw),
        // read_raw: if T::HAS_READ_RAW {
        //     Some(Self::read_raw)
        // } else {
        //     None
        // },
        write_raw: Some(Self::write_raw),
        // write_raw: if T::HAS_WRITE_RAW {
        //     Some(Self::write_raw)
        // } else {
        //     None
        // },
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    const fn build() -> &'static bindings::iio_info {
        &Self::VTABLE
    }
}
