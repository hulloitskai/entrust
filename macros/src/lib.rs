use proc_macro::TokenStream;

use syn::parse_macro_input;
use syn::DeriveInput;

mod object;

#[proc_macro_derive(Object)]
pub fn object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let output = object::expand(input);
    output.into()
}
