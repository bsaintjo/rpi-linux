use core::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
};

use crate::{
    device,
    error::VTABLE_DEFAULT_ERROR,
    iio::channels::{Channel, Sensor, Simple},
    prelude::*,
    str::CStr,
    types::{ARef, ForeignOwnable},
    ThisModule,
};

use crate::iio::channels::{SensorData, Specification};

// const IIO_DMA_MINALIGN: usize = {
//     const ARCH_ALIGN: usize = bindings::ARCH_DMA_MINALIGN as usize;
//     const SIZE: usize = core::mem::size_of::<ffi::c_long>() as usize;
//     if ARCH_ALIGN > SIZE {
//         ARCH_ALIGN
//     } else {
//         SIZE
//     }
// };

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
        try_pin_init!(Self {
            indio_dev: NonNull::new(unsafe { bindings::devm_iio_device_alloc(parent.as_raw(), 0) })
                .ok_or(ENOMEM)?,
            _priv: PhantomData,
        })
        .pin_chain(|mut this| {
            let dev_ref = this.as_mut().dev_ref();
            // let foo = KBox::pin_init(with_dev(this.into_ref()), GFP_KERNEL);
            let foo = KBox::pin_init(with_device(dev_ref), GFP_KERNEL)?;
            Ok(())
        })
        .pin_chain(|this: Pin<&mut Device<T>>| {
            // ptr::addr_of_mut!((*this.indio_dev.as_ptr()).priv_).write();
            // {
            //     let works = with_dev(understand);
            // }
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).info)
                    .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).channels)
                    .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).num_channels)
                    .write(T::CHANNELS.len() as i32);
            };
            Ok(())
        })
    }

    pub fn register(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        data: impl PinInit<T::Data, Error>,
    ) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            indio_dev: NonNull::new(unsafe { bindings::devm_iio_device_alloc(parent.as_raw(), 0) })
                .ok_or(ENOMEM)?,
            _priv: PhantomData,
        })
        .pin_chain(|this| {
            // ptr::addr_of_mut!((*this.indio_dev.as_ptr()).priv_).write();
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).info)
                    .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
            }
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).channels)
                    .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec)
            };
            todo!()
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

    // This registration allocates T::Data directly in line with the indio_dev
    pub fn register3(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        data: impl PinInit<T::Data, Error>,
    ) -> impl PinInit<Self, Error> {
        let sizeof_priv = mem::size_of::<T::Data>();
        try_pin_init!(Self {
            indio_dev: NonNull::new(unsafe {
                bindings::devm_iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
            })
            .ok_or(ENOMEM)?,
            _priv: PhantomData,
        })
        .pin_chain(|mut this| {
            // Both of these might be valid, but iio_priv is used in the subsystem so maybe that is better
            // let private: *mut T::Data = unsafe { ptr::addr_of_mut!((*this.indio_dev.as_ptr()).priv_) } as *mut T::Data;
            // let ptr_uninit: *mut MaybeUninit<T::Data> = private.cast();
            // unsafe { (*ptr_uninit).write(data) };

            // Does this still violate Rust rules for UB and need to work in addr_of_mut somewhere
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
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).info)
                    .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
            }
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).channels)
                    .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).num_channels)
                    .write(T::CHANNELS.len() as ffi::c_int)
            }
            todo!()
        })
    }

    pub fn register2(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        data: T::Data,
    ) -> impl PinInit<Self, Error> {
        let sizeof_priv = mem::size_of::<T::Data>();
        try_pin_init!(Self {
            indio_dev: NonNull::new(unsafe {
                bindings::devm_iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
            })
            .ok_or(ENOMEM)?,
            _priv: PhantomData,
        })
        .pin_chain(|this| {
            // SAFETY: devm_iio_device_alloc is guaranteed to allocate a memory equal to or greater than
            // let private: *mut T::Data = unsafe { (*this.indio_dev.as_ptr()).priv_ } as *mut T::Data;
            let private: *mut T::Data =
                unsafe { bindings::iio_priv(this.indio_dev.as_ptr()) } as *mut T::Data;
            let ptr_uninit: *mut MaybeUninit<T::Data> = private.cast();
            unsafe { (*ptr_uninit).write(data) };
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).info)
                    .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
            }
            unsafe {
                ptr::addr_of_mut!((*this.indio_dev.as_ptr()).channels)
                    .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec)
            };
            todo!()
        })
    }
}

