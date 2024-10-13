// Higher recursion limit for quote
#![recursion_limit = "512"]

extern crate proc_macro;

use crate::bitfield::bf::BitField;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod bitfield;

/// Generate the declaratively described state machine diagram.
///
/// See the main crate documentation for more details.
#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    let input2 = input.clone();
    let bf: BitField = parse_macro_input!(input2 as BitField);
    
    let expanded = quote!(#bf);
//    println!("{}", expanded);

    expanded.into()
}
