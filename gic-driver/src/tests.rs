extern crate std;
use crate::{
    IntId,
    define::nmi_attr_slot,
    version::v3::{LPI, RedistributorV3, RedistributorV4, SGI},
};

#[test]
fn size_lpi() {
    let size = size_of::<LPI>();
    assert_eq!(size, 0x10000);
}

#[test]
fn size_sgi() {
    assert_eq!(size_of::<SGI>(), 0x10000);
}

#[test]
fn test_v3_rd() {
    let size = size_of::<RedistributorV3>();
    assert_eq!(size, 0x20000);
}

#[test]
fn test_v4_rd() {
    let size = size_of::<RedistributorV4>();
    assert_eq!(size, 0x40000);
}

#[test]
#[should_panic]
fn test_sgi() {
    let id = IntId::sgi(40);
    assert_eq!(id.is_sgi(), true);
}

#[test]
#[should_panic]
fn test_ppi() {
    let id = IntId::ppi(17);
    assert_eq!(id.is_private(), true);
}

#[test]
fn nmi_attr_slot_sgi() {
    let (reg, bit) = nmi_attr_slot(IntId::sgi(1)).unwrap();
    assert_eq!(reg, 0);
    assert_eq!(bit, 1 << 1); // GICR_INMIR0 bit 1
}

#[test]
fn nmi_attr_slot_rejects_special_intid() {
    assert!(nmi_attr_slot(unsafe { IntId::raw(1023) }).is_err());
}

#[test]
fn nmi_attr_slot_ppi() {
    let (reg, bit) = nmi_attr_slot(IntId::ppi(14)).unwrap();
    assert_eq!(reg, 0);
    assert_eq!(bit, 1 << 14);
}

#[test]
fn nmi_attr_slot_spi() {
    let (reg, bit) = nmi_attr_slot(IntId::spi(42)).unwrap();
    assert_eq!(reg, 2); // INTID 74 -> GICD_INMIR[2]
    assert_eq!(bit, 1 << 10);
}
