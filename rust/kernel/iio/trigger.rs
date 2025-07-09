use crate::{c_str, iio, prelude::*};
use core::marker::PhantomData;

struct Trigger;

impl Trigger {
    fn new<I>(
        indio_dev: iio::Registration<I>,
        module: &'static ThisModule,
        name: &'static CStr,
        idx: i32,
    ) -> Result<Self>
    where
        I: iio::Driver,
    {
        let trig = unsafe {
            bindings::__devm_iio_trigger_alloc(
                indio_dev.device().as_raw(),
                module.0,
                c_str!("trig-%s-%d").as_char_ptr() as *mut ffi::c_char,
                name.as_char_ptr(),
                idx,
            )
        };
        if trig.is_null() {
            return Err(EINVAL);
        }
        unsafe {
            (*trig).ops = TriggerVtable::<Self>::build() as *const bindings::iio_trigger_ops;
        }
        let ret = unsafe { bindings::devm_iio_trigger_register(indio_dev.device().as_raw(), trig) };
        if ret < 0 {
            // TODO "Do I need to free on failure or the devm cleansup?"
            todo!()
        }
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
        trig: *mut bindings::iio_trigger,
        state: bool,
    ) -> ffi::c_int {
        todo!()
    }
    unsafe extern "C" fn try_reenable(trig: *mut bindings::iio_trigger) {
        todo!()
    }
    unsafe extern "C" fn validate_device(
        trig: *mut bindings::iio_trigger,
        indio_dev: *mut bindings::iio_dev,
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
