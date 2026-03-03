pub trait Bitmask {
    fn bitmask(&self) -> u32;
}

impl<T> Bitmask for [T]
where
    T: Bitmask,
{
    fn bitmask(&self) -> u32 {
        let mut mask = 0u32;
        for item in self {
            let value: u32 = item.bitmask();
            mask |= value;
        }
        mask
    }
}
