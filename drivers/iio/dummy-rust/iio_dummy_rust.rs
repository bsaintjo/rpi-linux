//! Implementation of a dummy device driver for the industrial I/O subsystem in Rust
//!
//! The goal is to demonstrate the Rust abstractions
use kernel::{c_str, faux, iio, prelude::*, sync::Mutex};

module! {
    type: DummyModule,
    name: "rust_iio_dummy",
    authors: ["Brandon Saint-John"],
    description: "IIO dummy driver in Rust",
    license: "GPL",
}

struct DummyModule {
    _fdev: faux::Registration,
    _indio_dev: iio::Registration<DummyDevice>,
}

impl kernel::Module for DummyModule {
    fn init(module: &'static ThisModule) -> Result<Self> {
        let _fdev = faux::Registration::new(c_str!("rust-iio-dummy-faux"), None)?;
        let options = iio::RegistrationOptions {
            name: c_str!("rust-iio-dummy"),
            mode: iio::Mode::Direct,
        };
        let _indio_dev: iio::Registration<DummyDevice> =
            iio::Registration::new(_fdev.as_ref(), module, &options)?;
        Ok(Self { _fdev, _indio_dev })
    }
}

const DUMMY_CHANNELS: &'static [iio::Specification] =
    &[iio::Specification::new(iio::ChannelType::Voltage)
        .as_output()
        .info_mask_separate(iio::channels::RAW)];

struct DummyState {
    dac_val: i32,
}

impl Default for DummyState {
    fn default() -> Self {
        Self { dac_val: 0 }
    }
}

#[pin_data]
struct DummyDevice {
    #[pin]
    st: Mutex<DummyState>,
}

#[vtable]
impl iio::Driver for DummyDevice {
    type Data = DummyState;
    type Ptr = Pin<KBox<Mutex<Self::Data>>>;
    const CHANNELS: &'static [iio::Specification] = DUMMY_CHANNELS;

    fn read_raw(data: Pin<&Mutex<Self::Data>>, spec: &iio::Specification) -> iio::SensorResult<i32> {
        match spec.channel_type() {
            iio::ChannelType::Voltage => {
                let guard = data.lock();
                iio::SensorResult::int(guard.dac_val)
            }
            _ => todo!(),
        }
    }

    fn write_raw(data: Pin<&mut Mutex<Self::Data>>, spec: &iio::Specification, value: i32) -> Result {
        match spec.channel_type() {
            iio::ChannelType::Voltage if spec.is_differential() => {
                let mut guard = data.lock();
                guard.dac_val = value;
            }
            _ => todo!(),
        }
        // Let-chains version
        // if let iio::ChannelType::Voltage = spec.channel_type() && spec.is_differential() {
        //     let guard = data.st.lock();
        //     *guard.dac_val = value;
        // }
        Ok(())
    }
}
