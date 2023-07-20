//! Contains the controller interface

use embedded_hal::{
    blocking::{delay::*, spi::Transfer, spi::Write},
    digital::v2::{InputPin, OutputPin},
};

/// Interface Error
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// A error in the spi driver
    SpiError,
    /// A error in the gpio driver
    GPIOError,
}

/// Trait to describe the interface with the controller
/// The controller supports different hardware interfaces like i2c, usb, spi and i80
pub trait IT8951Interface {
    /// active wait while the controller is busy and no new transactions should be issued
    fn wait_while_busy(&mut self) -> Result<(), Error>;

    /// write a 16bit value to the controller
    fn write_data(&mut self, data: u16) -> Result<(), Error>;

    /// write mutliple 16bit values to the controller
    fn write_multi_data(&mut self, data: &[u16]) -> Result<(), Error>;

    /// issue a command on the controller
    fn write_command(&mut self, cmd: u16) -> Result<(), Error>;

    /// issue a command with arguments on the controller
    fn write_command_with_args(&mut self, cmd: u16, args: &[u16]) -> Result<(), Error> {
        self.write_command(cmd)?;
        for arg in args {
            self.write_data(*arg)?;
        }
        Ok(())
    }

    /// read a single 16 bit value
    fn read_data(&mut self) -> Result<u16, Error>;

    /// read multiple 16bit values
    fn read_multi_data(&mut self, buf: &mut [u16]) -> Result<(), Error>;

    /// reset the controller
    fn reset(&mut self) -> Result<(), Error>;
}

/// Implements the controller interface for the spi hardware interface
/// Uses embedded_hal spi and gpio driver and a embedded_hal delay driver
pub struct IT8951SPIInterface<SPI, BUSY, RST, DELAY> {
    spi: SPI,
    busy: BUSY,
    rst: RST,
    delay: DELAY,
}

impl<SPI, BUSY, RST, DELAY> IT8951SPIInterface<SPI, BUSY, RST, DELAY>
where
    SPI: Write<u8> + Transfer<u8>,
    BUSY: InputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    /// Create a new spi controller interface
    pub fn new(
        spi: SPI,
        busy: BUSY,
        rst: RST,
        delay: DELAY,
    ) -> IT8951SPIInterface<SPI, BUSY, RST, DELAY> {
        IT8951SPIInterface {
            spi,
            busy,
            rst,
            delay,
        }
    }
}

impl<SPI, BUSY, RST, DELAY> IT8951Interface for IT8951SPIInterface<SPI, BUSY, RST, DELAY>
where
    SPI: Write<u8> + Transfer<u8>,
    BUSY: InputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn wait_while_busy(&mut self) -> Result<(), Error> {
        while self.busy.is_low().unwrap_or(true) {}
        Ok(())
    }

    fn write_data(&mut self, data: u16) -> Result<(), Error> {
        self.wait_while_busy()?;

        // Write Data:
        // 0x0000 -> Prefix for a Data Write
        // data; u16 -> 16bit data to write
        let buf = [0x00, 0x00, (data >> 8) as u8, data as u8];

        if self.spi.write(&buf).is_err() {
            return Err(Error::SpiError);
        }

        Ok(())
    }

    fn write_multi_data(&mut self, data: &[u16]) -> Result<(), Error> {
        self.wait_while_busy()?;

        // Write Data:
        // 0x0000 -> Prefix for a Data Write
        let mut buf = vec![0u8; data.len()*2 + 2 /*write data prefix */];

        for index in 0..data.len() {
            buf[index * 2 + 2] = (data[index] >> 8) as u8;
            buf[index * 2 + 2 + 1] = data[index] as u8;
        }

        if self.spi.write(&buf).is_err() {
            return Err(Error::SpiError);
        }

        Ok(())
    }

    fn write_command(&mut self, cmd: u16) -> Result<(), Error> {
        self.wait_while_busy()?;

        // Write Command:
        // 0x6000 -> Prefix for a Command
        // cmd; u16 -> 16bit Command code
        let buf = [0x60, 0x00, (cmd >> 8) as u8, cmd as u8];

        if self.spi.write(&buf).is_err() {
            return Err(Error::SpiError);
        }
        Ok(())
    }

    fn read_data(&mut self) -> Result<u16, Error> {
        self.wait_while_busy()?;

        // Read Data
        // 0x1000 -> Prefix for Read Data
        let mut buf = [0x10, 0x00, 0x00, 0x00, 0x00, 0x00];
        if self.spi.transfer(&mut buf).is_err() {
            return Err(Error::SpiError);
        }
        // we skip the first 2 bytes -> shifted out while transfer the prefix
        // the next two bytes are only dummies and are skipped to
        // only the last two bytes are the expected data and are stored
        Ok(u16::from_be_bytes([buf[4], buf[5]]))
    }

    fn read_multi_data(&mut self, buf: &mut [u16]) -> Result<(), Error> {
        self.wait_while_busy()?;
        // create a u8 buffer
        let mut read_buf = vec![0u8; buf.len()*2 /* nbr of data bytes */ + 2 /*dummby bytes */ + 2 /* read preamble */];

        // 0x1000 prefix for read data
        read_buf[0] = 0x10;
        read_buf[1] = 0x00;

        if self.spi.transfer(&mut read_buf).is_err() {
            return Err(Error::SpiError);
        }

        // we skip the first 2 bytes -> shifted out while transfer the prefix
        // the next two bytes are only dummies and are skipped to
        const OFFSET: usize = 4;
        for index in 0..buf.len() {
            buf[index] = u16::from_be_bytes([
                read_buf[OFFSET + index * 2],
                read_buf[OFFSET + index * 2 + 1],
            ]);
        }

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Error> {
        if self.rst.set_high().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(200);
        if self.rst.set_low().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(20);
        if self.rst.set_high().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(200);
        Ok(())
    }
}
