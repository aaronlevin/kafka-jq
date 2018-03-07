extern crate base64;
extern crate bson;

use self::bson::ordered::OrderedDocument;
use self::bson::Bson;
use self::bson::Bson::Boolean;
use self::bson::Bson::Array as BsonArray;
use self::bson::Bson::String as BsonString;
use self::bson::Bson::Document;
use self::bson::Bson::FloatingPoint;
use self::bson::Bson::Null;
use self::bson::Bson::I32;
use self::bson::Bson::I64;
use self::bson::Bson::Binary;
use self::bson::spec::BinarySubtype;

use jq::ffi::*;
use jq::jv_string;
use jq::jv_get_kind;
use jq::jv_is_integer;
use jq::jv_number_value;
use jq::jv_string_value;
use jq::jv_array_length;
use jq::jv_array_get;
use jq::jv_object_get;
use jq::jv_object_length;
use jq::jv_object_iter_key;
use jq::jv_object_iter_value;

const BSON_BINARY_KEY: &'static str = "$binary";
const BSON_TYPE_KEY: &'static str = "$type";
const BSON_BINARY_SUBTYPE_GENERIC: &'static str = "\\x00";
const BSON_BINARY_SUBTYPE_FUNCTION: &'static str = "\\x01";
const BSON_BINARY_SUBTYPE_BINARY_OLD: &'static str = "\\x02";
const BSON_BINARY_SUBTYPE_UUID_OLD: &'static str = "\\x03";
const BSON_BINARY_SUBTYPE_UUID: &'static str = "\\x04";
const BSON_BINARY_SUBTYPE_MD5: &'static str = "\\x05";
const BSON_BINARY_SUBTYPE_USER_DEFINED: &'static str = "\\x80";

pub fn binary_subtype_to_hex(subtype: BinarySubtype) -> &'static str {
    match subtype {
        BinarySubtype::Generic => BSON_BINARY_SUBTYPE_GENERIC,
        BinarySubtype::Function => BSON_BINARY_SUBTYPE_FUNCTION,
        BinarySubtype::BinaryOld => BSON_BINARY_SUBTYPE_BINARY_OLD,
        BinarySubtype::UuidOld => BSON_BINARY_SUBTYPE_UUID_OLD,
        BinarySubtype::Uuid => BSON_BINARY_SUBTYPE_UUID,
        BinarySubtype::Md5 => BSON_BINARY_SUBTYPE_MD5,
        BinarySubtype::UserDefined(_) => BSON_BINARY_SUBTYPE_USER_DEFINED,
    }
}

pub fn hex_to_binary_subtype(hex: &str) -> Option<BinarySubtype> {
    match hex {
        BSON_BINARY_SUBTYPE_GENERIC => Some(BinarySubtype::Generic),
        BSON_BINARY_SUBTYPE_FUNCTION => Some(BinarySubtype::Function),
        BSON_BINARY_SUBTYPE_BINARY_OLD => Some(BinarySubtype::BinaryOld),
        BSON_BINARY_SUBTYPE_UUID_OLD => Some(BinarySubtype::UuidOld),
        BSON_BINARY_SUBTYPE_UUID => Some(BinarySubtype::Uuid),
        BSON_BINARY_SUBTYPE_MD5 => Some(BinarySubtype::Md5),
        BSON_BINARY_SUBTYPE_USER_DEFINED => Some(BinarySubtype::UserDefined(0)),
        _ => None,
    }
}

pub fn bson_to_jv(bson: &Bson) -> jv {
    match bson {
        &Document(ref nested_doc) => ordered_doc_to_jv(nested_doc),
        &FloatingPoint(float_64) => unsafe { jv_number(float_64) },
        &BsonString(ref bson_string) => jv_string(bson_string.clone()),
        &BsonArray(ref bson_array) => {
            let mut array = unsafe { jv_array_sized(bson_array.len() as i32) };
            for bson_elem in bson_array {
                array = unsafe { jv_array_append(array, bson_to_jv(&bson_elem)) };
            }
            array
        }
        &Boolean(bson_bool) => unsafe { jv_bool(bson_bool as i32) },
        &Null => unsafe { jv_null() },
        &I32(bson_i32) => unsafe { jv_number(bson_i32 as f64) },
        &I64(bson_i64) => unsafe { jv_number(bson_i64 as f64) },
        &Binary(subtype, ref bson_binary) => {
            let b64 = base64::encode(bson_binary);
            let jv = unsafe { jv_object() };

            let bson_type_key = jv_string(BSON_TYPE_KEY.to_string());
            let bson_type_value = jv_string(binary_subtype_to_hex(subtype).to_owned());
            let bson_binary_key = jv_string(BSON_BINARY_KEY.to_string());
            let bson_binary_value = jv_string(b64);

            unsafe {
                jv_object_set(jv, bson_type_key, bson_type_value);
                jv_object_set(jv, bson_binary_key, bson_binary_value);
            }
            jv
        }
        _ => unsafe { jv_null() },
    }
}

