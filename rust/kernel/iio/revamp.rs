use core::{
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::{self, addr_of_mut, NonNull},
};

use crate::{
    device,
    error::{to_result, VTABLE_DEFAULT_ERROR},
    iio::{
        channels::{Channel, Simple},
        SensorValue,
    },
    prelude::*,
    str::CStr,
    types::ARef,
    ThisModule,
};

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
                addr_of_mut!((*this.indio_dev.as_ptr()).name)
                    .write(options.name.as_char_ptr());
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

    fn write_raw(
        data: Pin<&mut Self::Data>,
        spec: &Specification,
        sdata: SensorData,
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
                    }
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

mod type_test {
    use pin_init::pin_data;

    use kernel::prelude::*;
    use kernel::{c_str, faux, try_pin_init, types::ARef};

    use crate::iio::{
        revamp::{self, Device, DeviceRef, Driver},
        trigger::Trigger2,
        SensorData, Specification,
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
                modes: revamp::Mode::Direct,
            };
            try_pin_init!(Self {
                faux: faux?,
                // dev <- revamp::Device::register_with(dev?, module, options, |dev| DevData::init(dev)),
                dev <- revamp::Device::register_with(dev?, module, options, |dev| DevData::init3(dev)),
                // dev <- revamp::Device::register4(dev?, module, options, DevData::init3(todo!())),
                // dev <- revamp::Device::register_with2(dev?, module, options, |dev| DevData::init2(dev)),
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
        fn init2(indio_dev: Pin<&Device<Self>>) -> impl PinInit<Self, Error> + use<'_> {
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

        type Data = DevData;

        // fn read_raw(
        //     data: <Self::Ptr as ForeignOwnable>::Borrowed<'_>,
        //     _channel: &Specification,
        // ) -> Result<SensorData<i32>> {
        //     Ok(SensorData::int(data.x))
        // }

        fn read_raw(data: Pin<&Self::Data>, spec: &Specification) -> Result<SensorData> {
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
