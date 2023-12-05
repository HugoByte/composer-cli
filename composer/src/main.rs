use allocative::Allocative;
use anyhow::Error;
use convert_case::{Case, Casing};
use serde_derive::{Deserialize, Serialize};
use starlark::environment::{GlobalsBuilder, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::{none::NoneType, ProvidesStaticType, StarlarkValue, Value};
use starlark::environment::LibraryExtension::*;
use starlark::{starlark_module, starlark_simple_value, values::starlark_value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::io::ErrorKind;
use std::process::Command;
use std::result::Result::Ok;
use std::{env, fs};
use std::{io, path::Path};

pub mod common;
pub mod tests;

use common::*;

fn main() {
    let mut composer = Composer::default();

    composer.add_config("./config/custom_types(re-structured).star");

    composer.run();
}
