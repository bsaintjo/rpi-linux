use kernel::{
    c_str, i2c, of,
    prelude::*,
    regmap::{self, BitFieldReadOps, BitFieldWriteOps, RawFieldWriteOps},
    iio,
    sync::{new_mutex, Arc, Mutex},
};
// use register::*;

kernel::module_i2c_driver! {
    type: Cm3232,
    name: "cm3232-rust",
    author: "Brandon Saint-John <saint-john@lbl.gov",
    license: "GPL",
}

kernel::i2c_device_table!(
    I2C_ID_TABLE,
    MODULE_I2C_ID_TABLE,
    <Cm3232 as i2c::Driver>::IdInfo,
    [(i2c::DeviceId::new(c_str!("cm3232")), ()),]
);

kernel::of_device_table!(
    OF_ID_TABLE,
    MODULE_OF_ID_TABLE,
    <Cm3232 as i2c::Driver>::IdInfo,
    [(of::DeviceId::new(c_str!("capella,cm3232")), ()),]
);

struct Cm3232AlsInfo {
    regs_cmd_default: u8,
    hw_id: u8,
    calibscale: i32,
    mlux_per_bit: i32,
    mlux_per_bit_base_it: i32,
}

struct Cm3232Chip {
    i2c_client: i2c::Client,
    cm3232_als_info: Cm3232AlsInfo,
    regs_cmd: u8,
    regs_als: u16,
}

struct Cm3232;

impl i2c::Driver for Cm3232 {
    type IdInfo = ();

    // const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = Some(&I2C_ID_TABLE);
    // const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_ID_TABLE);
    const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = None;
    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = None;

    fn probe(client: &mut i2c::Client, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        Err(todo!())
        // let config = regmap::Config::<AccessOps>::new(8, 8)
        //     .with_max_register(0x16)
        //     .with_cache_type(regmap::CacheType::RbTree);
        // let regmap = Arc::new(regmap::Regmap::init_i2c(client, &config)?, GFP_KERNEL)?;
        // let fields = regmap::Fields::new(&regmap, &FIELD_DESCS)?;

        // let data = Arc::pin_init(new_mutex!(Ncv6336RegulatorData { fields }), GFP_KERNEL)?;
        // let config = Config::new(client.as_ref(), data.clone()).with_regmap(regmap.clone());
        // let regulator = Device::register(client.as_ref(), &NCV6336_DESC, config)?;

        // let drvdata = KBox::new(Self(regulator), GFP_KERNEL)?;

        // Ok(drvdata.into())
    }
}

#[vtable]
impl iio::Driver<1> for Cm3232 {
    const CHANNELS: [iio::IioChanSpec; 1] = [
        iio::IioChanSpec,
    ];

    fn read_raw<T>(indio_dev: &mut iio::IioDevice<T>, _channel: &iio::IioChanSpec, _fst: u32, _snd: u32, _mask: u32) {
        todo!()
    }
    fn write_raw<T>(indio_dev: &mut iio::IioDevice<T>, _channel: &iio::IioChanSpec, _fst: u32, _snd: u32, _mask: u32) {
        todo!()
    }
}