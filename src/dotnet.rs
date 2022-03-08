
use winapi::shared::guiddef::*;
use winapi::shared::ntdef::*;
use winapi::shared::minwindef::*;
use core::ptr;
use winapi::um::libloaderapi::*;
use core::mem::transmute;
use mscoree_sys_2::metahost::*;
use mscoree_sys_2::mscoree::*;
use mscorlib_sys::system::*;
use winapi::Interface;
use winapi::um::oaidl::*;
use winapi::um::oleauto::*;
use winapi::shared::wtypes::*;
use winapi::ctypes::c_void;
use winapi::ctypes::c_long;
use oaidl::SafeArrayExt;
use widestring::U16CString;

#[allow(dead_code)]
#[link(name="OleAut32")]
extern "system" {
     fn SafeArrayCreate(vt: VARTYPE, cDims: UINT, rgsabound: LPSAFEARRAYBOUND) -> LPSAFEARRAY;
	 fn SafeArrayDestroy(safe: LPSAFEARRAY)->HRESULT;
    
     fn SafeArrayGetDim(psa: LPSAFEARRAY) -> UINT;
	
     fn SafeArrayGetElement(psa: LPSAFEARRAY, rgIndices: *const c_long, pv: *mut c_void) -> HRESULT;
     fn SafeArrayGetElemSize(psa: LPSAFEARRAY) -> UINT;
    
     fn SafeArrayGetLBound(psa: LPSAFEARRAY, nDim: UINT, plLbound: *mut c_long)->HRESULT;
     fn SafeArrayGetUBound(psa: LPSAFEARRAY, nDim: UINT, plUbound: *mut c_long)->HRESULT;
    
     fn SafeArrayGetVartype(psa: LPSAFEARRAY, pvt: *mut VARTYPE) -> HRESULT;

     fn SafeArrayLock(psa: LPSAFEARRAY) -> HRESULT;
	 fn SafeArrayUnlock(psa: LPSAFEARRAY) -> HRESULT;
    
     fn SafeArrayPutElement(psa: LPSAFEARRAY, rgIndices: *const c_long, pv: *mut c_void) -> HRESULT;
}

pub type ClrCreateInstance = extern "system" fn( 
    clsid: *const CLSID,  
    riid:  *const IID,  
    ppInterface: LPVOID   
) -> HRESULT; 

#[derive(Debug, Clone)]
pub struct CLRInstance {
    meta_host: *mut ICLRMetaHost,
    runtime_info: *mut ICLRRuntimeInfo,
    runtime_host: *mut ICLRRuntimeHost,
    cor_runtime_host: *mut ICorRuntimeHost
}

#[derive(Debug, Clone)]
pub enum CLRLoadError {
    LoadLibraryError,
    LoadFunctionError,
    CreateClrInstanceError,
    GetRuntimeError,
    GetClrInterfaceError,
    GetCorInterfaceError,
    StartError
}

impl CLRInstance {
    pub unsafe fn load() -> Result<Self, CLRLoadError> {
        let mut clr =  CLRInstance {
            meta_host: ptr::null_mut(),
            runtime_info: ptr::null_mut(),
            runtime_host: ptr::null_mut(),
            cor_runtime_host: ptr::null_mut()
        };
        
        let mscoree = LoadLibraryA(b"mscoree.dll\x00".as_ptr() as *mut i8);
        if mscoree.is_null() { return Err(CLRLoadError::LoadLibraryError); }
        let clr_create_instance = GetProcAddress(mscoree, b"CreateInterface\x00".as_ptr() as *mut i8);
        if clr_create_instance.is_null(){return Err(CLRLoadError::LoadFunctionError);}
        
        let clr_create_instance: ClrCreateInstance = transmute(clr_create_instance);
        let instance = clr_create_instance(
            &CLSID_CLRMetaHost,
            &IID_ICLRMetaHost,
            &mut clr.meta_host as *mut _ as *mut _
        );
        if instance != 0 {
            return Err(CLRLoadError::CreateClrInstanceError);
        }
        let host_version = obfstr::wide!("v4.0.30319\x00");
        let runtime = (*clr.meta_host).GetRuntime(host_version.as_ptr(), &IID_ICLRRuntimeInfo, &mut clr.runtime_info as *mut _ as *mut _);
        if runtime != 0 {
            return Err(CLRLoadError::GetRuntimeError);
        }
        let host = (*clr.runtime_info).GetInterface(&CLSID_CLRRuntimeHost, &IID_ICLRRuntimeHost, &mut clr.runtime_host as *mut _ as *mut _);
        if host != 0 {
            return Err(CLRLoadError::GetClrInterfaceError);
        }

        (*clr.runtime_host).Start();

        let cor_host = (*clr.runtime_info).GetInterface(&CLSID_CorRuntimeHost, &IID_ICorRuntimeHost, &mut clr.cor_runtime_host as *mut _ as *mut _);
        if cor_host != 0 {
            return Err(CLRLoadError::GetCorInterfaceError);
        }
       
        return Ok(clr);
    }
    

