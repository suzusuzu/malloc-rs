use libc::size_t;

extern crate libc;
extern crate jemalloc_sys;

#[no_mangle]
pub unsafe extern "C" fn malloc(size: size_t) -> *mut libc::c_void {
    /*
    let message = "call malloc\n";
    let buf = message.as_ptr() as *const libc::c_void;
    let buf_len = message.len();
    libc::write(1, buf, buf_len);
    */
    jemalloc_sys::malloc(size)
}

#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut libc::c_void, size: size_t) -> *mut libc::c_void {
    jemalloc_sys::realloc(p, size)
}

#[no_mangle]
pub unsafe extern "C" fn calloc(number: size_t, size: size_t) -> *mut libc::c_void {
    jemalloc_sys::calloc(number, size)
}


#[no_mangle]
pub unsafe extern "C" fn free(p: *mut libc::c_void) {
    jemalloc_sys::free(p)
}