pub fn jv_to_bson(json: jv) -> Option<Bson> {
    match jv_get_kind(json) {
        jv_kind::JV_KIND_INVALID => None,
        jv_kind::JV_KIND_NULL => Some(Bson::Null),
        jv_kind::JV_KIND_FALSE => Some(Bson::Boolean(false)),
        jv_kind::JV_KIND_TRUE => Some(Bson::Boolean(true)),
        jv_kind::JV_KIND_NUMBER => {
            let json_number = jv_number_value(json);
            if jv_is_integer(json) {
                // this is lossy; round-trippiong bson->json->bson
                // with small, i64 would by lossy here
                if json_number < 2_147_483_648f64 && json_number > -2_147_483_648f64 {
                    Some(Bson::I32(json_number as i32))
                } else {
                    Some(Bson::I64(json_number as i64))
                }
            } else {
                // jv uses (int) to check for is integer and so this fails
                // on numbers 64-bit integers.
                if json_number.ceil() == json_number {
                    Some(Bson::I64(json_number.trunc() as i64))
                } else {
                    Some(Bson::FloatingPoint(json_number))
                }
            }
        }
        jv_kind::JV_KIND_STRING => {
            let string = jv_string_value(&json);
            Some(Bson::String(string.to_owned()))
        }
        jv_kind::JV_KIND_ARRAY => {
            let length = jv_array_length(json);
            let mut vector = Vec::with_capacity(length);
            for i in 0..length {
                vector.push(jv_to_bson(jv_array_get(json, i)).unwrap());
            }
            Some(Bson::Array(vector))
        }
        jv_kind::JV_KIND_OBJECT => {
            let mut doc = OrderedDocument::new();
            let type_value = jv_object_get(json, jv_string(BSON_BINARY_KEY.to_owned()));
            match type_value {
                None => {
                    let length = jv_object_length(json);
                    for i in 0..length {
                        let key = jv_object_iter_key(json, i);
                        let value = jv_object_iter_value(json, i);
                        let key_string = jv_string_value(&key);
                        doc.insert_bson(key_string.to_owned(), jv_to_bson(value).unwrap());
                    }
                }
                Some(jv_value) => {
                    let subtype = jv_object_get(json, jv_string(BSON_TYPE_KEY.to_owned())).map_or(
                        BinarySubtype::Generic,
                        |s| {
                            hex_to_binary_subtype(jv_string_value(&s))
                                .unwrap_or(BinarySubtype::Generic)
                        },
                    );
                    return Some(Bson::Binary(
                        subtype,
                        base64::decode(jv_string_value(&jv_value)).unwrap(),
                    ));
                }
            }
            Some(Bson::Document(doc))
        }
    }
}

pub fn ordered_doc_to_jv(doc: &OrderedDocument) -> jv {
    let mut jv = unsafe { jv_object() };
    for (key, value) in doc.iter() {
        let jv_key_string = jv_string(key.to_owned());
        let jv_value = bson_to_jv(value);
        unsafe {
            jv = jv_object_set(jv, jv_key_string, jv_value);
        }
    }
    jv
}

#[cfg(test)]
mod tests {
    extern crate bson as extbson;

    use bson::bson_to_jv;
    use bson::jv_to_bson;
    use jq::ffi::jv_false;
    use jq::ffi::jv_is_integer;
    use jq::ffi::jv_null;
    use jq::ffi::jv_number;
    use jq::ffi::jv_true;
    use jq::jv_string;
    use proptest::prelude::*;
    use self::extbson::Bson;
    use self::extbson::ordered::OrderedDocument;
    use self::extbson::spec::BinarySubtype;

