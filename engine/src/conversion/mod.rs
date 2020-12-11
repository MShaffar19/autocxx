// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod api;
mod bridge_name_tracker;
mod codegen;
mod namespace_organizer;
mod overload_tracker;
mod parse;
mod rust_name_tracker;
mod type_converter;
mod unqualify;
mod utilities;

pub(crate) use api::ConvertError;
use syn::{Item, ItemMod};

use crate::{byvalue_scanner::identify_byvalue_safe_types, type_database::TypeDatabase};

use self::{codegen::CodegenResults, parse::BridgeConversion};

/// Converts the bindings generated by bindgen into a form suitable
/// for use with `cxx`.
/// In fact, most of the actual operation happens within an
/// individual `BridgeConversion`.
///
/// # Flexibility in handling bindgen output
///
/// autocxx is inevitably tied to the details of the bindgen output;
/// e.g. the creation of a 'root' mod when namespaces are enabled.
/// At the moment this crate takes the view that it's OK to panic
/// if the bindgen output is not as expected. It may be in future that
/// we need to be a bit more graceful, but for now, that's OK.
pub(crate) struct BridgeConverter<'a> {
    include_list: &'a [String],
    type_database: &'a TypeDatabase,
}

impl<'a> BridgeConverter<'a> {
    pub fn new(include_list: &'a [String], type_database: &'a TypeDatabase) -> Self {
        Self {
            include_list,
            type_database,
        }
    }

    /// Convert a TokenStream of bindgen-generated bindings to a form
    /// suitable for cxx.
    pub(crate) fn convert(
        &mut self,
        mut bindgen_mod: ItemMod,
        exclude_utilities: bool,
    ) -> Result<CodegenResults, ConvertError> {
        match &mut bindgen_mod.content {
            None => Err(ConvertError::NoContent),
            Some((_, items)) => {
                let items_to_process = items.drain(..).collect();
                let items_in_root = Self::find_items_in_root(items_to_process)?;
                let byvalue_checker =
                    identify_byvalue_safe_types(&items_in_root, &self.type_database)?;
                let conversion = BridgeConversion::new(
                    bindgen_mod,
                    byvalue_checker,
                    &self.include_list,
                    &self.type_database,
                );
                conversion.convert_items(items_in_root, exclude_utilities)
            }
        }
    }

    fn find_items_in_root(items: Vec<Item>) -> Result<Vec<Item>, ConvertError> {
        for item in items {
            match item {
                Item::Mod(root_mod) => {
                    // With namespaces enabled, bindgen always puts everything
                    // in a mod called 'root'. We don't want to pass that
                    // onto cxx, so jump right into it.
                    assert!(root_mod.ident == "root");
                    if let Some((_, items)) = root_mod.content {
                        return Ok(items);
                    }
                }
                _ => return Err(ConvertError::UnexpectedOuterItem),
            }
        }
        Ok(Vec::new())
    }
}
