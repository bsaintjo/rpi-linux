use core::{marker::PhantomData, mem::MaybeUninit};

use crate::iio::buffer::BufferChannel;

struct Channels(&'static [Specification]);

impl Channels {
    const fn as_raw(self) -> *const bindings::iio_chan_spec {
        todo!()
    }
}

// TODO: Explore a more thickly wrapped Specification on top of this one
// This way, we can use pattern matching to make the Rust side more ergonomic
// Idea:
// struct Channel {
//   pub something: i32,
//   pub to_: &'static CStr,
//   pub match: bool,
//   pub onto: Option<u64>
//   spec: Specification,
// }
// then in read_raw/etc...
// fn read_raw(channel: &Channel, /* args */) {
//     // Example with let chains
//     if ChannelType::Voltage = chan.spec.channel_type() && let Some(x) = chan.onto && x > 12 {
//          /* Rust fun */
//     }
// }

#[repr(transparent)]
pub struct Specification<T = Simple> {
    spec: bindings::iio_chan_spec,
    _phantom: PhantomData<T>
}

pub struct Simple;

// Feature? Automatically infer return type via channel type?
impl Specification<Simple> {
    pub const fn new(channel_type: ChannelType) -> Self {
        // TODO Can iio_chan_spec initialized to zero?
        unsafe {
            Specification {
                spec: bindings::iio_chan_spec {
                type_: channel_type as ffi::c_uint,
                scan_index: -1,
                ..MaybeUninit::zeroed().assume_init()
            },
                _phantom: PhantomData, 
            }
        }
    }

    pub fn channel_type(&self) -> ChannelType {
        unsafe { core::mem::transmute(self.spec.type_) }
    }

    // TODO pull from actual specification
    pub fn is_differential(&self) -> bool {
        true
    }


    pub const fn info_mask_separate(mut self, mask: Mask) -> Self {
        self.spec.info_mask_separate = mask.0 as isize;
        self
    }

    // TODO: Figure out a better way to do this
    // .output is a bitfield member of iio_chan_spec
    // bindgen outputs a helper set_output to make is easy to set
    // however, it implements it using generic functions and isn't const
    // The below is copying out the necessary parts from bindgen so I can set
    // output in a const context
    pub const fn as_output(mut self) -> Self {
        let bit_offset = 2usize;
        let bit_width = 1u8;
        let index = if cfg!(target_endian = "big") {
            bit_offset - 1
        } else {
            bit_width as usize + bit_offset
        };
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };

        let mask: u8 = 1 << bit_index;
        self.spec._bitfield_1 = bindings::__BindgenBitfieldUnit::new([mask; 1]);
        self
    }
}

