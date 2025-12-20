use quote::quote;
use syn::parse::Parse;
use syn::parse_macro_input;
use crate::type_sig::TypeSignature;

mod type_sig;

#[proc_macro]
pub fn type_signature(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let type_sig = parse_macro_input!(tokens as TypeSignature);

    quote! { #type_sig }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
