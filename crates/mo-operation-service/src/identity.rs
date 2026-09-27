use mo_common::{InvalidId, RequestId};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt};
macro_rules! identity {
    ($($name:ident),+) => {$(
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(RequestId);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> {RequestId::new(value).map(Self)}
            pub fn as_str(&self) -> &str {self.0.as_str()}
        }
        impl fmt::Display for $name {fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{self.0.fmt(f)}}
    )+};
}
identity!(PrincipalId, ScopeId, JobId, UploadId);
macro_rules! counter {
    ($($name:ident),+) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from="String",into="String")]
        pub struct $name(i64);
        impl $name {
            pub fn new(value:i64)->Result<Self,&'static str>{if value<0{Err("negative host counter")}else{Ok(Self(value))}}
            pub const fn get(self)->i64{self.0}
            pub fn checked_add(self,value:i64)->Option<Self>{self.0.checked_add(value).filter(|v|*v>=0).map(Self)}
        }
        impl TryFrom<String> for $name {
            type Error=&'static str;
            fn try_from(value:String)->Result<Self,Self::Error>{let n=value.parse::<i64>().map_err(|_|"invalid host counter")?;if n.to_string()!=value{return Err("noncanonical host counter");}Self::new(n)}
        }
        impl From<$name> for String {fn from(value:$name)->String{value.0.to_string()}}
        impl JsonSchema for $name {
            fn schema_name()->Cow<'static,str>{stringify!($name).into()}
            fn json_schema(_: &mut SchemaGenerator)->Schema{json_schema!({"type":"string","pattern":"^(0|[1-9][0-9]{0,18})$","not":{"pattern":"[^0-9]"},"x-integer-maximum":"9223372036854775807"})}
        }
    )+};
}
counter!(UnixMillis, JobFence);