impl<T: BufferChannel> Specification<T> {
    // TODO Can iio_chan_spec initialized to zero?
    fn new(channel_type: ChannelType) -> Self {
        unsafe {
            Specification {
                spec: bindings::iio_chan_spec {
                type_: channel_type as ffi::c_uint,
                scan_index: T::SCAN_TYPE.scan_index,
                ..MaybeUninit::zeroed().assume_init()
            },
                _phantom: PhantomData, 
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Mask(u32);

impl core::ops::BitOr for Mask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

pub const RAW: Mask = Mask(bindings::iio_chan_info_enum_IIO_CHAN_INFO_RAW);
pub const OFFSET: Mask = Mask(bindings::iio_chan_info_enum_IIO_CHAN_INFO_OFFSET);
pub const PROCESSED: Mask = Mask(bindings::iio_chan_info_enum_IIO_CHAN_INFO_PROCESSED);
pub const CALIBSCALE: Mask = Mask(bindings::iio_chan_info_enum_IIO_CHAN_INFO_CALIBSCALE);
pub const INT_TIME: Mask = Mask(bindings::iio_chan_info_enum_IIO_CHAN_INFO_INT_TIME);

pub struct SensorResult<T> {
    value: T,
}

impl<T> SensorResult<T> {
    pub(crate) fn inner(self) -> T {
        self.value
    }
}

impl SensorResult<i32> {
    pub const VALUE_TYPE: IIOValue = IIOValue::Int;
    pub fn int(value: i32) -> Self {
        Self { value }
    }
}

/// Represents the return type
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum IIOValue {
    Int = bindings::IIO_VAL_INT,
    PlusMicro = bindings::IIO_VAL_INT_PLUS_MICRO,
    PlusNano = bindings::IIO_VAL_INT_PLUS_NANO,
    PlusMicroDb = bindings::IIO_VAL_INT_PLUS_MICRO_DB,
    IntMultiple = bindings::IIO_VAL_INT_MULTIPLE,
    Int64 = bindings::IIO_VAL_INT_64,
    Fractional = bindings::IIO_VAL_FRACTIONAL,
    FractionalLog2 = bindings::IIO_VAL_FRACTIONAL_LOG2,
    Char = bindings::IIO_VAL_CHAR,
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum ChannelType {
    Voltage = bindings::iio_chan_type_IIO_VOLTAGE,
    Current = bindings::iio_chan_type_IIO_CURRENT,
    Power = bindings::iio_chan_type_IIO_POWER,
    Accel = bindings::iio_chan_type_IIO_ACCEL,
    AnglVel = bindings::iio_chan_type_IIO_ANGL_VEL,
    Magn = bindings::iio_chan_type_IIO_MAGN,
    Light = bindings::iio_chan_type_IIO_LIGHT,
    Intensity = bindings::iio_chan_type_IIO_INTENSITY,
    Proximity = bindings::iio_chan_type_IIO_PROXIMITY,
    Temp = bindings::iio_chan_type_IIO_TEMP,
    Incli = bindings::iio_chan_type_IIO_INCLI,
    Rot = bindings::iio_chan_type_IIO_ROT,
    Angl = bindings::iio_chan_type_IIO_ANGL,
    Timestamp = bindings::iio_chan_type_IIO_TIMESTAMP,
    Capacitance = bindings::iio_chan_type_IIO_CAPACITANCE,
    AltVoltage = bindings::iio_chan_type_IIO_ALTVOLTAGE,
    Cct = bindings::iio_chan_type_IIO_CCT,
    Pressure = bindings::iio_chan_type_IIO_PRESSURE,
    HumidityRelative = bindings::iio_chan_type_IIO_HUMIDITYRELATIVE,
    Activity = bindings::iio_chan_type_IIO_ACTIVITY,
    Steps = bindings::iio_chan_type_IIO_STEPS,
    Energy = bindings::iio_chan_type_IIO_ENERGY,
    Distance = bindings::iio_chan_type_IIO_DISTANCE,
    Velocity = bindings::iio_chan_type_IIO_VELOCITY,
    Concentration = bindings::iio_chan_type_IIO_CONCENTRATION,
    Resistance = bindings::iio_chan_type_IIO_RESISTANCE,
    Ph = bindings::iio_chan_type_IIO_PH,
    UvIndex = bindings::iio_chan_type_IIO_UVINDEX,
    ElectricalConductivity = bindings::iio_chan_type_IIO_ELECTRICALCONDUCTIVITY,
    Count = bindings::iio_chan_type_IIO_COUNT,
    Index = bindings::iio_chan_type_IIO_INDEX,
    Gravity = bindings::iio_chan_type_IIO_GRAVITY,
    PositionRelative = bindings::iio_chan_type_IIO_POSITIONRELATIVE,
    Phase = bindings::iio_chan_type_IIO_PHASE,
    MassConcentration = bindings::iio_chan_type_IIO_MASSCONCENTRATION,
    DeltaAngl = bindings::iio_chan_type_IIO_DELTA_ANGL,
    DeltaVelocit = bindings::iio_chan_type_IIO_DELTA_VELOCITY,
    ColorTemp = bindings::iio_chan_type_IIO_COLORTEMP,
    Chromaticity = bindings::iio_chan_type_IIO_CHROMATICITY,
    Attention = bindings::iio_chan_type_IIO_ATTENTION,
}