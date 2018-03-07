use std::ffi::CString;
use std::ffi::CStr;

pub mod ffi;

type jv = ffi::jv;
type jv_kind = ffi::jv_kind;

impl PartialEq for jv {
    fn eq(&self, other: &jv) -> bool {
        unsafe { ffi::jv_equal(ffi::jv_copy(*self), ffi::jv_copy(*other)) == 1 }
    }
}

impl Eq for jv {}

pub fn jv_string(string: String) -> jv {
    let length = string.len() as i32;
    let c_string = CString::new(string).unwrap();
    let ptr = c_string.into_raw();
    let jv_string = unsafe { ffi::jv_string_sized(ptr, length) };
    let _ = unsafe { CString::from_raw(ptr) };
    jv_string
}

pub fn jv_string_value<'a>(arg: &'a jv) -> &'a str {
    unsafe {
        let c_string = ffi::jv_string_value(*arg);
        CStr::from_ptr::<'a>(c_string).to_str().unwrap()
    }
}

pub fn jv_array_length(arg: jv) -> usize {
    // technically w should call jv_array_length
    // however, that unnecessarily copies and frees.
    //unsafe { ffi::jv_array_length(arg) as usize }
    arg.size as usize
}

pub fn jv_array_get(arg: jv, index: usize) -> jv {
    unsafe { ffi::jv_array_get(ffi::jv_copy(arg), index as i32) }
}

pub fn jv_get_kind(arg: jv) -> jv_kind {
    unsafe { ffi::jv_get_kind(arg) }
}

pub fn jv_is_integer(arg: jv) -> bool {
    unsafe { ffi::jv_is_integer(arg) == 1 }
}

pub fn jv_object_length(arg: jv) -> usize {
    unsafe { ffi::jv_object_length(ffi::jv_copy(arg)) as usize }
}

pub fn jv_object_get(object: jv, key: jv) -> Option<jv> {
    let value = unsafe { ffi::jv_object_get(ffi::jv_copy(object), ffi::jv_copy(key)) };
    if jv_get_kind(value) == ffi::jv_kind::JV_KIND_INVALID {
        None
    } else {
        Some(value)
    }
}

pub fn jv_object_iter_key(arg: jv, index: usize) -> jv {
    unsafe { ffi::jv_object_iter_key(ffi::jv_copy(arg), index as i32) }
}

pub fn jv_object_iter_value(arg: jv, index: usize) -> jv {
    unsafe { ffi::jv_object_iter_value(ffi::jv_copy(arg), index as i32) }
}

pub fn jv_number_value(arg: jv) -> f64 {
    unsafe { ffi::jv_number_value(arg) }
}

#[cfg(test)]
mod tests {

    use jq::jv_string;

    proptest! {
        #[test]
        fn it_makes_string(ref s1 in "\\PC*") {
            let jv1 = jv_string(s1.to_string());
            let jv2 = jv_string(s1.to_string());

            assert_eq!(jv1, jv2);
            assert_eq!(jv2, jv1);
        }
    }
}
