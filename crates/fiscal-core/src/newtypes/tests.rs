use std::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};

use super::{AccessKey, Cents, Cfop, Gtin, IbgeCode, Ncm, Rate, Rate4, TaxId};

fn assert_scalar_json<T>(value: T, json: &str)
where
    T: Serialize + DeserializeOwned + Debug + PartialEq,
{
    assert_eq!(serde_json::to_string(&value).unwrap(), json);
    assert_eq!(serde_json::from_str::<T>(json).unwrap(), value);
    // O contrato aceita o valor escalar, sem envelope de objeto ou array.
    assert!(serde_json::from_str::<T>(&format!("[{json}]")).is_err());
    assert!(serde_json::from_str::<T>(&format!("{{\"value\":{json}}}")).is_err());
}

#[test]
fn string_newtypes_preserve_scalar_json_and_leading_zeroes() {
    let key = "35250112345678000190550010000000011000000010";
    assert_scalar_json(AccessKey(key.into()), &format!("\"{key}\""));
    assert_scalar_json(TaxId("01234567000189".into()), "\"01234567000189\"");
    assert_scalar_json(IbgeCode("0123456".into()), "\"0123456\"");
    assert_scalar_json(Gtin("07891234567895".into()), "\"07891234567895\"");
    assert_scalar_json(Gtin("SEM GTIN".into()), "\"SEM GTIN\"");
    assert_scalar_json(Ncm("01012100".into()), "\"01012100\"");
    assert_scalar_json(Cfop("5102".into()), "\"5102\"");
}

#[test]
fn monetary_newtypes_preserve_exact_integer_json() {
    for value in [i64::MIN, -1800, -1, 0, 1, 1050, 1800, i64::MAX] {
        let json = value.to_string();
        assert_scalar_json(Cents(value), &json);
        assert_scalar_json(Rate(value), &json);
        assert_scalar_json(Rate4(value), &json);
    }
}

#[cfg(feature = "ts")]
#[test]
fn newtypes_preserve_typescript_scalar_aliases() {
    let config = ts_rs::Config::default();
    macro_rules! assert_alias {
        ($type:ty, $scalar:literal) => {
            assert_eq!(
                <$type as ts_rs::TS>::decl(&config),
                concat!("type ", stringify!($type), " = ", $scalar, ";")
            );
        };
    }
    assert_alias!(AccessKey, "string");
    assert_alias!(TaxId, "string");
    assert_alias!(IbgeCode, "string");
    assert_alias!(Gtin, "string");
    assert_alias!(Ncm, "string");
    assert_alias!(Cfop, "string");
    assert_alias!(Cents, "number");
    assert_alias!(Rate, "number");
    assert_alias!(Rate4, "number");
}
