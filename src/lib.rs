extern crate libc;

use libc::{size_t, c_void};

/// malloc Header
struct Header {
    /// size of buffer
    size: size_t,

    /// flag of mmap
    is_mmap: size_t,

    // next header
    next: *mut Header,
}

/// 8byte aligment
const ALIGN: usize = 8;

/// max byte in segregated free list
const MAX_BYTE: usize = 512; 

/// init size of one free list
const INIT_LIST_SIZE: usize = 512;

/// add size of free list
const ADD_LIST_SIZE: usize = 512;

/// number of free list
const NUM_LIST: usize = MAX_BYTE/ALIGN + 1;

/// init size of heap(sbrk)
const INIT_HEAP_SIZE: usize = NUM_LIST * (INIT_LIST_SIZE + std::mem::size_of::<Header>());

/// flag of call init_malloc
static mut IS_INIT_MALLOC: bool = false;

/// segregated free list
static mut FREE_LISTS: [*mut Header; (NUM_LIST)] = [std::ptr::null_mut(); (NUM_LIST)];


fn get_align(size: usize) -> usize {
    (size + ALIGN - 1) / ALIGN * ALIGN
}

unsafe fn get_header(p: *mut c_void) -> *mut Header{
    let header= p.sub(std::mem::size_of::<Header>()) as *mut Header;
    header
}

/// init malloc function
/// Setup the initial value of the segregated free list using sbrk from heap.
unsafe fn init_malloc() -> Result<(), *mut c_void> {
    IS_INIT_MALLOC = true;

    let current_p = libc::sbrk(0);
    let ret = libc::sbrk((INIT_HEAP_SIZE as isize).try_into().unwrap());

    if ret != current_p {
        // fail sbrk
        return Err(ret);
    }

    // init segregated free list
    let mut p = ret;
    for i in 1..NUM_LIST {
        FREE_LISTS[i] = p as *mut Header;

        let num_header = INIT_LIST_SIZE/(i*ALIGN);
        for j in 0..num_header {
            let mut header = p as *mut Header;
            let size = i * ALIGN;
            (*header).size = size;
            (*header).is_mmap = 0;
            (*header).next = std::ptr::null_mut();

            
            let next_p = p.add(size + std::mem::size_of::<Header>());
            if j != (num_header - 1) {
                (*header).next = next_p as *mut Header;
            }else{
                // last element
                (*header).next = std::ptr::null_mut();
            }

            p = next_p;
        }

    }
        Ok(())
}

/// add segregated free list
/// When there is no more memory in the segregated free list, use sbrk to add memory from the heap.
unsafe fn add_list(size: usize) -> Result<*mut Header, *mut c_void> {
    let current_p = libc::sbrk(0);
    let num_header = ADD_LIST_SIZE/size;
    let ret = libc::sbrk((num_header * (size + std::mem::size_of::<Header>())).try_into().unwrap());

    if ret != current_p {
        // fail sbrk
        return Err(ret);
    }

    let mut p = ret;
    for j in 0..num_header {
        let mut header = p as *mut Header;
        (*header).size = size;
        (*header).is_mmap = 0;
        (*header).next = std::ptr::null_mut();
        
        let next_p = p.add(size + std::mem::size_of::<Header>());
        if j != (num_header - 1) {
            (*header).next = next_p as *mut Header;
        }else{
            // last element
            (*header).next = std::ptr::null_mut();
        }

        p = next_p;
    }

    Ok(ret as *mut Header)
}


/// find header function
/// Get a header of a given size from the segregated free list.
unsafe fn find_chunk(size: usize) -> Result<*mut Header, *mut c_void > {

    // index of segregated free list
    let index = size/8;

    if FREE_LISTS[index] == std::ptr::null_mut() {
        let new_list_ret = add_list(size);

        match new_list_ret {
            Ok(new_list) => {
                FREE_LISTS[index] = new_list;
            },
            Err(err) => {
                return Err(err);
            }
        }
    }

    let header = FREE_LISTS[index];

    // unlink chunk
    FREE_LISTS[index] = (*header).next;

    Ok(header)
}

/// malloc function
#[no_mangle]
pub unsafe extern "C" fn malloc(size: size_t) -> *mut c_void {
    if size == 0 {
        return std::ptr::null_mut();
    }

    if ! IS_INIT_MALLOC {
        if init_malloc().is_err() {
            return std::ptr::null_mut();
        }
    }

    let size_align = get_align(size);

    if size_align <= MAX_BYTE {
        // get memory from segregated free list
        let header_ret = find_chunk(size_align);
        if header_ret.is_err() {
            return std::ptr::null_mut();
        }
        let header = header_ret.unwrap();
        return (header as *mut c_void).add(std::mem::size_of::<Header>());
    }

    let mmap_size = std::mem::size_of::<Header>() + size;

    let p = libc::mmap(
        ::std::ptr::null_mut(),
        mmap_size,
        libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
        libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
        -1,
        0,
    );

    if p == libc::MAP_FAILED {
        return std::ptr::null_mut();
    }

    let mut header = p as *mut Header;
    (*header).size = mmap_size;
    (*header).is_mmap = 1;

    p.add(std::mem::size_of::<Header>())
}


/// realloc function
#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut c_void, size: size_t) -> *mut c_void {

    let size_align = get_align(size);
    if p == std::ptr::null_mut() {
        return malloc(size_align);
    }

    let new_p = malloc(size_align);
    let header = get_header(p);

    let memcpy_size = if (*header).size < size_align {(*header).size} else {size_align};
    libc::memcpy(new_p, p, memcpy_size);

    free(p);
    return new_p;
}

/// calloc function
#[no_mangle]
pub unsafe extern "C" fn calloc(number: size_t, size: size_t) -> *mut c_void {
    let new_p = malloc(size * number);
    libc::memset(new_p,0, size * number);
    return new_p;
}


/// free function
#[no_mangle]
pub unsafe extern "C" fn free(p: *mut c_void) {
    if p == std::ptr::null_mut() {
        return;
    }

    let header= get_header(p);
    let size = (*header).size;
    if (*header).is_mmap == 1 {
        // free mmap
        let munmap_ret = libc::munmap(p.sub(std::mem::size_of::<Header>()), size);
        debug_assert!(munmap_ret == 0);
        if munmap_ret == 0 {
            // success munmap
        } else {
            // fail munmap
            let message = "fail munmap\n";
            let buf = message.as_ptr() as *const c_void;
            let buf_len = message.len();
            libc::write(1, buf, buf_len);
        }
    }else{
        // reuse in segregated free list
        let index = size / ALIGN;
        let first_header = FREE_LISTS[index];
        FREE_LISTS[index] = header;
        (*header).next = first_header;
    }
}

