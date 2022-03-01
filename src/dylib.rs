use mach_o_sys::dyld::{
    NSCreateObjectFileImageFromMemory, 
    NSLinkModule, 
    NSUnLinkModule, 
    NSLookupSymbolInModule, 
    NSAddressOfSymbol, 
    NSModule, 
    NSObjectFileImageReturnCode, 
    __NSSymbol};
use core::mem::MaybeUninit;
use std::ffi::{c_void, CString};
use core::convert::AsRef;
use thiserror::Error;


/// BundleLoadError describes the various errors generated during BUNDLE/DYLIB loading.
#[derive(Error, Debug)]
pub enum BundleLoadError {
    #[error("Error converting byte array to NSObject")]
    InvalidNSObject,

    #[error("Symbol does not exist within the NSObject")]
    SymbolResolutionFailure,

    #[error("Symbol resolved but the address lookup failed")]
    SymbolAddressFailure
}
/// Contains a linked dylib bundle
#[derive(Debug)]
pub struct BundleLibrary<'a> {
    _module: &'a [u8;1],
    _memory: &'a mut [u8],
    _handle: NSModule
}

impl<'a> BundleLibrary<'a> {
    /// Instantiate a dylib/bundle file in memory using NSLinkModule.
    /// A dylib is converted to a 'bundle' type by patching the memory of the buffer to
    /// represent it as a bundle file. This allows it to be loaded.
    pub fn new(data: &'a mut [u8]) -> Result<Self, BundleLoadError> {
        data[12] = 0x8;
        let object_file_image = &mut MaybeUninit::uninit();
        let cstring = b"\x00";

        let module = unsafe {
            // This will return an error if failure ocurrs
            if let NSObjectFileImageReturnCode::NSObjectFileImageSuccess = NSCreateObjectFileImageFromMemory(
                data.as_ptr() as *const _,
                data.len(),
                object_file_image.as_mut_ptr()) {
                    NSLinkModule(
                        object_file_image.assume_init(),
                        cstring.as_ptr() as _,
                        0)  
                }
            else {
                return Err(BundleLoadError::InvalidNSObject)
            }
            
            
        };
        Ok(BundleLibrary{
            _module: cstring,
            _memory: data,
            _handle: module
        })

    }

    /// Fetch a symbol from the loaded in-memory library. This can be used to confirm
    /// the presence of the symbol. 
    pub fn get_symbol<T: AsRef<str>>(&self, symbol: T) -> Result<&mut __NSSymbol, BundleLoadError> {
        let symbol_name = CString::new(symbol.as_ref()).unwrap();
        let symbol_obj = unsafe {
            NSLookupSymbolInModule(
                self._handle,
                symbol_name.as_ptr())
        };
        return unsafe{
            symbol_obj.as_mut().ok_or(BundleLoadError::SymbolAddressFailure)
        }

    }

    /// Fetches the address of the symbol in the loaded module. 
    pub fn get_symbol_address<T: AsRef<str>>(&self, symbol: T) -> Result<&mut c_void, BundleLoadError> {
        let raw_symbol = self.get_symbol(symbol)?;
        let symbol_address = unsafe {
            NSAddressOfSymbol(raw_symbol)
        };

        if symbol_address as u32 == 0x0u32 {
            return Err(BundleLoadError::SymbolResolutionFailure)
        }
        return unsafe{ symbol_address.as_mut().ok_or(BundleLoadError::SymbolResolutionFailure)};
    }

    
}

impl<'a> Drop for BundleLibrary<'a> {
    fn drop(&mut self) {
        unsafe{
            NSUnLinkModule(self._handle, 0);
        }
    }
}