    fn arb_bson() -> BoxedStrategy<Bson> {
        let leaf = prop_oneof![
            Just(Bson::Null),
            any::<bool>().prop_map(Bson::Boolean),
            // need to improve the number testing.
            any::<i32>().prop_map(Bson::I32),
            (2_147_483_649..2_147_483_650i64).prop_map(Bson::I64),
            (0..199_254_740_992i64).prop_map(|f| Bson::FloatingPoint((f as f64) + 0.1)),
            any::<String>().prop_map(Bson::String),
            (
                prop::collection::vec(any::<u8>(), 0..10),
                prop_oneof![
                    Just(BinarySubtype::Generic),
                    Just(BinarySubtype::Function),
                    Just(BinarySubtype::Uuid),
                    Just(BinarySubtype::UuidOld),
                    Just(BinarySubtype::BinaryOld)
                ]
            ).prop_map(|(bytes, subtype)| Bson::Binary(subtype, bytes))
        ];
        leaf.prop_recursive(
            8,   // 8 levels deep
            256, // shoot for maximum size 256 nodes
            10,  // we put 10 items per collection
            |inner| {
                prop_oneof![
                    prop::collection::vec(inner.clone(), 0..10).prop_map(Bson::Array),
                    prop::collection::vec(inner.clone(), 0..10).prop_flat_map(|vec| {
                        let key = any::<String>();
                        key.prop_map(move |k| {
                            let mut doc = OrderedDocument::new();
                            for (i, bson) in vec.clone().iter().enumerate() {
                                let key_string: String = format!("{}-{}", k, i);
                                doc.insert_bson(key_string, bson.clone());
                            }
                            Bson::Document(doc)
                        })
                    })
                ]
            },
        ).boxed()
    }

    proptest! {
        #[test]
        fn it_round_trips_bson_and_jv(ref string in "\\PC*") {
            let bson_string = Bson::String(string.to_owned());
            let jv_string = jv_string(string.to_owned());

            assert_eq!(bson_to_jv(&bson_string), jv_string);
            assert_eq!(jv_to_bson(jv_string).unwrap(), bson_string);
            assert_eq!(jv_to_bson(bson_to_jv(&bson_string)).unwrap(), bson_string);
            assert_eq!(bson_to_jv(&jv_to_bson(jv_string).unwrap()), jv_string);
        }
    }

    proptest! {
        #[test]
        fn it_converts_bson_float_to_jv_double(float64 in any::<f64>()) {
            let jv = bson_to_jv(&Bson::FloatingPoint(float64));
            let jv_float = unsafe { jv_number(float64) };

            assert_eq!(jv_float, jv);
            // test twice to ensure there's no issues with use after free
            assert_eq!(jv, jv_float);
        }
    }

    proptest! {
        #[test]
        fn it_converts_bson_i32_to_jv_integer(int in any::<i32>()) {
            let jv = bson_to_jv(&Bson::I32(int));
            let jv_i32 = unsafe { jv_number(int as f64) };

            assert_eq!(jv_i32, jv);
            assert_eq!(jv, jv_i32);
            assert_eq!(unsafe { jv_is_integer(jv) }, 1);
            assert_eq!(unsafe { jv_is_integer(jv_i32) }, 1);
        }
    }

    proptest! {
        #[test]
        fn it_converts_bson_i64_to_jv_integer(int in any::<i64>()) {
            let jv = bson_to_jv(&Bson::I64(int));
            let jv_i64 = unsafe { jv_number(int as f64) };

            assert_eq!(jv_i64, jv);
            assert_eq!(jv, jv_i64);
            assert_eq!(unsafe { jv_is_integer(jv) }, 0);
            assert_eq!(unsafe { jv_is_integer(jv_i64) }, 0);
        }
    }

    #[test]
    fn it_converts_bson_bool_to_jv_bool() {
        let jv_from_bson_true = bson_to_jv(&Bson::Boolean(true));
        let jv_from_bson_false = bson_to_jv(&Bson::Boolean(false));
        let jv_true = unsafe { jv_true() };
        let jv_false = unsafe { jv_false() };

        assert_eq!(jv_from_bson_true, jv_true);
        assert_eq!(jv_from_bson_false, jv_false);
    }

    #[test]
    fn it_converts_bson_null_to_jv_null() {
        let jv_from_bson_null = bson_to_jv(&Bson::Null);
        let jv_null = unsafe { jv_null() };

        assert_eq!(jv_from_bson_null, jv_null);
    }

    proptest! {
        #[test]
        fn it_converts_arb_bson_to_jv_with_equality(ref bson in arb_bson()) {
            let jv1 = bson_to_jv(bson);
            let jv2 = bson_to_jv(bson);
            let bson_roundtrip = jv_to_bson(bson_to_jv(bson)).unwrap();

            assert_eq!(jv1, jv2);
            assert_eq!(jv2, jv1);
            assert_eq!(bson_roundtrip, *bson);
        }
    }
}
