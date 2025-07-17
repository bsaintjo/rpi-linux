#![allow(unused_variables)]
#![allow(dead_code)]
//! Implementation of a dummy device driver for the industrial I/O subsystem in Rust
//!
//! The goal is to demonstrate the Rust abstractions
use kernel::{c_str, faux, iio::{self, channels::Buffered}, new_mutex, prelude::*, sync::Mutex};

module! {
    type: DummyModule,
    name: "iio_dummy_rust",
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
        let _fdev = faux::Registration::new(c_str!("iio_dummy_rust_faux"), None)?;
        let options = iio::RegistrationOptions {
            name: c_str!("iio_dummy_rust"),
            mode: iio::Mode::Direct,
        };
        let _indio_dev: iio::Registration<DummyDevice> =
            iio::Registration::new(_fdev.as_ref(), module, &options, DummyDevice::new()?)?;
        Ok(Self { _fdev, _indio_dev })
    }
}

const DUMMY_CHANNELS: &'static [iio::Specification] =
    &[iio::Specification::new(iio::ChannelType::Voltage)
        .as_output()
        .info_mask_separate(iio::channels::RAW)];

const DUMMY_BUFFERED_CHANNELS: &'static [iio::Specification<Buffered>] =
    &[iio::Specification::new_buffered(iio::ChannelType::Voltage)];

#[pin_data]
struct DummyState {
    dac_val: i32,
}

impl Default for DummyState {
    fn default() -> Self {
        Self { dac_val: 0 }
    }
}

impl DummyState {
    fn new() -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            dac_val: 10
        })
    }
}

#[pin_data]
struct DummyDevice {
    #[pin]
    st: Mutex<DummyState>,
}

impl DummyDevice {
    fn new() -> Result<Pin<KBox<Self>>> {
        KBox::pin_init(pin_init!(Self {
            st <- new_mutex!(DummyState::default())
        }), GFP_KERNEL)
    }
}

#[vtable]
impl iio::Driver for DummyDevice {
    // type Data = Pin<KBox<DummyState>>;
    type Ptr = Pin<KBox<DummyDevice>>;
    const CHANNELS: &'static [iio::channels::Channel] = &kernel::concat_channels!(DUMMY_CHANNELS, DUMMY_BUFFERED_CHANNELS);

    fn read_raw(data: Pin<&Self>, spec: &iio::Specification) -> iio::SensorData<i32> {
        // match spec.channel_type() {
        //     iio::ChannelType::Voltage => {
        //         let guard = data.lock();
        //         iio::SensorData::int(guard.dac_val)
        //     }
        //     _ => todo!(),
        // }
        todo!()
    }

    fn write_raw(data: Pin<&mut Self>, spec: &iio::Specification, value: i32) -> Result {
        // match spec.channel_type() {
        //     iio::ChannelType::Voltage if spec.is_differential() => {
        //         let mut guard = data.lock();
        //         guard.dac_val = value;
        //     }
        //     _ => todo!(),
        // }
        // Let-chains version
        // if let iio::ChannelType::Voltage = spec.channel_type() && spec.is_differential() {
        //     let guard = data.st.lock();
        //     *guard.dac_val = value;
        // }
        Ok(())
    }
}

// use pin_init::pin_data;

// use kernel::prelude::*;
// use kernel::{c_str, faux, try_pin_init, types::ARef};

// module! {
//     type: MyModule,
//     name: "rust_iio_dummy",
//     authors: ["Brandon Saint-John"],
//     description: "IIO dummy driver in Rust",
//     license: "GPL",
// }

// #[pin_data]
// struct MyModule {
//     faux: faux::Registration,
//     #[pin]
//     dev: kernel::iio::revamp::Device<Data>,
// }

// impl kernel::InPlaceModule for MyModule {
//     fn init(
//         module: &'static kernel::ThisModule,
//     ) -> impl pin_init::PinInit<Self, kernel::error::Error> {
//         // let dev = ARef::from(faux.as_ref());
//         let faux = faux::Registration::new(c_str!("test"), None);
//         let dev = {
//             match faux {
//                 Ok(ref reg) => Ok(ARef::from(reg.as_ref())),
//                 Err(e) => Err(e),
//             }
//         };
//         let options = kernel::iio::revamp::RegistrationOptions {
//             name: c_str!("test2"),
//             mode: kernel::iio::revamp::Mode::Direct,
//         };
//         try_pin_init!(Self {
//             faux: faux?,
//             dev <- kernel::iio::revamp::Device::register(dev?, module, options),
//         })
//     }
// }

// struct Data {
//     x: i32,
// }
// const DUMMY_CHANNELS: &'static [kernel::iio::Specification] =
//     &[kernel::iio::Specification::new(kernel::iio::ChannelType::Voltage)
//         .as_output()
//         .info_mask_separate(kernel::iio::channels::RAW)];

// const DUMMY_BUFFERED_CHANNELS: &'static [kernel::iio::Specification<kernel::iio::channels::Buffered>] =
//     &[kernel::iio::Specification::new_buffered(kernel::iio::ChannelType::Voltage)];

// impl kernel::iio::revamp::Driver for Data {
//     const CHANNELS: &'static [kernel::iio::channels::Channel] = &kernel::concat_channels!(DUMMY_CHANNELS, DUMMY_BUFFERED_CHANNELS);
//     const USE_VTABLE_ATTR: () = ();

//     type Ptr = Pin<KBox<Data>>;

//     fn init() -> Result<Self::Ptr> {
//         KBox::pin_init(init!(Data { x: 10 }), GFP_KERNEL)
//     }

//     fn read_raw(
//         data: <Self::Ptr as kernel::types::ForeignOwnable>::Borrowed<'_>,
//         _channel: &kernel::iio::Specification,
//     ) -> Result<kernel::iio::SensorData<i32>> {
//         Ok(kernel::iio::channels::SensorData::int(data.x))
//     }
// }
