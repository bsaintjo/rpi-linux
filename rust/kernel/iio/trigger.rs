use crate::{c_str, iio::{self, revamp::{Device, DeviceRef, Driver}}, prelude::*};
use core::marker::PhantomData;

#[pin_data]
pub struct Trigger2 {}

impl Trigger2 {
    pub fn new<T: Driver>(
        indio_dev: Pin<&Device<T>>
    ) -> Result<Self> {
        Ok(Trigger2 {})
    }

    pub fn new2(
        indio_dev: &DeviceRef
    ) -> Result<Self> {
        Ok(Trigger2 {})
    }

    pub fn new_pinned<'a, T: Driver>(
        indio_dev: Pin<&'a Device<T>>
    ) -> impl PinInit<Self, Error> + use<T> {
        try_pin_init!(Trigger2 {})
    }

    pub fn new_pinned_ref<'a, T: Driver>(
        indio_dev: &'a Device<T>
    ) -> impl PinInit<Self, Error> + use<T> {
        try_pin_init!(Trigger2 {})
    }

    pub fn new_dev(indio_dev: &DeviceRef) -> impl PinInit<Self, Error> + use<> {
        let _ = indio_dev.inner();
        try_pin_init!(Trigger2 {
        })
    }
}

struct Trigger;

impl Trigger {
    fn new<I>(
        _indio_dev: iio::Registration<I>,
        _module: &'static ThisModule,
        _name: &'static CStr,
        _idx: i32,
    ) -> Result<Self>
    where
        I: iio::Driver,
    {
        let trig = unsafe {
            bindings::__devm_iio_trigger_alloc(
                _indio_dev.device().as_raw(),
                _module.0,
                c_str!("trig-%s-%d").as_char_ptr(),
                _name.as_char_ptr(),
                _idx,
            )
        };
        if trig.is_null() {
            return Err(EINVAL);
        }
        // unsafe {
        //     (*trig).ops = TriggerVtable::<Self>::build() as *const bindings::iio_trigger_ops;
        // }
        // let ret = unsafe { bindings::devm_iio_trigger_register(indio_dev.device().as_raw(), trig) };
        // if ret < 0 {
        //     // TODO "Do I need to free on failure or the devm cleansup?"
        //     todo!()
        // }
        todo!()
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        todo!()
    }
}

#[vtable]
impl TriggerOps for Trigger {
    fn set_trigger_state() {
        todo!()
    }

    fn reenable() {
        todo!()
    }

    fn validate_device() {
        todo!()
    }
}

#[vtable]
trait TriggerOps {
    fn set_trigger_state();
    fn reenable();
    fn validate_device();
}

struct TriggerVtable<T: TriggerOps>(PhantomData<T>);

impl<T: TriggerOps> TriggerVtable<T> {
    unsafe extern "C" fn set_trigger_state(
        _trig: *mut bindings::iio_trigger,
        _state: bool,
    ) -> ffi::c_int {
        todo!()
    }
    unsafe extern "C" fn try_reenable(_trig: *mut bindings::iio_trigger) {
        todo!()
    }
    unsafe extern "C" fn validate_device(
        _trig: *mut bindings::iio_trigger,
        _indio_dev: *mut bindings::iio_dev,
    ) -> ffi::c_int {
        todo!()
    }

    const VTABLE: bindings::iio_trigger_ops = bindings::iio_trigger_ops {
        set_trigger_state: Some(Self::set_trigger_state),
        reenable: if T::HAS_REENABLE {
            Some(Self::try_reenable)
        } else {
            None
        },
        validate_device: Some(Self::validate_device),
    };

    const fn build() -> &'static bindings::iio_trigger_ops {
        &Self::VTABLE
    }
}