    pub unsafe fn get_appdomain(&mut self) -> AppDomain {
        //dbg!(cor_host)
        let mut default_domain = ptr::null_mut();
        let _domain = (*self.cor_runtime_host).GetDefaultDomain(&mut default_domain);
        //let cor_host = (*clr.cor_runtime_host).GetInterface(&CLSID_CorRuntimeHost, &IID_ICorRuntimeHost, &mut clr.cor_runtime_host as *mut _ as *mut _);
        let mut defappdomain: *mut _AppDomain = ptr::null_mut();
        let _hr = (*default_domain).QueryInterface(&_AppDomain::uuidof(), &mut defappdomain as *mut _ as *mut _);
        
        return AppDomain{
            appdomain: defappdomain
        }
        //let mut _bstr = ptr::null_mut();
        //(*_type).ToString_(&mut _bstr);
        //dbg!(widestring::ucstring::UCString::from_ptr_str(_bstr));
    }
}

pub struct AppDomain {
    appdomain: *mut _AppDomain
}

impl AppDomain {
    pub unsafe fn _oldload(&self, module: &str) {
        let mut assembly =  widestring::U16CString::from_str(module).unwrap();
        let mut retval: *mut mscorlib_sys::system::reflection::_Assembly = ptr::null_mut();
        let _hr = (*self.appdomain).Load_2(assembly.as_mut_ptr(), &mut retval as *mut _);
        // COR_E_MARSHALDIRECTIVE currently throwing
        //let mut _bstr = ptr::null_mut();
        //(*retval).ToString_(&mut _bstr);
        //dbg!(widestring::ucstring::UCString::from_ptr_str(_bstr));
    }

    pub unsafe fn load_assembly(&self, module: &[u8]) -> () {
        let mut p_assembly: *mut mscorlib_sys::system::reflection::_Assembly  = ptr::null_mut();
        let psafe = module.iter().map(|&x|x).into_safearray().unwrap();
        let hr = (*self.appdomain).Load_3(psafe.as_ptr(), &mut p_assembly as *mut _);
        dbg!(hr);
        let mut output = ptr::null_mut();
        let mut s_output = core::mem::zeroed();
        dbg!((*p_assembly).get_EntryPoint(&mut output));
    
        let mut _params = SafeArrayCreateVector(VT_VARIANT as u16, 0, 1);


        let mut vtpsa: winapi::um::oaidl::__tagVARIANT = core::mem::zeroed();
        vtpsa.vt = VT_ARRAY as u16 | VT_BSTR as u16;
        *(vtpsa.n3.parray_mut()) = SafeArrayCreateVector(VT_BSTR as _, 0, 1);
        let param = SysAllocString(obfstr::wide!("audit\x00").as_ptr());
        SafeArrayPutElement(*(vtpsa.n3.parray_mut()), &0, param as *mut _);
        
        SafeArrayPutElement(_params, &0, &mut vtpsa as *mut _ as *mut _);

        let obj: oaidl::Ptr<VARIANT> = oaidl::VariantExt::into_variant(VT_NULL).unwrap();
        //dbg!(obj);
        dbg!((*output).Invoke_3(*(obj.as_ptr()), _params, &mut s_output));

    }
    pub unsafe fn load(&self, module: &[u8]) -> Assembly {
        let mut p_assembly: *mut mscorlib_sys::system::reflection::_Assembly  = ptr::null_mut();
        let psafe = module.iter().map(|&x|x).into_safearray().unwrap();
        let _hr = (*self.appdomain).Load_3(psafe.as_ptr(), &mut p_assembly as *mut _);
        Assembly(p_assembly)
    }
}

pub struct Assembly (*mut mscorlib_sys::system::reflection::_Assembly);

impl Assembly {
    pub unsafe fn invoke_entry(&mut self, args: Option<&[&str]>) -> Result<(), String> {
        let mut entry = ptr::null_mut();
        let hr = (*self.0).get_EntryPoint(&mut entry);
        if hr != 0 {
            return Err(String::from("Entrypoint failed"));
        }
        if let Some(arg_list) = args {
            let mut _params = SafeArrayCreateVector(VT_VARIANT as u16, 0, 1);

            let mut vtpsa: winapi::um::oaidl::__tagVARIANT = core::mem::zeroed();
            vtpsa.vt = VT_ARRAY as u16 | VT_BSTR as u16;
            *(vtpsa.n3.parray_mut()) = SafeArrayCreateVector(VT_BSTR as _, 0, arg_list.len() as u32);
            
            for (i, v) in arg_list.iter().enumerate(){
                let mut var = i as i32;
                let widestring_conversion_parameter = U16CString::from_str(v).unwrap();
                let param = SysAllocString(widestring_conversion_parameter.as_ptr());
                SafeArrayPutElement(*(vtpsa.n3.parray_mut()), &mut var, param as *mut _);
            }
            SafeArrayPutElement(_params, &0, &mut vtpsa as *mut _ as *mut _);

            let obj: oaidl::Ptr<VARIANT> = oaidl::VariantExt::into_variant(VT_NULL).unwrap();
            //dbg!(obj);
            let mut s_output = core::mem::zeroed();
            dbg!((*entry).Invoke_3(*(obj.as_ptr()), _params, &mut s_output));    
        } 
        Ok(())


    }
}


#[test]
pub fn loadtest() {
    let mut clr = unsafe{CLRInstance::load().unwrap()};
    let appdomain = unsafe{clr.get_appdomain()};
    unsafe{
        appdomain.load(include_bytes!(<dir>)).invoke_entry(Some(&["HijackablePaths"]));
    }
}
