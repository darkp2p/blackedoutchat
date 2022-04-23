use std::{future::Future, pin::Pin};

use crate::error::Result;

pub trait Handler {
    fn listen(self) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}
