use reqwest::Url;
use std::path::PathBuf;

use crate::UniffiCustomTypeConverter;
// Custom UniFFI types

// `Url` as a custom type, with `String` as the Builtin
uniffi::custom_type!(Url, String);

impl UniffiCustomTypeConverter for Url {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        val.parse::<Url>().map_err(|e| e.into())
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        obj.as_str().to_owned()
    }
}

// `PathBuf` as a custom type, with `String` as the Builtin
uniffi::custom_type!(PathBuf, String);

impl UniffiCustomTypeConverter for PathBuf {
    type Builtin = String;

    fn into_custom(val: Self::Builtin) -> uniffi::Result<Self> {
        Ok(PathBuf::from(val))
    }

    fn from_custom(obj: Self) -> Self::Builtin {
        format!("{:?}", obj.display())
    }
}
