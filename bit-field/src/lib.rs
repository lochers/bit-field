#![no_std]
pub use bit_field_macro::bitfield;

pub trait Integer {
    const SIZE: usize;
}

macro_rules! impl_integer {
    ($type:ty, $val:expr) => {
        impl Integer for $type {
            const SIZE: usize = $val;
        }
    }
}

impl_integer!(u8  , 8);
impl_integer!(u16 , 16);
impl_integer!(u32 , 32);
impl_integer!(u64 , 64);
impl_integer!(u128, 128);
impl_integer!(i8  , 8);
impl_integer!(i16 , 16);
impl_integer!(i32 , 32);
impl_integer!(i64 , 64);
impl_integer!(i128, 128);

pub fn can_fit<I: Integer>(width: usize) -> bool {
    width < I::SIZE
}

pub trait BitField {
    type Ux;

    fn into_inner(self) -> Self::Ux;
}

