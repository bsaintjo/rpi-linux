use core::{
    alloc::Layout,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
};

use crate::{
    alloc::Allocator,
    device,
    error::{from_err_ptr, to_result, VTABLE_DEFAULT_ERROR},
    iio::channels::{Buffered, Channel, Simple},
    prelude::*,
    str::CStr,
    types::{ARef, AlwaysRefCounted, ForeignOwnable, Opaque},
    ThisModule,
};

use crate::iio::channels::{ChannelType, SensorData, SensorValue, Specification};

// const IIO_DMA_MINALIGN: usize = {
//     const ARCH_ALIGN: usize = bindings::ARCH_DMA_MINALIGN as usize;
//     const SIZE: usize = core::mem::size_of::<ffi::c_long>() as usize;
//     if ARCH_ALIGN > SIZE {
//         ARCH_ALIGN
//     } else {
//         SIZE
//     }
// };

#[derive(Zeroable)]
#[repr(transparent)]
#[pin_data]
pub struct Device<T: Driver> {
    #[pin]
    indio_dev: Opaque<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

unsafe impl<T: Send + Sync + Driver> Send for Device<T> {}
unsafe impl<T: Send + Sync + Driver> Sync for Device<T> {}

impl<T: Driver> Device<T> {
    pub fn register<'a>(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
    ) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            indio_dev <- Opaque::try_ffi_init(move |slot: *mut bindings::iio_dev| {
                // indio_device_alloc takes a parent device and a size_of
                // argument of private data we want to store along side the
                // IIO device.
                // Internally, if the sizeof is greater than zero
                // it allocates space for the iio_dev and the private data,
                // and ensures the alignment is suitable for DMA
                // otherwise, it just allocates for the iio_dev
                // This private pointer is expected to not be accessed
                // by anyone else.
                // To make use of ForeignOwnable, we initialize the iio_dev
                // with no space for the private data, allocate it on our side
                // and make the private pointer _priv point to our new data.
                let indio_dev: *mut bindings::iio_dev = unsafe {
                    bindings::iio_device_alloc(parent.as_raw(), 0)
                };
                if indio_dev.is_null() {
                    return Err(ENOMEM);
                }
                unsafe { *slot = *indio_dev };
                // Safety: slot should still be ok for initialization
                unsafe { (*slot).priv_ = T::init()?.into_foreign().cast() };
                // let private: *mut T = unsafe { bindings::iio_priv(slot) }.cast();
                // unsafe { private.write(data) };
                // unsafe { data.__pinned_init(private) } if data is type impl PinInit
                // Ok::<(), Error>(())
                Ok(())
            }),
            _priv: PhantomData,
        })

        // let indio_dev: *mut bindings::iio_dev = unsafe {
        //     bindings::iio_device_alloc(parent.as_raw(), mem::size_of::<T::Data>() as i32)
        // };
        // let indio_dev = NonNull::new(indio_dev).ok_or(ENOMEM)?;
        // // TODO: iio_priv gives us the pointer to the iio_dev member, so we aren't violating aliasing rules?
        // let private: *mut T::Data = unsafe { bindings::iio_priv(indio_dev.as_ptr())}.cast();
        // unsafe { data.__pinned_init(private) }.inspect_err(|_| {
        //     // if iio_device_alloc was successful, then we should be ok?
        //     todo!("Understand errors")
        // });
        // unsafe { ptr::addr_of_mut!((*indio_dev.as_ptr()).info).write(IioVTableAdapter::<T>::build() as *const bindings::iio_info) ;}
        // unsafe { ptr::addr_of_mut!((*indio_dev.as_ptr()).channels).write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec) };
        // let indio_dev: *mut Device<T> = NonNull::new(indio_dev.as_ptr().cast());

        // try_pin_init!(
        //     Self {
        //     indio_dev <- Opaque::try_ffi_init(move |slot: *mut bindings::iio_dev| {
        //         let sizeof_priv = core::mem::size_of::<T>() as ffi::c_int;
        //         unsafe {
        //             let dev = bindings::devm_iio_device_alloc(dev.as_raw(), sizeof_priv);
        //             if dev.is_null() {
        //                 return Err(ENOMEM);
        //             }
        //             *slot = *dev;
        //         }
        //         unsafe {
        //             (*slot).name = options.name.as_char_ptr();
        //             // (*slot).priv_ = T::alloc()?.into_foreign().cast();
        //             (*slot).channels = T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec;
        //             (*slot).num_channels = T::CHANNELS.len() as i32;
        //             (*slot).modes = Mode::Direct as i32;
        //             (*slot).info = IioVTableAdapter::<T>::build() as *const bindings::iio_info;
        //             // (*slot).dev.parent = dev.as_raw();
        //         }
        //         to_result(unsafe { indings::__devm_iio_device_register(dev.as_raw(), slot, module.as_ptr()) })
        //     }),
        //     _priv: PhantomData,
        // })
    }
}

