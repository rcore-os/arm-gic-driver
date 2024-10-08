extern crate std;
use crate::version::v3::{LPI, SGI};

#[test]
fn test_size() {
    let size = size_of::<LPI>();
    assert_eq!(size, 0x10000);

    assert_eq!(size_of::<SGI>(), 0x10000);
}
