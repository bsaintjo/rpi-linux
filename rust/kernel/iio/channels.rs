use core::{
    mem::MaybeUninit,
};

use crate::{prelude::*, str::CStr, device::Device, error::VTABLE_DEFAULT_ERROR, ThisModule};

struct Channels(&'static [Specification]);

impl Channels {
    const fn as_raw(self) -> *const bindings::iio_chan_spec {
        todo!()
    }
}

#[repr(transparent)]
pub struct Specification(bindings::iio_chan_spec);

// Feature? Automatically infer return type via channel type?
impl Specification {
    pub const fn new(channel_type: ChannelType) -> Self {
        // TODO Can iio_chan_spec initialized to zero?
        unsafe { Specification(bindings::iio_chan_spec {
            type_: channel_type as ffi::c_uint,
            ..MaybeUninit::zeroed().assume_init()
        }) }
    }

    const fn panic_test(mut self) -> Self {
        panic!("Will this work");
        self
    }

    // pub const fn indexed(mut self, idx: u32) -> Self {
    //     self.0.indexed = idx;
    //     self
    // }

    pub const fn info_mask_separate(mut self, mask: Mask) -> Self {
        self.0.info_mask_separate = mask.0 as isize;
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Mask(u32);

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
