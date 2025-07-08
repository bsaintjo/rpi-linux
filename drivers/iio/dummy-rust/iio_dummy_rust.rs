use kernel::faux;

use kernel::c_str;
use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::iio;

module! {
    type: IioDummyModule,
    name: "rust_iio_dummy",
    authors: ["Brandon Saint-John"],
    description: "IIO dummy driver in Rust",
    license: "GPL",
}

struct IioDummyModule {
    _fdev: faux::Registration,
    indio_dev: iio::Registration<EmptyState>,
}

impl kernel::Module for IioDummyModule {
    fn init(module: &'static ThisModule) -> Result<Self> {
        let _fdev = faux::Registration::new(c_str!("rust-iio-dummy-faux"), None)?;
        let indio_dev: iio::Registration<EmptyState> = iio::Registration::new(c_str!("rust-iio-dummy"), _fdev.as_ref(), module)?;
        Ok(Self {
            _fdev,
            indio_dev,
        })
    }
}

struct EmptyState;

#[vtable]
impl iio::Driver for EmptyState {
    const CHANNELS: &'static [iio::IioChanSpec] = &[];
}


struct IioDummyState {
    dac_val: i32,
    single_ended_adc_val: i32,
    differential_adc_val: [i32; 2],
    accel_val: i32,
    accel_calibbias: i32,
    activity_running: i32,
    activity_walking: i32,
    // accel_calib_scale
    steps_enabled: i32,
    steps: i32,
    height: i32,
}

#[pin_data]
struct IioDummy {
    #[pin]
    st: Mutex<IioDummyState>,
}

// impl iio::Driver for Dummy {
//     fn read_raw(indio_dev: &mut Device) {
//         todo!()
//     }

//     fn write_raw(indio_dev: &mut Device) {
//         todo!()
//     }
// }