unsafe impl<T: Driver> AlwaysRefCounted for Device<T> {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        // unsafe { bindings::drm_dev_get(self.as_raw()) };
        todo!()
    }

    unsafe fn dec_ref(obj: NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        // unsafe { bindings::drm_dev_put(obj.cast().as_ptr()) };
        todo!()
    }
}

// #[pinned_drop]
// impl<T> PinnedDrop for BetterRegistration<T> {
//     fn drop(self: Pin<&mut Self>) {
//         unsafe { bindings::iio_device_unregister(self.indio_dev.get()) };
//     }
// }

// unsafe impl<T: Send + Sync> Send for Device<T> {}
// unsafe impl<T: Send + Sync> Sync for Device<T> {}

// TODO Sealed for now, figure out if necessary
// impl<T: Send + Sync> crate::private::Sealed for Device<T> {}

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
    const CHANNELS: &'static [Channel];
    type Ptr: ForeignOwnable + Send + Sync;

    fn init() -> Result<Self::Ptr>;

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
        let private = unsafe { (*indio_dev).priv_ }.cast();
        let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        let device = unsafe { <T::Ptr as ForeignOwnable>::borrow(private) };

        // // TODO need to check the mask before casting
        // let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        // let ret = T::read_raw(device, channel);

        // // Safety: val out-pointer maybe uninitialized
        // unsafe { let val = val as *mut MaybeUninit<i32>; }
        // unsafe { val.write(ret.inner()); }
        // SensorData::SENSOR_VALUE as ffi::c_int
        todo!()
    }
    unsafe extern "C" fn write_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: ffi::c_int,
        val2: ffi::c_int,
        mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        let private = unsafe { (*indio_dev).priv_ }.cast();
        let ptr = unsafe { <T::Ptr as ForeignOwnable>::from_foreign(private) };
        let device = unsafe { <T::Ptr as ForeignOwnable>::borrow_mut(private) };

        // // TODO need to check the mask before casting
        // let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        // match T::write_raw(device, channel, val) {
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

mod type_test {
    use core::ptr::NonNull;

    use pin_init::pin_data;

    use crate::prelude::*;
    use crate::{c_str, faux, try_pin_init, types::ARef};

    #[pin_data]
    struct MyModule {
        faux: faux::Registration,
        #[pin]
        dev: super::Device<Data>,
    }

    impl kernel::InPlaceModule for MyModule {
        fn init(
            module: &'static crate::ThisModule,
        ) -> impl pin_init::PinInit<Self, crate::error::Error> {
            // let dev = ARef::from(faux.as_ref());
            let faux = faux::Registration::new(c_str!("test"), None);
            let dev = {
                match faux {
                    Ok(ref reg) => Ok(ARef::from(reg.as_ref())),
                    Err(e) => Err(e),
                }
            };
            let options = super::RegistrationOptions {
                name: c_str!("test2"),
                mode: crate::iio::revamp::Mode::Direct,
            };
            try_pin_init!(Self {
                faux: faux?,
                dev <- super::Device::register(dev?, module, options),
            })
        }
    }

    struct Data {
        x: i32,
    }

    impl super::Driver for Data {
        const CHANNELS: &'static [super::Channel] = &[] as &'static [super::Channel];
        const USE_VTABLE_ATTR: () = ();

        type Ptr = Pin<KBox<Data>>;

        fn init() -> Result<Self::Ptr> {
            KBox::pin_init(init!(Data { x: 10 }), GFP_KERNEL)
        }
    }
}
