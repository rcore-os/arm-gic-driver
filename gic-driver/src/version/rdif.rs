extern crate alloc;

use crate::fdt_parse_irq_config;

use rdif_intc::*;

impl DriverGeneric for super::v2::Gic {
    fn open(&mut self) -> Result<(), KError> {
        self.init();
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for super::v2::Gic {
    fn parse_dtb_fn(&self) -> Option<FuncFdtParseConfig> {
        Some(parse_dtb_fn)
    }
}

impl DriverGeneric for super::v3::Gic {
    fn open(&mut self) -> Result<(), KError> {
        self.init();
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for super::v3::Gic {
    fn parse_dtb_fn(&self) -> Option<FuncFdtParseConfig> {
        Some(parse_dtb_fn)
    }
}

fn parse_dtb_fn(itr: &[u32]) -> Result<IrqConfig, alloc::string::String> {
    let config = fdt_parse_irq_config(itr).map_err(alloc::string::String::from)?;
    Ok(config.into())
}

impl From<crate::define::IrqConfig> for IrqConfig {
    fn from(config: crate::define::IrqConfig) -> Self {
        IrqConfig {
            irq: (config.id.to_u32() as usize).into(),
            trigger: match config.trigger {
                crate::v2::Trigger::Edge => Trigger::EdgeRising,
                crate::v2::Trigger::Level => Trigger::LevelHigh,
            },
            is_private: config.id.is_private(),
        }
    }
}
