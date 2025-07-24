#![allow(unused_variables)]
#![allow(dead_code)]
//! Implementation of a dummy device driver for the industrial I/O subsystem in Rust
//!
//! The goal is to demonstrate the Rust abstractions

use pin_init::pin_data;

use kernel::{
    c_str, faux,
    iio::{
        channels::{self, Buffered, ChannelDefinition},
        ChannelType, SensorData, Specification,
    },
    new_mutex,
    sync::Mutex,
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
            dev <- revamp::Device::register(dev?, module, options, DevData::init()),
        })
    }
}

#[pin_data]
struct DevData {
    #[pin]
    x: Mutex<i32>,
}

impl DevData {
    fn init() -> impl PinInit<Self, Error> {
        try_pin_init!(Self { x <- new_mutex!(6704) })
    }
}

const DUMMY_INDEX_VOLTAGE_0: i32 = 7;

const DUMMY_CHANNELS: &'static [Specification] = &[
    Specification::new(ChannelType::Voltage)
        .info_mask_separate(channels::RAW.or(channels::OFFSET).or(channels::SCALE))
        .scan_index(DUMMY_INDEX_VOLTAGE_0)
        .channel_idx(0),
    Specification::new(ChannelType::Voltage)
        .info_mask_separate(channels::RAW)
        .channel_idx(0)
        .as_output(),
];

const DUMMY_BUFFERED_CHANNELS: &'static [Specification<Buffered>] =
    // &[Specification::new_buffered(ChannelType::Voltage)];
    &[];

impl revamp::Driver for DevData {
    const CHANNELS: &'static [channels::Channel] =
        &kernel::concat_channels!(DUMMY_CHANNELS, DUMMY_BUFFERED_CHANNELS);
    const USE_VTABLE_ATTR: () = ();

    type Data = DevData;

    fn read_raw(data: Pin<&Self::Data>, channel: &Specification) -> Result<SensorData<i32>> {
        match channel.definition() {
            ChannelDefinition { output: false, .. } => {
                let guard = data.x.lock();
                Ok(SensorData::int(*guard))
            }
            _ => Err(EINVAL),
        }
    }

    fn write_raw(
        data: Pin<&mut Self::Data>,
        channel: &Specification,
        _sdata: SensorData<i32>,
    ) -> Result {
        match channel.definition() {
            ChannelDefinition { output: true, .. } => {
                let mut guard = data.x.lock();
                *guard += 1;
                Ok(())
            }
            _ => Err(EINVAL),
        }
    }
}