#[pinned_drop]
impl<T: Driver> PinnedDrop for Device<T> {
    fn drop(self: Pin<&mut Self>) {
        // Need to free the device, but before that,
        // we need to grab out Self::Ptr from _priv and deallocate it on our side
        // Set _priv to null(?)
        // and free the device
        // unsafe { bindings::iio_device_unregister(self.indio_dev.get()) };
        todo!()
    }
}

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
    type Data: Send + Sync;
    type Ptr: ForeignOwnable + Send + Sync;

    fn read_raw2(data: Pin<&Self::Data>);

    fn read_raw(
        _data: <Self::Ptr as ForeignOwnable>::Borrowed<'_>,
        _channel: &Specification,
    ) -> Result<SensorData<i32>> {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(
        _data: <Self::Ptr as ForeignOwnable>::BorrowedMut<'_>,
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
        _mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        let private = unsafe { &raw mut (*indio_dev).priv_ }.cast();
        let device = unsafe { <T::Ptr as ForeignOwnable>::borrow(private) };

        // // TODO need to check the mask before casting
        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };
        match T::read_raw(device, channel) {
            Ok(sdata) => {
                unsafe {
                    let val = val as *mut MaybeUninit<i32>;
                    (*val).write(sdata.inner());
                }
                SensorData::<i32>::SENSOR_VALUE as ffi::c_int
            }
            Err(e) => e.to_errno(),
        }

        // // Safety: val out-pointer maybe uninitialized
    }
    unsafe extern "C" fn write_raw(
        _indio_dev: *mut bindings::iio_dev,
        _iio_chan_spec: *const bindings::iio_chan_spec,
        _val: ffi::c_int,
        _val2: ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        // Copied from kernel::miscdevice
        let private = unsafe { &raw mut (*_indio_dev).priv_ }.cast();
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
    use pin_init::pin_data;

    use kernel::prelude::*;
    use kernel::{c_str, faux, try_pin_init, types::ARef};

    use crate::{
        iio::{
            revamp::{self, Device, DeviceRef, Driver},
            trigger::Trigger2,
            SensorData, Specification,
        },
        types::ForeignOwnable,
    };

    #[pin_data]
    struct MyModule {
        faux: faux::Registration,
        #[pin]
        dev: revamp::Device<DevData>,
    }

    impl kernel::InPlaceModule for MyModule {
        fn init(module: &'static ThisModule) -> impl PinInit<Self, Error> {
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
                mode: kernel::iio::revamp::Mode::Direct,
            };
            try_pin_init!(Self {
                faux: faux?,
                // dev <- revamp::Device::register_with(dev?, module, options, |dev| DevData::init(dev)),
                dev <- revamp::Device::register_with(dev?, module, options, |dev| DevData::init3(dev)),
                // dev <- revamp::Device::register_with(dev?, module, options, |dev| {
                //     let t = dev.with_trigger();
                //     try_pin_init!(DevData { x: 10, trigger <- t})
                // }),
                // dev: todo!(),
            })
        }
    }

    #[pin_data]
    struct DevData {
        x: i32,
        #[pin]
        trigger: Trigger2,
    }

    impl DevData {
        fn init4(indio_dev: DeviceRef) -> Result<Self> {
            Ok(Self {
                x: 10,
                trigger: Trigger2::new2(&indio_dev)?,
            })
        }

        fn init3(indio_dev: DeviceRef) -> impl PinInit<Self, Error> {
            try_pin_init!(Self {
                x: 10,
                trigger <- Trigger2::new_dev(&indio_dev)
            })
        }

        fn init<T: Driver>(indio_dev: Pin<&Device<T>>) -> Result<Pin<KBox<Self>>> {
            Box::try_pin_init(
                try_pin_init!(Self {
                    x: 10,
                    trigger <- Trigger2::new_pinned(indio_dev)
                    // trigger: todo!()
                }),
                GFP_KERNEL,
            )
        }

        // Potential footgun here:
        // if you directly try to directly initialize with trigger <- Trigger2::new(indio_dev)
        // The compiler tries to infer that Trigger2::new captures the lifetime of 'a from indio_dev
        // indio_dev Pin<&'a Device<T>>
        fn init2<T: Driver>(indio_dev: Pin<&Device<T>>) -> impl PinInit<Self, Error> {
            let t = Trigger2::new_pinned_ref(indio_dev.get_ref());
            try_pin_init!(Self {
                x: 10,
                // trigger <- Trigger2::new_pinned(indio_dev),
                // trigger <- Trigger2::new_pinned(indio_dev.get_ref()),
                trigger <- t
            })
        }
    }

    impl Driver for DevData {
        const CHANNELS: &'static [super::Channel] = &[] as &'static [super::Channel];
        const USE_VTABLE_ATTR: () = ();

        type Ptr = Pin<KBox<DevData>>;
        type Data = DevData;

        fn read_raw(
            data: <Self::Ptr as ForeignOwnable>::Borrowed<'_>,
            _channel: &Specification,
        ) -> Result<SensorData<i32>> {
            Ok(SensorData::int(data.x))
        }

        fn read_raw2(data: Pin<&Self::Data>) {
            todo!()
        }
    }

    #[pin_data]
    struct Foo {
        #[pin]
        inner: Bar,
    }

    #[pin_data]
    struct Bar {}

    impl Bar {
        fn pinned<T>(x: Pin<&T>) -> impl PinInit<Self, Error> {
            try_pin_init!(Bar {})
        }
    }

    impl Foo {
        fn foo<T>(x: Pin<&T>) -> impl PinInit<Self, Error> + '_ {
            // let inner = Bar::pinned(x);
            try_pin_init!(Self {
                inner <- Bar::pinned(x),
                // inner <- inner
            })
        }
    }
}
