use kernel::faux;

use kernel::prelude::*;
use kernel::c_str;
use kernel::sync::Mutex;
use kernel::iio::IioDevice;

module! {
    type: IioDummyModule,
    name: "rust_iio_dummy",
    authors: ["Brandon Saint-John"],
    description: "IIO dummy driver in Rust",
    license: "GPL",
}

struct IioDummyModule {
    _faux_reg: faux::Registration,
    indio_dev: IioDevice<()>,
}

impl kernel::Module for IioDummyModule {
    fn init(module: &'static ThisModule) -> Result<Self> {
        let _faux_reg = faux::Registration::new(c_str!("rust-iio-dummy"), None)?;
        let indio_dev: IioDevice<()> = IioDevice::register(_faux_reg.as_ref(), module)?;
        Ok(Self { _faux_reg, indio_dev })
    }
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