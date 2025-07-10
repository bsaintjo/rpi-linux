use core::{any::Any, marker::PhantomData, mem::MaybeUninit};

use crate::iio::buffer::BufferChannel;

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
#[derive(Copy, Clone, Default)]
pub struct Specification<T = Simple> {
    spec: bindings::iio_chan_spec,
    _phantom: PhantomData<T>
}

#[derive(Copy, Clone, Default)]
pub struct Simple;

#[derive(Copy, Clone, Default)]
pub struct Buffered;

#[derive(Copy, Clone, Default)]
pub struct Channel {
    inner: bindings::iio_chan_spec
}

impl<T> Specification<T> {
    pub const fn as_channel(&'static self) -> Channel {
        unsafe { Channel { inner: self.spec }}
    }
}

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

impl Specification<Buffered> {
    // TODO Can iio_chan_spec initialized to zero?
    pub const fn new_buffered(channel_type: ChannelType) -> Self {
        unsafe {
            Specification {
                spec: bindings::iio_chan_spec {
                type_: channel_type as ffi::c_uint,
                // scan_index: T::SCAN_TYPE.scan_index,
                scan_index: 7,
                ..MaybeUninit::zeroed().assume_init()
            },
                _phantom: PhantomData, 
            }
        }
    }
}

#[macro_export]
macro_rules! concat_channels {
    ($a:expr, $b:expr) => {{
        let _a: &'static [$crate::iio::Specification<$crate::iio::channels::Simple>] = $a;
        let _b: &'static [$crate::iio::Specification<$crate::iio::channels::Buffered>] = $b;
        const LEN_A: usize = $a.len();
        const LEN_B: usize = $b.len();
        const LEN: usize = LEN_A + LEN_B;

        let mut result = unsafe { [core::mem::zeroed(); LEN] };
        let mut i = 0;
        while i < LEN_A {
            result[i] = $a[i].as_channel();
            i += 1;
        }
        let mut j = 0;
        while j < LEN_B {
            result[LEN_A + j] = $b[j].as_channel();
            j += 1;
        }
        result
    }};
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

pub struct SensorData<T> {
    value: T,
}

impl<T> SensorData<T> {
    pub(crate) fn inner(self) -> T {
        self.value
    }
}

impl SensorData<i32> {
    pub const SENSOR_VALUE: SensorValue = SensorValue::Int;
    pub fn int(value: i32) -> Self {
        Self { value }
    }
}

/// Represents the return type
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum SensorValue {
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
    // TODO: From a talk a few years ago, it seemed like count sensors are moved out
    // So maybe don't allow for implmenting count channels
    // https://www.youtube.com/watch?v=644oH1FXdtE, 24:54, ABI 'mistakes'
    // Need to check on the status, and add the deprecated attribute until confirmed
    #[deprecated]
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