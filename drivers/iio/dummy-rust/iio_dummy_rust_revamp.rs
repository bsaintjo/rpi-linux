#![allow(unused_variables)]
#![allow(dead_code)]
//! Implementation of a dummy device driver for the industrial I/O subsystem in Rust
//!
//! The goal is to demonstrate the Rust abstractions

use pin_init::pin_data;

use kernel::{
    c_str, faux,
    iio::{
        channels::{self, Buffered},
        ChannelType, SensorData, Specification,
    },
    try_pin_init,
    types::ARef,
};
use kernel::{iio::revamp, prelude::*};

module! {
    type: MyModule,
    name: "iio_dummy_rust_revamp",
    authors: ["Brandon Saint-John"],
    description: "IIO dummy driver in Rust",
    license: "GPL",
}

#[pin_data]
struct MyModule {
    faux: faux::Registration,
    #[pin]
    dev: revamp::Device<DevData>,
}

impl kernel::InPlaceModule for MyModule {
    fn init(module: &'static ThisModule) -> impl PinInit<Self, Error> {
        let faux = faux::Registration::new(c_str!("test"), None);
        let dev = {
            match faux {
                Ok(ref reg) => Ok(ARef::from(reg.as_ref())),
                Err(e) => Err(e),
            }
        };
        let options = revamp::RegistrationOptions {
            name: c_str!("test2"),
            modes: revamp::Mode::Direct,
        };
        try_pin_init!(Self {
            faux: faux?,
            dev <- revamp::Device::register3(dev?, module, options, DevData::init()),
        })
    }
}

#[pin_data]
struct DevData {
    x: i32,
}

impl DevData {
    fn init() -> impl PinInit<Self, Error> {
        try_pin_init!(Self { x: 6704 })
    }
}

const DUMMY_CHANNELS: &'static [Specification] = &[
    Specification::new(ChannelType::Voltage)
        .info_mask_separate(channels::RAW)
        .channel_idx(0),
    Specification::new(ChannelType::Voltage)
        .info_mask_separate(channels::RAW)
        .as_output()
        .channel_idx(1),
];

const DUMMY_BUFFERED_CHANNELS: &'static [Specification<Buffered>] =
    &[Specification::new_buffered(ChannelType::Voltage)];

impl revamp::Driver for DevData {
    const CHANNELS: &'static [channels::Channel] =
        &kernel::concat_channels!(DUMMY_CHANNELS, DUMMY_BUFFERED_CHANNELS);
    const USE_VTABLE_ATTR: () = ();

    type Data = DevData;

    fn read_raw(data: Pin<&Self::Data>, _channel: &Specification) -> Result<SensorData<i32>> {
        Ok(SensorData::int(data.x))
    }

    fn write_raw(
        mut data: Pin<&mut Self::Data>,
        spec: &Specification,
        _sdata: SensorData<i32>,
    ) -> Result {
        data.x = data.x.wrapping_add(1);
        Ok(())
    }
}
